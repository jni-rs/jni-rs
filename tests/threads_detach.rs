#![cfg(feature = "invocation")]
extern crate jni;
extern crate error_chain;

use std::thread::spawn;

use jni::{
    objects::JValue,
    sys::jint,
};

mod util;
use util::jvm;

// We forced to combine several tests in one function, because every test function is running in
// a separate thread and interferes the results of others.
#[test]
fn thread_attach() {
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

    // Thread detaches when finished.
    let thread = spawn(|| {
        let env = jvm().attach_current_thread_permanently().unwrap();
        let val = env
            .call_static_method("java/lang/Math", "abs", "(I)I", &[JValue::from(-2 as jint)])
            .unwrap().i().unwrap();
        assert_eq!(val, 2);
        assert_eq!(jvm().threads_attached(), 1);
    });

    thread.join().unwrap();
    assert_eq!(jvm().threads_attached(), 0);

    // Daemon threads works the same way
    let thread = spawn(|| {
        let env = jvm().attach_current_thread_as_daemon().unwrap();
        let val = env
            .call_static_method("java/lang/Math", "abs", "(I)I", &[JValue::from(-3 as jint)])
            .unwrap().i().unwrap();
        assert_eq!(val, 3);
        assert_eq!(jvm().threads_attached(), 1);
    });

    thread.join().unwrap();
    assert_eq!(jvm().threads_attached(), 0);
}
