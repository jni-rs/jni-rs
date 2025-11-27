use jni::EnvUnowned;
use jni::objects::JObject;
use jni::sys::jlong;
use jni_macros::native_method;

// This should fail with shorthand syntax - raw function returns jlong instead of jint

type MyClass<'local> = JObject<'local>;

extern "system" fn my_method_impl<'local>(
    _env: EnvUnowned<'local>,
    _this: JObject<'local>,
) -> jlong {
    // Wrong! Should return jint
    0
}

fn main() {
    let _method = native_method! {
        raw fn MyClass::myMethod() -> jint,
        fn = my_method_impl
    };
}
