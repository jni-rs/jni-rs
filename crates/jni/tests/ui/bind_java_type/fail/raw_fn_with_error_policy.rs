// Test that error_policy cannot be used with raw = true

use jni::EnvUnowned;
use jni::bind_java_type;
use jni::sys::{JNI_TRUE, jboolean, jint};

extern "system" fn raw_method<'local>(
    _env: EnvUnowned<'local>,
    _this: TestClass<'local>,
    _value: jint,
) -> jboolean {
    JNI_TRUE
}

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    native_methods = {
        fn test_method {
            sig = (value: jint) -> jboolean,
            fn = raw_method,
            raw = true,
            error_policy = ThrowRuntimeExAndDefault
        }
    }
}

fn main() {}
