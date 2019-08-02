#![cfg(feature = "invocation")]

use std::thread::spawn;

mod util;
use util::{attach_current_thread_as_daemon, call_java_abs, jvm};

#[test]
fn daemon_thread_detaches_when_finished() {
    let thread = spawn(|| {
        let env = attach_current_thread_as_daemon();
        let val = call_java_abs(&env, -3);
        assert_eq!(val, 3);
        assert_eq!(jvm().threads_attached(), 1);
    });

    thread.join().unwrap();
    assert_eq!(jvm().threads_attached(), 0);
}
