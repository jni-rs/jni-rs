#![cfg(feature = "invocation")]

use std::{sync::Arc, thread::spawn};

use jni::{
    errors::{Error, JniError},
    Executor, JNIVersion, JavaVM,
};

mod util;
use util::jvm;

/// Checks if nested attaches are working properly and threads detach themselves
/// on exit.
#[test]
fn nested_attach() {
    let executor = Executor::new(jvm().clone());

    assert_eq!(jvm().threads_attached(), 0);
    let thread = spawn(|| {
        assert_eq!(jvm().threads_attached(), 0);
        check_nested_attach(jvm(), executor);
        assert_eq!(jvm().threads_attached(), 1);
    });
    thread.join().unwrap();
    assert_eq!(jvm().threads_attached(), 0);
}

/// Checks if nested `with_attached` calls does not detach the thread before the outer-most
/// call is finished.
fn check_nested_attach(vm: &Arc<JavaVM>, executor: Executor) {
    check_detached(vm);
    executor
        .with_attached::<_, _, Error>(|_| {
            check_attached(vm);
            executor.with_attached::<_, _, Error>(|_| {
                check_attached(vm);
                Ok(())
            })?;
            check_attached(vm);
            Ok(())
        })
        .unwrap();
}

fn check_attached(vm: &JavaVM) {
    assert!(is_attached(vm));
}

fn check_detached(vm: &JavaVM) {
    assert!(!is_attached(vm));
}

fn is_attached(vm: &JavaVM) -> bool {
    // Safety: assumes tests are only run against a JavaVM that implements JNI >= 1.4
    unsafe { vm.get_env(JNIVersion::V1_4) }
        .map(|_| true)
        .or_else(|jni_err| match jni_err {
            Error::JniCall(JniError::ThreadDetached) => Ok(false),
            _ => Err(jni_err),
        })
        .expect("An unexpected JNI error occurred")
}
