use jni::EnvUnowned;
use jni::objects::JObject;
use jni::sys::jlong;
use jni_macros::native_method;

// This should fail because the raw function returns jlong but signature expects jint

extern "system" fn my_method_impl<'local>(
    _env: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jlong {
    // Wrong! Should return jint
    0
}

fn main() {
    let _method = native_method! {
        name = "myMethod",
        sig = () -> jint,
        fn = my_method_impl,
        raw = true,
    };
}
