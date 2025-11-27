use jni::Env;
use jni::errors::Error;
use jni::objects::JObject;
use jni::sys::{jint, jlong};
use jni_macros::native_method;

// This should fail with shorthand syntax - returns jlong instead of jint

type MyClass<'local> = JObject<'local>;

fn my_method_impl<'local>(_env: &mut Env<'local>, _this: JObject<'local>) -> Result<jlong, Error> {
    // Wrong! Should return jint
    Ok(0)
}

fn main() {
    let _method = native_method! {
        fn MyClass::myMethod() -> jint,
        fn = my_method_impl
    };
}
