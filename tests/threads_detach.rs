#![cfg(feature = "invocation")]
extern crate jni;
extern crate error_chain;

use std::thread::spawn;

mod util;
use util::{attach_current_thread_permanently, jvm, call_java_abs};

#[test]
fn thread_detaches_when_finished() {
    let thread = spawn(|| {
        let env = attach_current_thread_permanently();
        let val = call_java_abs(&env, -2);
        assert_eq!(val, 2);
        assert_eq!(jvm().threads_attached(), 1);
    });

    thread.join().unwrap();
    assert_eq!(jvm().threads_attached(), 0);
}
