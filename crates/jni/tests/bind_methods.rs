#![cfg(feature = "invocation")]
mod util;

use jni::Env;
use jni::bind_java_type;
use jni::objects::JString;
use rusty_fork::rusty_fork_test;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

// Create bindings for TestMethods class
bind_java_type! {
    rust_type = TestMethods,
    java_type = "com.example.TestMethods",
    constructors {
        fn new(),
        fn new_with_message(message: JString),
        fn new_with_message_and_counter(message: JString, counter: jint),
    },
    methods {
        static fn get_static_message() -> JString,
        static fn add(a: jint, b: jint) -> jint,
        static fn multiply(a: jlong, b: jlong) -> jlong,
        static fn concat(a: JString, b: JString) -> JString,
        fn get_message() -> JString,
        fn get_counter() -> jint,
        fn set_message(message: JString) -> void,
        fn set_counter(counter: jint) -> void,
        fn increment() -> void,
        fn increment_by(amount: jint) -> void,
        fn format_message(prefix: JString, suffix: JString) -> JString,
        fn is_positive() -> jboolean,
        fn calculate(a: jint, b: jlong, c: jfloat, d: jdouble) -> jdouble,
        fn to_string_custom() -> JString,
        fn reset() -> void,

        // Methods for testing non_null validation
        fn get_nullable_message() -> JString,  // Can return null
        non_null fn get_required_message() -> JString,  // Shorthand syntax - must not return null
        fn get_validated_message {  // Block syntax with explicit non_null
            sig = () -> JString,
            non_null = true,
        },

        // Methods for testing cfg attribute support
        // These are guarded by _cfg_test which is never enabled in tests
        #[cfg(feature = "_cfg_test")]
        static fn cfg_test_method() -> jint,
        #[cfg(feature = "_cfg_test")]
        fn instance_cfg_test_method() -> jint,

        // These are guarded by invocation which is always enabled in tests
        #[cfg(feature = "invocation")]
        static fn invocation_method() -> jint,
        #[cfg(feature = "invocation")]
        fn instance_invocation_method() -> jint,
    }
}

rusty_fork_test! {
#[test]
fn test_static_methods() {
    let out_dir = setup_test_output("bind_methods_static");

    // Compile Java class
    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        // Test get_static_message()
        let message = TestMethods::get_static_message(env)?;
        let message_str = message.to_string();
        assert_eq!(message_str, "static message");

        // Test add()
        let result = TestMethods::add(env, 10, 20)?;
        assert_eq!(result, 30);

        // Test multiply()
        let result = TestMethods::multiply(env, 1000, 2000)?;
        assert_eq!(result, 2_000_000);

        // Test concat()
        let a = JString::from_str(env, "Hello, ")?;
        let b = JString::from_str(env, "World!")?;
        let result = TestMethods::concat(env, &a, &b)?;
        let result_str = result.to_string();
        assert_eq!(result_str, "Hello, World!");

        Ok(())
    })
    .expect("Static methods test failed");
}
}

rusty_fork_test! {
#[test]
fn test_constructors() {
    let out_dir = setup_test_output("bind_methods_constructors");

    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        // Test default constructor
        let obj1 = TestMethods::new(env)?;
        let message = obj1.get_message(env)?;
        assert_eq!(message.to_string(), "default");
        let counter = obj1.get_counter(env)?;
        assert_eq!(counter, 0);

        // Test constructor with message
        let msg = JString::from_str(env, "custom message")?;
        let obj2 = TestMethods::new_with_message(env, &msg)?;
        let message = obj2.get_message(env)?;
        assert_eq!(message.to_string(), "custom message");
        let counter = obj2.get_counter(env)?;
        assert_eq!(counter, 0);

        // Test constructor with message and counter
        let msg = JString::from_str(env, "test")?;
        let obj3 = TestMethods::new_with_message_and_counter(env, &msg, 42)?;
        let message = obj3.get_message(env)?;
        assert_eq!(message.to_string(), "test");
        let counter = obj3.get_counter(env)?;
        assert_eq!(counter, 42);

        Ok(())
    })
    .expect("Constructors test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_methods_getters_setters() {
    let out_dir = setup_test_output("bind_methods_getters_setters");

    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        // First attempt to make a call with a null object reference
        let res = TestMethods::null().get_message(env);
        assert!(matches!(res, Err(jni::errors::Error::NullPtr(_))));

        let obj = TestMethods::new(env)?;

        // Test initial values
        let message = obj.get_message(env)?;
        assert_eq!(message.to_string(), "default");
        let counter = obj.get_counter(env)?;
        assert_eq!(counter, 0);

        // Test setters
        let new_msg = JString::from_str(env, "updated message")?;
        obj.set_message(env, &new_msg)?;
        let message = obj.get_message(env)?;
        assert_eq!(message.to_string(), "updated message");

        obj.set_counter(env, 100)?;
        let counter = obj.get_counter(env)?;
        assert_eq!(counter, 100);

        Ok(())
    })
    .expect("Getters/setters test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_methods_operations() {
    let out_dir = setup_test_output("bind_methods_operations");

    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        let obj = TestMethods::new(env)?;

        // Test increment()
        obj.increment(env)?;
        let counter = obj.get_counter(env)?;
        assert_eq!(counter, 1);

        // Test increment_by()
        obj.increment_by(env, 5)?;
        let counter = obj.get_counter(env)?;
        assert_eq!(counter, 6);

        // Test is_positive()
        let is_positive = obj.is_positive(env)?;
        assert!(is_positive);

        // Set counter to negative and test again
        obj.set_counter(env, -5)?;
        let is_positive = obj.is_positive(env)?;
        assert!(!is_positive);

        Ok(())
    })
    .expect("Operations test failed");
}
}

rusty_fork_test! {
#[test]
fn test_string_methods() {
    let out_dir = setup_test_output("bind_methods_strings");

    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        let msg = JString::from_str(env, "test message")?;
        let obj = TestMethods::new_with_message(env, &msg)?;

        // Test format_message()
        let prefix = JString::from_str(env, "[")?;
        let suffix = JString::from_str(env, "]")?;
        let formatted = obj.format_message(env, &prefix, &suffix)?;
        assert_eq!(formatted.to_string(), "[test message]");

        // Test to_string_custom()
        let obj_str = obj.to_string_custom(env)?;
        let obj_str_value = obj_str.to_string();
        assert!(obj_str_value.contains("test message"));
        assert!(obj_str_value.contains("counter=0"));

        Ok(())
    })
    .expect("String methods test failed");
}
}

rusty_fork_test! {
#[test]
fn test_calculate_multiple_types() {
    let out_dir = setup_test_output("bind_methods_calculate");

    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        let obj = TestMethods::new(env)?;

        // Test calculate() with different primitive types
        let result = obj.calculate(env, 10, 20, 1.5, 2.5)?;
        // Expected: 10 + 20 + 1.5 + 2.5 = 34.0
        assert!((result - 34.0).abs() < 0.001);

        Ok(())
    })
    .expect("Calculate test failed");
}
}

rusty_fork_test! {
#[test]
fn test_reset_method() {
    let out_dir = setup_test_output("bind_methods_reset");

    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        let msg = JString::from_str(env, "custom")?;
        let obj = TestMethods::new_with_message_and_counter(env, &msg, 99)?;

        // Verify initial state
        let message = obj.get_message(env)?;
        assert_eq!(message.to_string(), "custom");
        let counter = obj.get_counter(env)?;
        assert_eq!(counter, 99);

        // Reset
        obj.reset(env)?;

        // Verify reset state
        let message = obj.get_message(env)?;
        assert_eq!(message.to_string(), "default");
        let counter = obj.get_counter(env)?;
        assert_eq!(counter, 0);

        Ok(())
    })
    .expect("Reset test failed");
}
}

rusty_fork_test! {
#[test]
fn test_non_null_validation() {
    let out_dir = setup_test_output("bind_methods_non_null");

    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        let obj = TestMethods::new(env)?;

        // Test nullable method - should allow null without error
        let nullable = obj.get_nullable_message(env)?;
        assert!(nullable.is_null(), "Expected get_nullable_message to return null");

        // Test non_null method with shorthand syntax
        let result = obj.get_required_message(env);
        assert!(
            matches!(result, Err(jni::errors::Error::NullPtr(_))),
            "Expected Error::NullPtr when non_null method returns null"
        );

        // Test non_null method with block syntax
        let result = obj.get_validated_message(env);
        assert!(
            matches!(result, Err(jni::errors::Error::NullPtr(_))),
            "Expected Error::NullPtr when non_null method returns null"
        );

        Ok(())
    })
    .expect("Non-null validation test failed");
}
}

rusty_fork_test! {
#[test]
fn test_cfg_guarded_methods() {
    let out_dir = setup_test_output("bind_methods_cfg");

    // Compile Java class
    javac::Build::new()
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_methods_class(env, &out_dir)?;

        // The TestMethods binding includes methods guarded by both:
        // - invocation feature (always enabled in tests) - we test these
        // - _cfg_test feature (never enabled in tests) - these are not available

        // The following would fail to compile if uncommented:
        // TestMethods::cfg_test_method(env)?;
        // obj.instance_cfg_test_method(env)?;
        // Note: we have separate ui / trybuild tests to check that these are properly excluded from the bindings

        let result = TestMethods::invocation_method(env)?;
        assert_eq!(result, 99);

        let message = JString::from_str(env, "test")?;
        let obj = TestMethods::new_with_message_and_counter(
            env,
            &message,
            15,
        )?;
        let result = obj.instance_invocation_method(env)?;
        assert_eq!(result, 215); // counter (15) + 200

        #[cfg(feature = "_cfg_test")]
        {
            let result = TestMethods::cfg_test_method(env)?;
            assert_eq!(result, 42);

            let result = obj.instance_cfg_test_method(env)?;
            assert_eq!(result, 115); // counter (15) + 100
        }

        Ok(())
    })
    .expect("Cfg-guarded methods test failed");
}
}

// Helper function to load the TestMethods class
fn load_test_class(env: &mut Env, out_dir: &Path, name: &str) -> jni::errors::Result<()> {
    let class_path = out_dir.join(format!("com/example/{}.class", name));
    assert!(class_path.exists(), "{}.class not found", name);

    let class_bytes =
        fs::read(&class_path).unwrap_or_else(|_| panic!("Failed to read {}.class", name));

    let class_loader = jni::objects::JClassLoader::get_system_class_loader(env)
        .expect("Failed to get system class loader");

    env.define_class(
        Option::<&jni::strings::JNIStr>::None, // Urgh, passing None was supposed to be the easy choice :)
        &class_loader,
        &class_bytes,
    )
    .expect("Failed to define TestMethods class");

    Ok(())
}

fn load_test_methods_class(env: &mut Env, out_dir: &Path) -> jni::errors::Result<()> {
    load_test_class(env, out_dir, "TestMethods")
}

// Helper function to set up test output directory
fn setup_test_output(test_name: &str) -> PathBuf {
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("jni_macros_tests")
        .join(test_name);

    // Clean up any existing output
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("Failed to create test output directory");

    out_dir
}

bind_java_type! {
    TestVersion => "com.example.TestVersion",
    fields {
        #[allow(non_snake_case)]
        static VERSION: jint,
    }
}

// Cache for the Java version, queried once from TestVersion.VERSION
static TEST_VERSION: OnceLock<u32> = OnceLock::new();

// Demonstrate checking some `#[jni(requires = )]` conditions at runtime by
// querying a Java version field and caching it
fn jni_check_version(required_version: u32) -> bool {
    let version = TEST_VERSION.get_or_init(|| {
        let jvm = jni::JavaVM::singleton().expect("JavaVM singleton not initialized");
        jvm.attach_current_thread(|env| TestVersion::VERSION(env))
            .expect("Failed to get Java version") as u32
    });

    println!(
        "Checking Java version ({}) >= {}",
        *version, required_version
    );

    *version >= required_version
}

fn check_version_21() -> bool {
    jni_check_version(21)
}

fn check_version(version: u32) -> bool {
    jni_check_version(version)
}

fn check_feature_enabled() -> bool {
    true // Simulate feature is enabled
}

// Bindings for testing requires attribute with methods
bind_java_type! {
    rust_type = TestMethodsWithRequires,
    java_type = "com.example.TestMethods",
    constructors {
        fn new(),
        // This constructor should fail at runtime since check_version_21() returns false
        #[jni(requires = check_version_21())]
        fn new_v21_only(message: JString),
    },
    methods {
        fn get_message() -> JString,
        // This method should fail at runtime
        #[jni(requires = check_version_21())]
        fn get_counter() -> jint,
        // This method should work
        #[jni(requires = check_version(19))]
        fn set_message(message: JString) -> void,
        // Test with literal expression
        #[jni(requires = "true")]
        fn reset() -> void,
        // Test with literal expression false
        #[jni(requires = "false")]
        fn increment() -> void,
        // Test with multiple requires - all must be true (should work)
        #[jni(requires = check_version(19))]
        #[jni(requires = check_feature_enabled())]
        fn combined_check_pass() -> void,
        // Test with multiple requires - one is false (should fail)
        #[jni(requires = check_version(19))]
        #[jni(requires = check_version_21())]
        fn combined_check_fail() -> void,
    }
}

rusty_fork_test! {
#[test]
fn test_requires_attribute_methods() {
    let out_dir = setup_test_output("bind_methods_requires");

    javac::Build::new()
        .file("tests/java/com/example/TestVersion.java")
        .file("tests/java/com/example/TestMethods.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_class(env, &out_dir, "TestVersion")?;
        load_test_methods_class(env, &out_dir)?;

        // Test that new() works (no requires)
        let obj = TestMethodsWithRequires::new(env)?;

        // Test that constructor with requires = false fails
        let msg = JString::from_str(env, "test")?;
        let result = TestMethodsWithRequires::new_v21_only(env, &msg);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, jni::errors::Error::UnsupportedVersion));
        }

        // Test that get_message() works (no requires)
        let message = obj.get_message(env)?;
        assert_eq!(message.to_string(), "default");

        // Test that get_counter() fails (requires = false)
        let result = obj.get_counter(env);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, jni::errors::Error::UnsupportedVersion));
        }

        // Test that set_message() works (requires = true)
        let new_msg = JString::from_str(env, "updated")?;
        obj.set_message(env, &new_msg)?;
        let message = obj.get_message(env)?;
        assert_eq!(message.to_string(), "updated");

        // Test that reset() works (requires = "true")
        obj.reset(env)?;
        let message = obj.get_message(env)?;
        assert_eq!(message.to_string(), "default");

        // Test that increment() fails (requires = "false")
        let result = obj.increment(env);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, jni::errors::Error::UnsupportedVersion));
        }

        // Test combined requires - all conditions true (should work)
        obj.combined_check_pass(env)?;
        let message = obj.get_message(env)?;
        assert_eq!(message.to_string(), "combined pass");

        // Test combined requires - one condition false (should fail)
        let result = obj.combined_check_fail(env);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, jni::errors::Error::UnsupportedVersion));
        }

        Ok(())
    })
    .expect("Requires attribute methods test failed");
}
}
