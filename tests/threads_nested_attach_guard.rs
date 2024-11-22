#![cfg(feature = "invocation")]

use assert_matches::assert_matches;

mod util;
use jni::{
    errors::{Error, JniError},
    JNIVersion,
};
use util::{attach_current_thread, call_java_abs, jvm};

#[test]
pub fn nested_attaches_should_not_detach_guarded_thread() {
    assert_eq!(jvm().threads_attached(), 0);
    let mut env = attach_current_thread();
    let val = call_java_abs(&mut env, -1);
    assert_eq!(val, 1);
    assert_eq!(jvm().threads_attached(), 1);

    // Can't create nested AttachGuard.
    assert_matches!(
        jvm().attach_current_thread(),
        Err(Error::JniCall(JniError::ThreadAlreadyAttached))
    );

    // Call a Java method after nested attach attempt has failed to check that
    // this thread has not been detached.
    let val = call_java_abs(&mut env, -3);
    assert_eq!(val, 3);
    assert_eq!(jvm().threads_attached(), 1);

    // Can't create nested attach_permanently.
    assert_matches!(
        jvm().attach_current_thread_permanently(),
        Err(Error::JniCall(JniError::ThreadAlreadyAttached))
    );

    // Call a Java method after nested attach attempt has failed to check that
    // this thread has not been detached.
    let val = call_java_abs(&mut env, -4);
    assert_eq!(val, 4);
    assert_eq!(jvm().threads_attached(), 1);

    // Can't create nested attach_as_daemon.
    assert_matches!(
        unsafe { jvm().attach_current_thread_as_daemon() },
        Err(Error::JniCall(JniError::ThreadAlreadyAttached))
    );

    // Call a Java method after nested attach attempt has failed to check that
    // this thread has not been detached.
    let val = call_java_abs(&mut env, -5);
    assert_eq!(val, 5);
    assert_eq!(jvm().threads_attached(), 1);

    // Check that after guard is dropped the thread is properly detached
    // despite attempts at nested "permanent" attaches.
    drop(env);
    assert_eq!(jvm().threads_attached(), 0);
    assert_matches!(
        unsafe { jvm().get_env(JNIVersion::V1_4) },
        Err(Error::JniCall(JniError::ThreadDetached))
    );
}
