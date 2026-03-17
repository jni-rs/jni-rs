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

        // Fields for testing non_null validation
        nullable_string_field: JString,  // Can be null
        non_null required_string_field: JString,  // Shorthand syntax - must not be null
        validated_string_field {  // Block syntax with explicit non_null
            sig = JString,
            non_null = true,
        },

        // Fields for testing cfg attribute support
        // These are guarded by _cfg_test which is never enabled in tests
        #[cfg(feature = "_cfg_test")]
        static static_cfg_test_field: jint,
        #[cfg(feature = "_cfg_test")]
        instance_cfg_test_field: jint,

        // These are guarded by invocation which is always enabled in tests
        #[cfg(feature = "invocation")]
        static static_invocation_field: jint,
        #[cfg(feature = "invocation")]
        instance_invocation_field: jint,
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

        // First, check attempts to get/set instances fields with a null reference

        let res = TestFields::null().int_field(env);
        assert!(matches!(res, Err(jni::errors::Error::NullPtr(_))));
        let res = TestFields::null().set_int_field(env, 0);
        assert!(matches!(res, Err(jni::errors::Error::NullPtr(_))));

        let obj = TestFields::new(env)?;

        // Test getting/setting instance primitive fields
        let int_val = obj.int_field(env)?;
        assert_eq!(int_val, 10);
        obj.set_int_field(env, 20)?;
        let int_val = obj.int_field(env)?;
        assert_eq!(int_val, 20);

        let long_val = obj.long_field(env)?;
        assert_eq!(long_val, 100);
        obj.set_long_field(env, 200)?;
        let long_val = obj.long_field(env)?;
        assert_eq!(long_val, 200);

        let bool_val = obj.boolean_field(env)?;
        assert!(!bool_val);
        obj.set_boolean_field(env, true)?;
        let bool_val = obj.boolean_field(env)?;
        assert!(bool_val);

        let byte_val = obj.byte_field(env)?;
        assert_eq!(byte_val, 1);
        obj.set_byte_field(env, 2)?;
        let byte_val = obj.byte_field(env)?;
        assert_eq!(byte_val, 2);

        let short_val = obj.short_field(env)?;
        assert_eq!(short_val, 200);
        obj.set_short_field(env, 300)?;
        let short_val = obj.short_field(env)?;
        assert_eq!(short_val, 300);

        let float_val = obj.float_field(env)?;
        assert!((float_val - 1.5).abs() < 0.01);
        obj.set_float_field(env, 3.5)?;
        let float_val = obj.float_field(env)?;
        assert!((float_val - 3.5).abs() < 0.01);

        let double_val = obj.double_field(env)?;
        assert!((double_val - 2.5).abs() < 0.01);
        obj.set_double_field(env, 4.5)?;
        let double_val = obj.double_field(env)?;
        assert!((double_val - 4.5).abs() < 0.01);

        let char_val = obj.char_field(env)?;
        assert_eq!(char_val, 'A' as u16);
        obj.set_char_field(env, 'B' as u16)?;
        let char_val = obj.char_field(env)?;
        assert_eq!(char_val, 'B' as u16);

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

        Ok(())
    })
    .expect("Instance string field test failed");
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
fn test_cfg_guarded_fields() {
    let out_dir = setup_test_output("bind_fields_cfg");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        // The TestFields binding includes fields guarded by both:
        // - invocation feature (always enabled in tests) - we test these
        // - _cfg_test feature (never enabled in tests) - these are not available

        // The following would fail to compile if uncommented:
        // TestFields::static_cfg_test_field(env)?;
        // obj.instance_cfg_test_field(env)?;
        // Note: we have a separate ui / trybuild test to check this

        let val = TestFields::static_invocation_field(env)?;
        assert_eq!(val, 55);

        TestFields::set_static_invocation_field(env, 111)?;
        let val = TestFields::static_invocation_field(env)?;
        assert_eq!(val, 111);

        let obj = TestFields::new(env)?;
        let val = obj.instance_invocation_field(env)?;
        assert_eq!(val, 88);

        obj.set_instance_invocation_field(env, 222)?;
        let val = obj.instance_invocation_field(env)?;
        assert_eq!(val, 222);

        #[cfg(feature = "_cfg_test")]
        {
            // These should compile and run if _cfg_test feature is enabled
            let val = TestFields::static_cfg_test_field(env)?;
            assert_eq!(val, 99);

            let obj = TestFields::new(env)?;
            let val = obj.instance_cfg_test_field(env)?;
            assert_eq!(val, 77);
        }

        Ok(())
    })
    .expect("Cfg-guarded fields test failed");
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

// Version check function for testing requires attribute
fn field_check_version_21() -> bool {
    false // Simulate version < 21
}

fn field_check_version(version: u32) -> bool {
    const CURRENT_VERSION: u32 = 20;
    version <= CURRENT_VERSION
}

// Bindings for testing requires attribute with fields
bind_java_type! {
    rust_type = TestFieldsWithRequires,
    java_type = "com.example.TestFields",
    constructors {
        fn new(),
    },
    fields {
        int_field: jint,  // Should work (no requires)

        // This field should fail at runtime
        #[jni(requires = field_check_version_21())]
        long_field: jlong,

        // This field should work
        #[jni(requires = field_check_version(19))]
        string_field: JString,

        // Test with literal expression
        #[jni(requires = "true")]
        boolean_field: jboolean,

        // Test with literal expression false
        #[jni(requires = "false")]
        byte_field: jbyte,
    }
}

rusty_fork_test! {
#[test]
fn test_non_null_field_validation() {
    let out_dir = setup_test_output("bind_fields_non_null");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        let obj = TestFields::new(env)?;

        // Test nullable field - should allow null without error
        // We know from TestFields constructor that this field is initialized to null
        let nullable = obj.nullable_string_field(env)?;
        assert!(nullable.is_null(), "Expected nullable field to initially be null");

        // Test getter with non_null (shorthand syntax)
        // We know this field is initialized to null, so getting it should fail
        let result = obj.required_string_field(env);
        assert!(
            matches!(result, Err(jni::errors::Error::NullPtr(_))),
            "Expected Error::NullPtr when getting non_null field that is null"
        );

        // Test getter with non_null (block syntax)
        // We know this field is initialized to null, so getting it should fail
        let result = obj.validated_string_field(env);
        assert!(
            matches!(result, Err(jni::errors::Error::NullPtr(_))),
            "Expected Error::NullPtr when getting non_null field that is null"
        );

        // Test setting a non_null field to null should also fail
        let null_string = JString::null();
        let result = obj.set_required_string_field(env, &null_string);
        assert!(
            matches!(result, Err(jni::errors::Error::NullPtr(_))),
            "Setting non_null field to null should return NullPtr error"
        );

        // Test setting a non_null field to a non-null value should succeed
        let valid_string = JString::from_str(env, "valid value")?;
        obj.set_required_string_field(env, &valid_string)?;

        // Now getting the field should succeed
        let result = obj.required_string_field(env)?;
        assert!(!result.is_null());
        assert_eq!(result.to_string(), "valid value");

        // Test setting validated_string_field (block syntax) to null should also fail
        let result = obj.set_validated_string_field(env, &null_string);
        assert!(
            matches!(result, Err(jni::errors::Error::NullPtr(_))),
            "Setting non_null field to null should return NullPtr error"
        );

        // Test setting validated_string_field to a non-null value should succeed
        let another_valid = JString::from_str(env, "another valid")?;
        obj.set_validated_string_field(env, &another_valid)?;

        // Now getting the field should succeed
        let result = obj.validated_string_field(env)?;
        assert!(!result.is_null());
        assert_eq!(result.to_string(), "another valid");

        // Test that nullable field allows setting null (doesn't fail)
        obj.set_nullable_string_field(env, &null_string)?;
        let result = obj.nullable_string_field(env)?;
        assert!(result.is_null(), "Expected nullable field to be null after setting to null");

        Ok(())
    })
    .expect("Non-null field validation test failed");
}
}

rusty_fork_test! {
#[test]
fn test_requires_attribute_fields() {
    let out_dir = setup_test_output("bind_fields_requires");

    javac::Build::new()
        .file("tests/java/com/example/TestFields.java")
        .output_dir(&out_dir)
        .compile();

    util::attach_current_thread(|env| {
        load_test_fields_class(env, &out_dir)?;

        let obj = TestFieldsWithRequires::new(env)?;

        // Test that int_field works (no requires)
        let value = obj.int_field(env)?;
        assert_eq!(value, 10); // default value from constructor

        obj.set_int_field(env, 42)?;
        let value = obj.int_field(env)?;
        assert_eq!(value, 42);

        // Test that long_field fails (requires = false)
        let result = obj.long_field(env);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, jni::errors::Error::UnsupportedVersion));
        }

        let result = obj.set_long_field(env, 123456);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, jni::errors::Error::UnsupportedVersion));
        }

        // Test that string_field works (requires = true)
        let _value = obj.string_field(env)?;
        let new_string = JString::from_str(env, "test")?;
        obj.set_string_field(env, &new_string)?;
        let result = obj.string_field(env)?;
        assert_eq!(result.to_string(), "test");

        // Test that boolean_field works (requires = "true")
        let value = obj.boolean_field(env)?;
        assert!(!value); // default value
        obj.set_boolean_field(env, true)?;
        let value = obj.boolean_field(env)?;
        assert!(value);

        // Test that byte_field fails (requires = "false")
        let result = obj.byte_field(env);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, jni::errors::Error::UnsupportedVersion));
        }

        let result = obj.set_byte_field(env, 10);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, jni::errors::Error::UnsupportedVersion));
        }

        Ok(())
    })
    .expect("Requires attribute fields test failed");
}
}
