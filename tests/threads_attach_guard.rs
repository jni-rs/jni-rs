#![cfg(feature = "invocation")]
extern crate jni;
extern crate error_chain;

mod util;
use util::{attach_current_thread, jvm, call_java_abs};

#[test]
fn thread_attach_guard_detaches_on_drop() {
    assert_eq!(jvm().threads_attached(), 0);
    {
        let guard = attach_current_thread();
        assert_eq!(jvm().threads_attached(), 1);
        let val = call_java_abs(&guard, -1);
        assert_eq!(val, 1);
    }
    assert_eq!(jvm().threads_attached(), 0);
    // Verify that this thread is really detached.
    assert!(jvm().get_env().is_err());
}
