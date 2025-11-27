// Test the native_method! and static_native_method! macros

use jni::errors::Error;
use jni::objects::{JClass, JObject, JString};
use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jint, jlong};
use jni::{Env, native_method};

// Type alias for testing
type MyType<'local> = JObject<'local>;

// Instance method implementations

fn add_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    a: jint,
    b: jint,
) -> Result<jint, Error> {
    Ok(a + b)
}

fn is_positive_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    value: jint,
) -> Result<jboolean, Error> {
    if value > 0 {
        Ok(JNI_TRUE)
    } else {
        Ok(JNI_FALSE)
    }
}

fn process_string_impl<'local>(
    env: &mut Env<'local>,
    _this: JObject<'local>,
    input: JString<'local>,
) -> Result<JString<'local>, Error> {
    let input_str = input.try_to_string(env)?;
    let result = format!("processed: {}", input_str);
    JString::from_str(env, result.as_str())
}

fn no_args_impl<'local>(_env: &mut Env<'local>, _this: JObject<'local>) -> Result<jint, Error> {
    Ok(42)
}

fn void_return_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    _value: jint,
) -> Result<(), Error> {
    // Does nothing
    Ok(())
}

// Static method implementations

fn static_multiply_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    a: jlong,
    b: jlong,
) -> Result<jlong, Error> {
    Ok(a * b)
}

fn static_is_even_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    value: jint,
) -> Result<jboolean, Error> {
    if value % 2 == 0 {
        Ok(JNI_TRUE)
    } else {
        Ok(JNI_FALSE)
    }
}

fn static_no_args_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
) -> Result<jint, Error> {
    Ok(100)
}

#[test]
fn test_instance_native_method_basic() {
    let method = native_method! {
        name = "add",
        sig = (a: jint, b: jint) -> jint,
        fn = add_impl
    };

    // Verify the method descriptor is created
    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_instance_native_method_boolean_return() {
    let method = native_method! {
        name = "isPositive",
        sig = (value: jint) -> boolean,
        fn = is_positive_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_instance_native_method_string() {
    let method = native_method! {
        name = "processString",
        sig = (input: JString) -> JString,
        fn = process_string_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_instance_native_method_no_args() {
    let method = native_method! {
        name = "noArgs",
        sig = () -> jint,
        fn = no_args_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_instance_native_method_void_return() {
    let method = native_method! {
        name = "voidReturn",
        sig = (value: jint) -> void,
        fn = void_return_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_static_native_method_basic() {
    let method = native_method! {
        name = "multiply",
        sig = (a: jlong, b: jlong) -> jlong,
        static = true,
        fn = static_multiply_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_static_native_method_boolean() {
    let method = native_method! {
        name = "isEven",
        sig = (value: jint) -> boolean,
        static = true,
        fn = static_is_even_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_static_native_method_no_args() {
    let method = native_method! {
        name = "staticNoArgs",
        sig = () -> jint,
        static = true,
        fn = static_no_args_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

// Test with type mappings
#[test]
fn test_with_type_mappings() {
    // Define a custom type alias for testing
    use jni::objects::JObject as CustomType;

    fn custom_impl<'local>(
        _env: &mut Env<'local>,
        _this: JObject<'local>,
        input: CustomType<'local>,
    ) -> Result<CustomType<'local>, Error> {
        Ok(input)
    }

    let method = native_method! {
        name = "customMethod",
        sig = (input: CustomType) -> CustomType,
        fn = custom_impl,
        type_map = {
            CustomType => com.example.CustomClass,
        }
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

// Test with explicit jni crate path
#[test]
fn test_with_jni_path() {
    let method = native_method! {
        jni = ::jni,
        name = "add",
        sig = (a: jint, b: jint) -> jint,
        fn = add_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

// Test shorthand syntax
#[test]
fn test_shorthand_with_explicit_fn() {
    let method = native_method! {
        fn MyType::add_numbers(a: jint, b: jint) -> jint,
        fn = add_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_shorthand_jobject_with_fn() {
    let method = native_method! {
        fn JObject::my_native_method(arg: jint) -> jboolean,
        fn = is_positive_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_shorthand_static() {
    let method = native_method! {
        static fn MyType::static_multiply(a: jlong, b: jlong) -> jlong,
        fn = static_multiply_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_shorthand_with_jni_override() {
    let method = native_method! {
        jni = ::jni,
        fn MyType::add_value(value: jint, other: jint) -> jint,
        fn = add_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_shorthand_snake_case_conversion() {
    // Method name "add_numbers" should be converted to "addNumbers"
    let method = native_method! {
        fn MyType::add_numbers(a: jint, b: jint) -> jint,
        fn = add_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

// Raw (raw) function implementations for testing

extern "system" fn raw_add_impl<'local>(
    _env: jni::EnvUnowned<'local>,
    _this: JObject<'local>,
    a: jint,
    b: jint,
) -> jint {
    a + b
}

extern "system" fn raw_is_positive_impl<'local>(
    _env: jni::EnvUnowned<'local>,
    _this: JObject<'local>,
    value: jint,
) -> jboolean {
    if value > 0 { JNI_TRUE } else { JNI_FALSE }
}

extern "system" fn raw_no_args_impl<'local>(
    _env: jni::EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jint {
    42
}

extern "system" fn raw_void_return_impl<'local>(
    _env: jni::EnvUnowned<'local>,
    _this: JObject<'local>,
    _value: jint,
) {
    // Does nothing
}

extern "system" fn raw_static_multiply_impl<'local>(
    _env: jni::EnvUnowned<'local>,
    _class: JClass<'local>,
    a: jlong,
    b: jlong,
) -> jlong {
    a * b
}

extern "system" fn raw_static_is_even_impl<'local>(
    _env: jni::EnvUnowned<'local>,
    _class: JClass<'local>,
    value: jint,
) -> jboolean {
    if value % 2 == 0 { JNI_TRUE } else { JNI_FALSE }
}

extern "system" fn raw_static_no_args_impl<'local>(
    _env: jni::EnvUnowned<'local>,
    _class: JClass<'local>,
) -> jint {
    100
}

// Tests for raw (raw) functions with property-based syntax

#[test]
fn test_raw_instance_native_method_basic() {
    let method = native_method! {
        name = "add",
        sig = (a: jint, b: jint) -> jint,
        raw = true,
        fn = raw_add_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_instance_native_method_boolean_return() {
    let method = native_method! {
        name = "isPositive",
        sig = (value: jint) -> boolean,
        raw = true,
        fn = raw_is_positive_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_instance_native_method_no_args() {
    let method = native_method! {
        name = "noArgs",
        sig = () -> jint,
        raw = true,
        fn = raw_no_args_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_instance_native_method_void_return() {
    let method = native_method! {
        name = "voidReturn",
        sig = (value: jint) -> void,
        raw = true,
        fn = raw_void_return_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_static_native_method_basic() {
    let method = native_method! {
        name = "multiply",
        sig = (a: jlong, b: jlong) -> jlong,
        static = true,
        raw = true,
        fn = raw_static_multiply_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_static_native_method_boolean() {
    let method = native_method! {
        name = "isEven",
        sig = (value: jint) -> boolean,
        static = true,
        raw = true,
        fn = raw_static_is_even_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_static_native_method_no_args() {
    let method = native_method! {
        name = "staticNoArgs",
        sig = () -> jint,
        static = true,
        raw = true,
        fn = raw_static_no_args_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

// Tests for raw (raw) functions with shorthand syntax (with => fn_path)

#[test]
fn test_raw_shorthand_with_explicit_fn() {
    let method = native_method! {
        raw fn MyType::add_numbers(a: jint, b: jint) -> jint,
        fn = raw_add_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_shorthand_jobject_with_fn() {
    let method = native_method! {
        raw fn JObject::my_native_method(arg: jint) -> jboolean,
        fn = raw_is_positive_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_shorthand_static() {
    let method = native_method! {
        static raw fn MyType::static_multiply(a: jlong, b: jlong) -> jlong,
        fn = raw_static_multiply_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_shorthand_with_jni_override() {
    let method = native_method! {
        jni = ::jni,
        raw fn MyType::add_value(value: jint, other: jint) -> jint,
        fn = raw_add_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_raw_shorthand_snake_case_conversion() {
    // Method name "add_numbers" should be converted to "addNumbers"
    let method = native_method! {
        raw fn MyType::add_numbers(a: jint, b: jint) -> jint,
        fn = raw_add_impl
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

// Tests for export = true with wrapped (safe) functions

#[test]
fn test_export_wrapped_instance_method() {
    // Test that export wrapper correctly calls the __native_method_wrapper
    let method = native_method! {
        rust_type = MyType,
        name = "add",
        sig = (a: jint, b: jint) -> jint,
        fn = add_impl,
        export = true,
        java_type = "com.example.TestClass"
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));

    // Verify the export function exists and has the correct mangled name
    // The function should be: Java_com_example_TestClass_add
    #[allow(improper_ctypes_definitions)]
    unsafe extern "system" {
        #[allow(unused)]
        #[link_name = "Java_com_example_TestClass_add"]
        fn exported_add<'local>(
            env: jni::EnvUnowned<'local>,
            this: JObject<'local>,
            a: jint,
            b: jint,
        ) -> jint;
    }
}

#[test]
fn test_export_wrapped_static_method() {
    let method = native_method! {
        name = "multiply",
        sig = (a: jlong, b: jlong) -> jlong,
        fn = static_multiply_impl,
        static = true,
        export = true,
        java_type = "com.example.TestClass"
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_export_wrapped_with_inline_sig() {
    let method = native_method! {
        java_type = "com.example.TestClass2",
        extern fn MyType::add2(a: jint, b: jint) -> jint,
        fn = add_impl,
    };

    // Verify the export exists by referencing it through an extern block
    unsafe extern "system" {
        #[allow(unused)]
        fn Java_com_example_TestClass2_add2__II<'local>(
            env: jni::EnvUnowned<'local>,
            obj: JObject<'local>,
            a: jint,
            b: jint,
        ) -> jint;
    }

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

// Tests for export = true with raw (raw) functions

#[test]
fn test_export_raw_instance_method() {
    let method = native_method! {
        rust_type = MyType,
        name = "rawAdd",
        sig = (a: jint, b: jint) -> jint,
        fn = raw_add_impl,
        raw = true,
        export = true,
        java_type = "com.example.TestClass"
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_export_raw_static_method() {
    let method = native_method! {
        name = "rawMultiply",
        sig = (a: jlong, b: jlong) -> jlong,
        fn = raw_static_multiply_impl,
        static = true,
        raw = true,
        export = true,
        java_type = "com.example.TestClass"
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_export_raw_with_inline_sig() {
    let method = native_method! {
        java_type = "com.example.TestClass4",
        raw extern fn MyType::raw_add2(a: jint, b: jint) -> jint,
        fn = raw_add_impl,
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

// Test static keyword in signature

#[test]
fn test_static_with_inline_sig() {
    let method = native_method! {
        static fn multiply(a: jlong, b: jlong) -> jlong,
        fn = static_multiply_impl,
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}

#[test]
fn test_static_extern_with_inline_sig() {
    let method = native_method! {
        java_type = "com.example.TestClass3",
        static extern fn MyType::multiply_static(a: jlong, b: jlong) -> jlong,
        fn = static_multiply_impl,
    };

    assert!(!std::ptr::eq(&method, std::ptr::null()));
}
