#![cfg(feature = "invocation")]

mod util;
use jni::JNIVersion;
use util::{attach_current_thread_permanently, call_java_abs, detach_current_thread, jvm};

#[test]
pub fn explicit_detach_detaches_thread_attached_permanently() {
    assert_eq!(jvm().threads_attached(), 0);
    let mut guard = attach_current_thread_permanently();
    let val = call_java_abs(&mut guard, -1);
    assert_eq!(val, 1);
    assert_eq!(jvm().threads_attached(), 1);

    // # Safety
    // we won't be trying to use a pre-existing (invalid) `JNIEnv` after detaching
    unsafe {
        detach_current_thread();
    }
    assert_eq!(jvm().threads_attached(), 0);
    unsafe { assert!(jvm().get_env(JNIVersion::V1_4).is_err()) };
}
