use jni::Env;
use jni::errors::Error;
use jni::objects::JObject;
use jni::sys::jint;
use jni_macros::native_method;

// This should fail because the function has 1 param but signature expects 2

fn my_method_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    _value: jint, // Missing second parameter!
) -> Result<(), Error> {
    Ok(())
}

fn main() {
    let _method = native_method! {
        name = "myMethod",
        sig = (a: jint, b: jint) -> void,
        fn = my_method_impl
    };
}
