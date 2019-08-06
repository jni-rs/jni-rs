#![cfg(feature = "invocation")]

mod util;
use util::{attach_current_thread_permanently, call_java_abs, detach_current_thread, jvm};

#[test]
pub fn explicit_detach_detaches_thread_attached_permanently() {
    assert_eq!(jvm().threads_attached(), 0);
    let guard = attach_current_thread_permanently();
    let val = call_java_abs(&guard, -1);
    assert_eq!(val, 1);
    assert_eq!(jvm().threads_attached(), 1);

    detach_current_thread();
    assert_eq!(jvm().threads_attached(), 0);
    assert!(jvm().get_env().is_err());
}
