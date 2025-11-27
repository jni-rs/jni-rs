use jni::Env;
use jni::errors::Error;
use jni::objects::JObject;
use jni::sys::jlong;
use jni_macros::native_method;

// This should fail because the function returns jlong but signature expects jint

fn my_method_impl<'local>(_env: &mut Env<'local>, _this: JObject<'local>) -> Result<jlong, Error> {
    // Wrong! Should return jint
    Ok(0)
}

fn main() {
    let _method = native_method! {
        name = "myMethod",
        sig = () -> jint,
        fn = my_method_impl
    };
}
