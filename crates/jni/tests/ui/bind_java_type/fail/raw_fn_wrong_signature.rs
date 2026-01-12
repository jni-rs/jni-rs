// Test that fn with raw = true must match the declared signature

use jni::EnvUnowned;
use jni::bind_java_type;
use jni::sys::{jboolean, jint, jlong};

// This function has the WRONG signature - takes jlong instead of jint
extern "system" fn raw_check_positive<'local>(
    _env: EnvUnowned<'local>,
    _this: TestClass<'local>,
    value: jlong, // Wrong! Should be jint
) -> jboolean {
    if value > 0 {
        jni::sys::JNI_TRUE
    } else {
        jni::sys::JNI_FALSE
    }
}

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    native_methods = {
        // Declares signature with jint, but raw function takes jlong
        fn check_positive {
            sig = (value: jint) -> jboolean,
            fn = raw_check_positive,
            raw = true,
        }
    }
}

fn main() {}
