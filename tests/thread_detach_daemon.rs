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

#[test]
fn thread_detach() {
    // Daemon threads detach when finished
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
