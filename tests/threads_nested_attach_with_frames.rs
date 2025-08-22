#![cfg(feature = "invocation")]

use std::{sync::Arc, thread::spawn};

use jni::JavaVM;

mod util;
use util::jvm;

/// Checks if nested attaches are working properly and threads detach themselves
/// on exit.
#[test]
fn nested_attach() {
    assert_eq!(jvm().threads_attached(), 0);
    let thread = spawn(|| {
        assert_eq!(jvm().threads_attached(), 0);
        check_nested_attach(jvm());
        assert_eq!(jvm().threads_attached(), 1);
    });
    thread.join().unwrap();
    assert_eq!(jvm().threads_attached(), 0);
}

/// Checks if nested `with_attached` calls does not detach the thread before the outer-most
/// call is finished.
fn check_nested_attach(vm: &Arc<JavaVM>) {
    check_detached(vm);

    vm.attach_current_thread(|_env| -> jni::errors::Result<()> {
        check_attached(vm);

        // Alias the outer _env so we avoid having more than one mutable JNIEnv in scope
        vm.attach_current_thread(|_env| -> jni::errors::Result<()> {
            check_attached(vm);
            Ok(())
        })
        .unwrap();

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
    vm.is_thread_attached()
        .expect("An unexpected JNI error occurred")
}
