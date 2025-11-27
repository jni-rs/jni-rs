/// Macro to simplify native method tests.
#[macro_export]
macro_rules! native_method_test {
    (
        $(#[$attribute:meta])*
        test_name: $test_name:ident,
        java_class: $java_class:literal,
        api: $api:ty,
        test_body: |$env:ident| $body:block
    ) => {
        rusty_fork::rusty_fork_test! {
            #[test]
            $(#[$attribute])*
            fn $test_name() {
                #[allow(clippy::crate_in_macro_def)]
                let out_dir = $crate::util::setup_test_output(stringify!($test_name));

                javac::Build::new()
                    .file(concat!("tests/java/", $java_class))
                    .output_dir(&out_dir)
                    .compile();

                #[allow(clippy::crate_in_macro_def)]
                $crate::util::attach_current_thread(|$env| {
                    let class_name = $java_class
                        .trim_end_matches(".java")
                        .split('/')
                        .last()
                        .expect("Invalid Java class path");

                    #[allow(clippy::crate_in_macro_def)]
                    $crate::util::load_test_class($env, &out_dir, class_name)?;
                    let loader = jni::refs::LoaderContext::default();
                    <$api>::get($env, &loader)?;

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
        api: $api:ty,
        expect_exception: $expected_msg:expr,
        test_body: |$env:ident| $body:block
    ) => {
        rusty_fork::rusty_fork_test! {
            #[test]
            $(#[$attribute])*
            fn $test_name() {
                #[allow(clippy::crate_in_macro_def)]
                let out_dir = $crate::util::setup_test_output(stringify!($test_name));

                javac::Build::new()
                    .file(concat!("tests/java/", $java_class))
                    .output_dir(&out_dir)
                    .compile();

                #[allow(clippy::crate_in_macro_def)]
                let result = $crate::util::attach_current_thread(|$env| {
                    let class_name = $java_class
                        .trim_end_matches(".java")
                        .split('/')
                        .last()
                        .expect("Invalid Java class path");

                    #[allow(clippy::crate_in_macro_def)]
                    $crate::util::load_test_class($env, &out_dir, class_name)?;
                    let loader = jni::refs::LoaderContext::default();
                    <$api>::get($env, &loader)?;

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
