// ANCHOR: imports
use jni::objects::JClass;
use jni::sys::jint;
use jni::JNIEnv;
// ANCHOR_END: imports

// ANCHOR: try_java_imports
use crate::error::try_java;
use anyhow::Context;
// ANCHOR_END: try_java_imports

#[cfg(feature = "division_0")]
// ANCHOR: division_0
pub fn Java_jni_1rs_1book_NativeAPI_divide(a: jint, b: jint) -> jint {
    a / b
}
// ANCHOR_END: division_0

#[cfg(feature = "division_1")]
// ANCHOR: division_1
pub fn Java_jni_1rs_1book_NativeAPI_divide(_env: JNIEnv, _class: JClass, a: jint, b: jint) -> jint {
    a / b
}
// ANCHOR_END: division_1

#[cfg(feature = "division_2")]
// ANCHOR: division_2
#[no_mangle]
pub fn Java_jni_1rs_1book_NativeAPI_divide(_env: JNIEnv, _class: JClass, a: jint, b: jint) -> jint {
    a / b
}
// ANCHOR_END: division_2

#[cfg(feature = "division_3")]
// ANCHOR: division_3
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_divide(
    _env: JNIEnv,
    _class: JClass,
    a: jint,
    b: jint,
) -> jint {
    a / b
}
// ANCHOR_END: division_3

#[cfg(feature = "division_complete")]
// ANCHOR: try_java
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_divide(
    env: JNIEnv,
    _class: JClass,
    a: jint,
    b: jint,
) -> jint {
    try_java(env, 0, || Ok(a / b))
}
// ANCHOR_END: try_java
