#![cfg(feature = "invocation")]

mod util;

use rusty_fork::rusty_fork_test;

rusty_fork_test! {
#[test]
fn test_exception_unsafe_calls() {
    let jvm = util::jvm();
    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let _ = env.throw("Test Exception".to_string());

        let res = env.new_string("Test");

        println!("new_string res = {:?}", res);
        // No new string should be allocated because there is a pending exception
        // and until the exception is cleared, the JNI calls would lead to
        // undefined behavior
        assert!(matches!(res, Err(jni::errors::Error::JavaException)));

        let pending = env.with_local_frame(8, |env| -> jni::errors::Result<bool> {
            Ok(env.exception_check())
        }).expect("Push/PopLocalFrame + ExceptionCheck should be exception safe");

        assert!(pending);
        Ok(())
    }).unwrap();
}
}
