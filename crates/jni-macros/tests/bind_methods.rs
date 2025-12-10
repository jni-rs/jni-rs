mod util;

use jni::Env;
use jni::objects::JString;
use jni::{bind_java_type, jni_str};
use rusty_fork::rusty_fork_test;
use std::fs;
use std::path::{Path, PathBuf};

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

// Helper function to load the TestMethods class
fn load_test_methods_class(env: &mut Env, out_dir: &Path) -> jni::errors::Result<()> {
    let class_path = out_dir.join("com/example/TestMethods.class");
    assert!(class_path.exists(), "TestMethods.class not found");

    let class_bytes = fs::read(&class_path).expect("Failed to read TestMethods.class");

    let class_loader = jni::objects::JClassLoader::get_system_class_loader(env)
        .expect("Failed to get system class loader");

    env.define_class(
        Some(jni_str!("com/example/TestMethods")),
        &class_loader,
        &class_bytes,
    )
    .expect("Failed to define TestMethods class");

    Ok(())
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
