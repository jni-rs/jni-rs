// Example showing native_method! used in an array for register_native_methods

use jni::errors::Error;
use jni::objects::{JClass, JObject};
use jni::sys::{jint, jlong};
use jni::{Env, NativeMethod, native_method};

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

const ADD_METHOD: NativeMethod = native_method! {
    name = "staticAdd",
    sig = (a: jlong, b: jlong) -> jlong,
    fn = static_add_impl,
    static = true,
};

// This is an example of how you would use these macros with register_native_methods
#[allow(dead_code)]
fn register_my_methods(env: &mut Env, class: JClass) -> jni::errors::Result<()> {
    // Create an array of NativeMethods using the macros
    const METHODS: &[NativeMethod] = &[
        ADD_METHOD,
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

    // Register all the methods at once
    unsafe {
        env.register_native_methods(class, METHODS)?;
    }

    Ok(())
}

#[test]
fn test_example_compiles() {
    // This test just verifies the example compiles
}
