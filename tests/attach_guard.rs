#![cfg(feature = "invocation")]
extern crate jni;
extern crate error_chain;

use jni::{
    objects::JValue,
    sys::jint,
};

mod util;
use util::jvm;

#[test]
fn attach_guard() {
    // `AttachGuard` detaches thread on drop.
    assert_eq!(jvm().threads_attached(), 0);
    {
        let guard = jvm().attach_current_thread().unwrap();
        assert_eq!(jvm().threads_attached(), 1);
        let val = guard
            .call_static_method("java/lang/Math", "abs", "(I)I", &[JValue::from(-1 as jint)])
            .unwrap().i().unwrap();
        assert_eq!(val, 1);
    }
    assert_eq!(jvm().threads_attached(), 0);
    // Verify that this thread is really detached.
    assert!(jvm().get_env().is_err());
}
