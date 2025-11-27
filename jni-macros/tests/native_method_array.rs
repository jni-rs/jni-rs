// Test that native_method_struct! can be used in an array

use jni::errors::Error;
use jni::objects::{JClass, JObject};
use jni::sys::{jint, jlong};
use jni::{Env, NativeMethod, native_method};

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

fn multiply_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    a: jint,
    b: jint,
) -> Result<jint, Error> {
    Ok(a * b)
}

// Static method implementations
fn static_add_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    a: jlong,
    b: jlong,
) -> Result<jlong, Error> {
    Ok(a + b)
}

fn static_multiply_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    a: jlong,
    b: jlong,
) -> Result<jlong, Error> {
    Ok(a * b)
}

#[test]
fn test_native_methods_in_array() {
    let methods: &[NativeMethod] = &[
        native_method! {
            name = "staticAdd",
            sig = (a: jlong, b: jlong) -> jlong,
            fn = static_add_impl,
            static = true,
        },
        native_method! {
            name = "staticMultiply",
            sig = (a: jlong, b: jlong) -> jlong,
            fn = static_multiply_impl,
            static = true,
        },
        native_method! {
            name = "add",
            sig = (a: jint, b: jint) -> jint,
            fn = add_impl
        },
        native_method! {
            name = "multiply",
            sig = (a: jint, b: jint) -> jint,
            fn = multiply_impl
        },
    ];

    // Verify we created 4 methods
    assert_eq!(methods.len(), 4);
}

#[test]
fn test_native_methods_in_array_shorthand() {
    let methods: &[NativeMethod] = &[
        native_method! {
            static fn MyType::staticAdd(a: jlong, b: jlong) -> jlong,
            fn = static_add_impl
        },
        native_method! {
            static fn MyType::staticMultiply(a: jlong, b: jlong) -> jlong,
            fn = static_multiply_impl
        },
        native_method! {
            fn MyType::add(a: jint, b: jint) -> jint,
            fn = add_impl
        },
        native_method! {
            fn MyType::multiply(a: jint, b: jint) -> jint,
            fn = multiply_impl
        },
    ];

    // Verify we created 4 methods
    assert_eq!(methods.len(), 4);
}
