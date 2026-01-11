#![cfg(feature = "invocation")]
mod util;

use jni::Env;
use jni::objects::JString;
use jni::{bind_java_type, jni_str};
use rusty_fork::rusty_fork_test;
use std::fs;
use std::path::{Path, PathBuf};

// Create bindings for TestFields class
bind_java_type! {
    rust_type = TestFields,
    java_type = "com.example.TestFields",
    constructors {
        fn new(),
        fn new_with_values(int_value: jint, string_value: JString),
    },
    fields {
        static static_int_field: jint,
        static static_long_field: jlong,
        static static_boolean_field: jboolean,
        static static_byte_field: jbyte,
        static static_short_field: jshort,
        static static_float_field: jfloat,
        static static_double_field: jdouble,
        static static_char_field: jchar,
        static static_string_field: JString,
        int_field: jint,
        long_field: jlong,
        boolean_field: jboolean,
        byte_field: jbyte,
        short_field: jshort,
        float_field: jfloat,
        double_field: jdouble,
        char_field: jchar,
        string_field: JString,
    },
    methods {
        fn get_int_field() -> jint,
        fn get_string_field() -> JString,
        static fn get_static_int_field() -> jint,
        static fn get_static_string_field() -> JString,
    }
}

rusty_fork_test! {
#[test]
fn test_static_primitive_fields() {
    let out_dir = setup_test_output("bind_fields_static_primitives");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        // Test reading static primitive fields
        let int_val = TestFields::static_int_field(env)?;
        assert_eq!(int_val, 42);

        let long_val = TestFields::static_long_field(env)?;
        assert_eq!(long_val, 9876543210);

        let bool_val = TestFields::static_boolean_field(env)?;
        assert!(bool_val);

        let byte_val = TestFields::static_byte_field(env)?;
        assert_eq!(byte_val, 127);

        let short_val = TestFields::static_short_field(env)?;
        assert_eq!(short_val, 32000);

        let float_val = TestFields::static_float_field(env)?;
        assert!((float_val - std::f32::consts::PI).abs() < 0.01);

        let double_val = TestFields::static_double_field(env)?;
        assert!((double_val - std::f64::consts::E).abs() < 0.00001);

        let char_val = TestFields::static_char_field(env)?;
        assert_eq!(char_val, 'X' as u16);

        Ok(())
    })
    .expect("Static primitive fields test failed");
}
}

rusty_fork_test! {
#[test]
fn test_static_string_field() {
    let out_dir = setup_test_output("bind_fields_static_string");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        // Test reading static string field
        let string_val = TestFields::static_string_field(env)?;
        assert_eq!(string_val.to_string(), "static string value");

        // Test method that returns static field
        let string_val2 = TestFields::get_static_string_field(env)?;
        assert_eq!(string_val2.to_string(), "static string value");

        Ok(())
    })
    .expect("Static string field test failed");
}
}

rusty_fork_test! {
#[test]
fn test_static_field_write() {
    let out_dir = setup_test_output("bind_fields_static_write");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        // Read initial value
        let initial_val = TestFields::static_int_field(env)?;
        assert_eq!(initial_val, 42);

        // Write new value
        TestFields::set_static_int_field(env, 100)?;

        // Read updated value
        let updated_val = TestFields::static_int_field(env)?;
        assert_eq!(updated_val, 100);

        // Verify via method call
        let method_val = TestFields::get_static_int_field(env)?;
        assert_eq!(method_val, 100);

        Ok(())
    })
    .expect("Static field write test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_primitive_fields() {
    let out_dir = setup_test_output("bind_fields_instance_primitives");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        let obj = TestFields::new(env)?;

        // Test reading instance primitive fields
        let int_val = obj.int_field(env)?;
        assert_eq!(int_val, 10);

        let long_val = obj.long_field(env)?;
        assert_eq!(long_val, 100);

        let bool_val = obj.boolean_field(env)?;
        assert!(!bool_val);

        let byte_val = obj.byte_field(env)?;
        assert_eq!(byte_val, 1);

        let short_val = obj.short_field(env)?;
        assert_eq!(short_val, 200);

        let float_val = obj.float_field(env)?;
        assert!((float_val - 1.5).abs() < 0.01);

        let double_val = obj.double_field(env)?;
        assert!((double_val - 2.5).abs() < 0.01);

        let char_val = obj.char_field(env)?;
        assert_eq!(char_val, 'A' as u16);

        Ok(())
    })
    .expect("Instance primitive fields test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_string_field() {
    let out_dir = setup_test_output("bind_fields_instance_string");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        let obj = TestFields::new(env)?;

        // Test reading instance string field
        let string_val = obj.string_field(env)?;
        assert_eq!(string_val.to_string(), "instance string");

        // Test method that returns instance field
        let string_val2 = obj.get_string_field(env)?;
        assert_eq!(string_val2.to_string(), "instance string");

        Ok(())
    })
    .expect("Instance string field test failed");
}
}

rusty_fork_test! {
#[test]
fn test_instance_field_write() {
    let out_dir = setup_test_output("bind_fields_instance_write");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        let obj = TestFields::new(env)?;

        // Read initial value
        let initial_val = obj.int_field(env)?;
        assert_eq!(initial_val, 10);

        // Write new value
        obj.set_int_field(env, 999)?;

        // Read updated value
        let updated_val = obj.int_field(env)?;
        assert_eq!(updated_val, 999);

        // Verify via method call
        let method_val = obj.get_int_field(env)?;
        assert_eq!(method_val, 999);

        Ok(())
    })
    .expect("Instance field write test failed");
}
}

rusty_fork_test! {
#[test]
fn test_constructor_with_values() {
    let out_dir = setup_test_output("bind_fields_constructor");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        let string_val = JString::from_str(env, "custom string")?;
        let obj = TestFields::new_with_values(env, 50, &string_val)?;

        // Verify fields were set correctly by constructor
        let int_val = obj.int_field(env)?;
        assert_eq!(int_val, 50);

        let long_val = obj.long_field(env)?;
        assert_eq!(long_val, 500); // intValue * 10

        let string_field = obj.string_field(env)?;
        assert_eq!(string_field.to_string(), "custom string");

        let bool_val = obj.boolean_field(env)?;
        assert!(bool_val); // true because intValue > 0

        Ok(())
    })
    .expect("Constructor with values test failed");
}
}

rusty_fork_test! {
#[test]
fn test_multiple_instance_fields_write() {
    let out_dir = setup_test_output("bind_fields_multiple_write");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        let obj = TestFields::new(env)?;

        // Write multiple fields
        obj.set_int_field(env, 777)?;
        obj.set_long_field(env, 888)?;
        obj.set_boolean_field(env, true)?;
        obj.set_double_field(env, 9.99)?;

        let new_string = JString::from_str(env, "updated")?;
        obj.set_string_field(env, &new_string)?;

        // Read back all values
        assert_eq!(obj.int_field(env)?, 777);
        assert_eq!(obj.long_field(env)?, 888);
        assert!(obj.boolean_field(env)?);
        assert!((obj.double_field(env)? - 9.99).abs() < 0.01);
        assert_eq!(obj.string_field(env)?.to_string(), "updated");

        Ok(())
    })
    .expect("Multiple instance fields write test failed");
}
}

// Helper function to load the TestFields class
fn load_test_fields_class(env: &mut Env, out_dir: &Path) -> jni::errors::Result<()> {
    let class_path = out_dir.join("com/example/TestFields.class");
    assert!(class_path.exists(), "TestFields.class not found");

    let class_bytes = fs::read(&class_path).expect("Failed to read TestFields.class");

    let class_loader = jni::objects::JClassLoader::get_system_class_loader(env)
        .expect("Failed to get system class loader");

    env.define_class(
        Some(jni_str!("com/example/TestFields")),
        &class_loader,
        &class_bytes,
    )
    .expect("Failed to define TestFields class");

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
