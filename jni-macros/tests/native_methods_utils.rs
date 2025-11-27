//! Utilities for testing the `native_method!` macro.
//!
//! This module provides helper functions and macros to simplify writing tests for
//! the `native_method!` macro, particularly for runtime tests that need to compile
//! Java classes, register native methods, and call them through JNI.

/// Compile a test Java class and return the output directory.
///
/// This helper compiles the specified Java file and returns the path to the
/// output directory containing the compiled .class files.
#[allow(dead_code)]
pub fn compile_test_class(test_name: &str, java_file: &str) -> std::path::PathBuf {
    let out_dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("jni_macros_tests")
        .join(test_name);

    // Clean up any existing output
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir).expect("Failed to create test output directory");

    javac::Build::new()
        .file(java_file)
        .output_dir(&out_dir)
        .compile();

    out_dir
}

/// Load a test class from the output directory into the JVM.
///
/// This helper loads a compiled Java class file into the JVM using a system class loader.
/// The class should be in the package `com.example`.
#[allow(dead_code)]
pub fn load_test_class<'local>(
    env: &mut jni::Env<'local>,
    out_dir: &std::path::Path,
    class_name: &str,
) -> jni::errors::Result<jni::objects::JClass<'local>> {
    let class_path = out_dir.join(format!("com/example/{}.class", class_name));
    assert!(
        class_path.exists(),
        "{}.class not found at {:?}",
        class_name,
        class_path
    );

    let class_bytes = std::fs::read(&class_path)
        .unwrap_or_else(|_| panic!("Failed to read {}.class", class_name));

    let class_loader = jni::objects::JClassLoader::get_system_class_loader(env)
        .expect("Failed to get system class loader");

    let class_internal_name = format!("com/example/{}", class_name);
    let class_jni = jni::strings::JNIString::new(class_internal_name.as_str());

    env.define_class(Some(&class_jni), &class_loader, &class_bytes)
}

/// Macro to simplify native method tests that manually register methods.
///
/// This macro reduces boilerplate for tests that need to:
/// 1. Compile a Java class
/// 2. Set up the JVM
/// 3. Load the class
/// 4. Register native methods
/// 5. Execute test code
/// 6. Optionally handle expected exceptions
///
/// # Examples
///
/// Basic test that expects success:
/// ```ignore
/// native_method_test! {
///     test_name: test_my_native_method,
///     java_class: "com/example/TestClass.java",
///     methods: |class| &[
///         native_method! {
///             name = "myMethod",
///             sig = (value: jint) -> jint,
///             fn = my_method_impl,
///         },
///     ],
///     test_body: |env, class| {
///         let result = env.call_static_method(class, "myMethod", "(I)I", &[jni::objects::JValue::Int(42)])?;
///         assert_eq!(result.i()?, 84);
///         Ok(())
///     }
/// }
/// ```
///
/// Test that expects an exception:
/// ```ignore
/// native_method_test! {
///     test_name: test_expected_exception,
///     java_class: "com/example/TestClass.java",
///     methods: |class| &[
///         native_method! {
///             name = "badMethod",
///             sig = () -> void,
///             fn = bad_method_impl,
///         },
///     ],
///     expect_exception: "Method should throw",
///     test_body: |env, class| {
///         env.call_static_method(class, "badMethod", "()V", &[])?;
///         Ok(())
///     }
/// }
/// ```
#[macro_export]
macro_rules! native_method_test {
    // Variant that expects the test to succeed
    (
        $(#[$attribute:meta])*
        test_name: $test_name:ident,
        java_class: $java_class:literal,
        methods: |$class_param:ident| $methods:expr,
        test_body: |$env:ident, $class:ident| $body:block
    ) => {
        rusty_fork::rusty_fork_test! {
            #[test]
            $(#[$attribute])*
            fn $test_name() {
                #[allow(clippy::crate_in_macro_def)]
                let out_dir = $crate::native_methods_utils::compile_test_class(
                    stringify!($test_name),
                    concat!("tests/java/", $java_class)
                );

                #[allow(clippy::crate_in_macro_def)]
                $crate::util::attach_current_thread(|$env| {
                    let class_name = $java_class
                        .trim_end_matches(".java")
                        .split('/')
                        .last()
                        .expect("Invalid Java class path");

                    #[allow(clippy::crate_in_macro_def)]
                    let $class = $crate::native_methods_utils::load_test_class($env, &out_dir, class_name)?;

                    // Register native methods
                    let methods: &[jni::NativeMethod] = $methods;
                    unsafe {
                        $env.register_native_methods(&$class, methods)?;
                    }

                    // Run test body
                    $body
                })
                .expect(concat!(stringify!($test_name), " failed"));
            }
        }
    };

    // Variant that expects the test to fail with a JavaException
    (
        $(#[$attribute:meta])*
        test_name: $test_name:ident,
        java_class: $java_class:literal,
        methods: |$class_param:ident| $methods:expr,
        expect_exception: $expected_msg:expr,
        test_body: |$env:ident, $class:ident| $body:block
    ) => {
        rusty_fork::rusty_fork_test! {
            #[test]
            $(#[$attribute])*
            fn $test_name() {
                #[allow(clippy::crate_in_macro_def)]
                let out_dir = $crate::native_methods_utils::compile_test_class(
                    stringify!($test_name),
                    concat!("tests/java/", $java_class)
                );

                #[allow(clippy::crate_in_macro_def)]
                let result = $crate::util::attach_current_thread(|$env| {
                    let class_name = $java_class
                        .trim_end_matches(".java")
                        .split('/')
                        .last()
                        .expect("Invalid Java class path");

                    #[allow(clippy::crate_in_macro_def)]
                    let $class = $crate::native_methods_utils::load_test_class($env, &out_dir, class_name)?;

                    // Register native methods
                    let methods: &[jni::NativeMethod] = $methods;
                    unsafe {
                        $env.register_native_methods(&$class, methods)?;
                    }

                    // Run test body
                    $body
                });

                match result {
                    Err(jni::errors::Error::JavaException) => {
                        println!("✓ {}", $expected_msg);
                    }
                    _ => panic!("Expected JavaException: {}, got: {:?}", $expected_msg, result),
                }
            }
        }
    };
}

/// Macro to call an instance method with integer argument.
///
/// Simplified wrapper around `call_method` for the common case of calling
/// a method that takes a single `jint` and returns a `jint`.
///
/// # Example
/// ```ignore
/// let result = call_int_method!(env, &obj, "callMethod", 42)?;
/// ```
#[macro_export]
macro_rules! call_int_method {
    ($env:expr, $obj:expr, $method_name:literal, $arg:expr) => {{
        use jni::jni_str;
        let result = $env.call_method(
            $obj,
            jni_str!($method_name),
            jni_str!("(I)I"),
            //jni_sig!("(I)I"), TODO
            &[jni::objects::JValue::Int($arg)],
        )?;
        result.i()
    }};
}

/// Macro to call a static method with integer argument.
///
/// Simplified wrapper around `call_static_method` for the common case of calling
/// a static method that takes a single `jint` and returns a `jint`.
///
/// # Example
/// ```ignore
/// let result = call_static_int_method!(env, class, "callStaticMethod", 42)?;
/// ```
#[macro_export]
macro_rules! call_static_int_method {
    ($env:expr, $class:expr, $method_name:literal, $arg:expr) => {{
        use jni::jni_str;
        let result = $env.call_static_method(
            $class,
            jni_str!($method_name),
            jni_str!("(I)I"),
            //jni_sig!("(I)I"), TODO
            &[jni::objects::JValue::Int($arg)],
        )?;
        result.i()
    }};
}

/// Macro to create a new object with default constructor.
///
/// Simplified wrapper around `new_object` for the common case of creating
/// an object with a no-argument constructor.
///
/// # Example
/// ```ignore
/// let obj = new_object!(env, class)?;
/// ```
#[macro_export]
macro_rules! new_object {
    ($env:expr, $class:expr) => {{
        use jni::jni_str;
        $env.new_object(&$class, jni_str!("()V"), &[])
        //$env.new_object(&$class, jni_sig!("()V"), &[]) TODO
    }};
}
