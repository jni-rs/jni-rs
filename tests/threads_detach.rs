#![cfg(feature = "invocation")]

mod util;
use jni::errors::Error;
use util::{attach_current_thread, attach_current_thread_for_scope, call_java_abs, detach_current_thread, jvm};
use std::thread::spawn;

use rusty_fork::rusty_fork_test;

rusty_fork_test! {
#[test]
fn thread_detaches_for_tls_attachment_when_finished() {
    let thread = spawn(|| {
        let mut guard = unsafe { attach_current_thread() };
        let env = guard.current_frame_env();
        let val = call_java_abs(env, -2);
        assert_eq!(val, 2);
        assert_eq!(jvm().threads_attached(), 1);
    });

    thread.join().unwrap();
    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}


rusty_fork_test! {
#[test]
fn thread_detaches_for_scoped_attachment() {
    assert_eq!(jvm().threads_attached(), 0);
    {
        let mut guard = unsafe { attach_current_thread_for_scope() };
        let env = guard.current_frame_env();
        assert_eq!(jvm().threads_attached(), 1);
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
    }
    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}


rusty_fork_test! {
// TODO: check nested scope guards and that we don't register TLS guards if already attached
#[test]
fn thread_detaches_for_outer_scoped_attachment() {

}
}


rusty_fork_test! {
#[test]
fn threads_explicit_detach_error_for_scoped_attachment() {
    assert_eq!(jvm().threads_attached(), 0);
    {
        let mut guard = unsafe { attach_current_thread_for_scope() };
        let env = guard.current_frame_env();
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);

        // It's not safe to detach a thread while there's an AttachGuard in-use
        assert!(matches!(detach_current_thread(), Err(Error::ThreadAttachmentGuarded)));

        assert_eq!(jvm().threads_attached(), 1);
        assert!(util::is_thread_attached());

        // AttachGuard Drop
    }

    // Double check something didn't go wrong after dropping the guard
    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());

    // It should be a no-op to try and detach the thread in this case
    assert!(detach_current_thread().is_ok());
}
}


rusty_fork_test! {
#[test]
fn threads_explicit_detach_tls_attachment() {
    assert_eq!(jvm().threads_attached(), 0);

    {
        let mut guard = unsafe { attach_current_thread() };
        let env = guard.current_frame_env();
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);

        // It's not safe to detach a thread while there's an AttachGuard in-use
        assert!(matches!(detach_current_thread(), Err(Error::ThreadAttachmentGuarded)));

        assert_eq!(jvm().threads_attached(), 1);
        assert_eq!(jvm().thread_attach_guard_level(), 1);

        // AttachGuard Drop
    }
    assert_eq!(jvm().threads_attached(), 1);
    assert!(util::is_thread_attached());

    assert!(detach_current_thread().is_ok());

    assert_eq!(jvm().threads_attached(), 0);
    assert_eq!(jvm().thread_attach_guard_level(), 0);
    assert!(!util::is_thread_attached());

    assert!(detach_current_thread().is_ok());

    assert_eq!(jvm().thread_attach_guard_level(), 0);
    assert!(!util::is_thread_attached());
}
}


rusty_fork_test! {
#[test]
fn threads_scoped_attachments_nest() {
    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());

    // Ensure the `AttachGuard` `env` we get from the attachment will be dropped
    // and won't be visible for the final attachment check via `jvm().get_env()`
    {
        // Safety: there is no other `AttachGuard` or mutable `JNIEnv` in scope,
        // so we aren't creating an opportunity for local references to be
        // created in association with the wrong stack frame.
        let mut guard = unsafe { attach_current_thread_for_scope() };
        let env = guard.current_frame_env();
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);

        // Create another scoped AttachGuard, which should re-use the existing
        // attachment and not detach the thread when dropped
        fn create_nested_scope_attachment() {
            // Safety: there is no other mutable `JNIEnv` in scope, so we aren't
            // creating an opportunity for local references to be created
            // in association with the wrong stack frame.
            let mut guard = unsafe { attach_current_thread_for_scope() };
            let env = guard.current_frame_env();
            let val = call_java_abs(env, -2);
            assert_eq!(val, 2);
            assert_eq!(jvm().threads_attached(), 1);
        }
        // We use an inner function to ensure the `env` in this scope is not visible
        // to the code that makes an `unsafe` attachment to materialise a new `JNIEnv`.
        create_nested_scope_attachment();

        assert_eq!(jvm().threads_attached(), 1);
        assert!(util::is_thread_attached());

        // Call a Java method after nested guard has been dropped to double
        // check that this thread has not been detached.
        let val = call_java_abs(env, -3);
        assert_eq!(val, 3);

        // Drop of outer `guard` should detach the thread here
    }

    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}


rusty_fork_test! {
#[test]
fn threads_scope_attachments_block_tls_attachments() {
    assert_eq!(jvm().threads_attached(), 0);

    // Ensure the `AttachGuard` `env` we get from the attachment will be dropped
    // and won't be visible for the final attachment check via `jvm().get_env()`
    {
        // Safety: there is no other mutable `JNIEnv` in scope, so we aren't
        // creating an opportunity for local references to be created
        // in association with the wrong stack frame.
        let mut guard = unsafe { attach_current_thread_for_scope() };
        let env = guard.current_frame_env();
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);

        // Request a permanent AttachGuard, which should re-use the existing
        // attachment and should therefore not register a permanent TLS
        // attachment guard
        fn create_nested_tls_attachment() {
            // Safety: there is no other mutable `JNIEnv` in scope, so we aren't
            // creating an opportunity for local references to be created
            // in association with the wrong stack frame.
            let mut guard = unsafe { attach_current_thread() };
            let env= guard.current_frame_env();
            let val = call_java_abs(env, -4);
            assert_eq!(val, 4);
            assert_eq!(jvm().threads_attached(), 1);
        }
        // We use an inner function to ensure the `guard` in this scope is not visible
        // to the code that makes an `unsafe` attachment to materialise a new `JNIEnv`.
        create_nested_tls_attachment();
        assert_eq!(jvm().threads_attached(), 1);
    }

    // Check that after guard is dropped the thread is properly detached
    // despite the request to create a nested permanent attachment.
    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}


rusty_fork_test! {
#[test]
fn threads_guard_nesting_blocks_explicit_detachment() {
    assert_eq!(jvm().threads_attached(), 0);
    assert_eq!(jvm().thread_attach_guard_level(), 0);

    // While there are no guards and no attachment then `detach_current_thread`
    // is a no-op that shouldn't return an error.
    assert!(jvm().detach_current_thread().is_ok());

    {
        // Safety: there is no other mutable `JNIEnv` in scope, so we aren't
        // creating an opportunity for local references to be created
        // in association with the wrong stack frame.
        let mut guard = unsafe { attach_current_thread() };
        let env = guard.current_frame_env();
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);
        assert_eq!(jvm().thread_attach_guard_level(), 1);

        // `detach_current_thread()` should fail while there are guards
        assert!(jvm().detach_current_thread().is_err());

        // Create nested AttachGuard.
        fn create_nested_attachment() {
            // Safety: there is no other mutable `JNIEnv` in scope, so we aren't
            // creating an opportunity for local references to be created
            // in association with the wrong stack frame.
            let mut guard = unsafe { attach_current_thread() };
            let env = guard.current_frame_env();
            let val = call_java_abs(env, -2);
            assert_eq!(val, 2);
            assert_eq!(jvm().threads_attached(), 1);
            assert_eq!(jvm().thread_attach_guard_level(), 2);
        }
        // We use an inner function to ensure the `env` in this scope is not visible
        // to the code that makes an `unsafe` attachment to materialise a new `JNIEnv`.
        create_nested_attachment();

        assert_eq!(jvm().threads_attached(), 1);
        assert_eq!(jvm().thread_attach_guard_level(), 1);
        // `detach_current_thread()` should fail while there are guards
        assert!(jvm().detach_current_thread().is_err());

        // Call a Java method after nested guard has been dropped to double
        // check that this thread has not been detached.
        let val = call_java_abs(env, -3);
        assert_eq!(val, 3);

        // Create nested AttachGuard.
        fn create_nested_attachment_for_scope() {
            // Safety: there is no other mutable `JNIEnv` in scope, so we aren't
            // creating an opportunity for local references to be created
            // in association with the wrong stack frame.
            let mut guard = unsafe { attach_current_thread_for_scope() };
            let env = guard.current_frame_env();
            let val = call_java_abs(env, -2);
            assert_eq!(val, 2);
            assert_eq!(jvm().threads_attached(), 1);
            assert_eq!(jvm().thread_attach_guard_level(), 2);
        }
        // We use an inner function to ensure the `env` in this scope is not visible
        // to the code that makes an `unsafe` attachment to materialise a new `JNIEnv`.
        create_nested_attachment_for_scope();

        assert_eq!(jvm().threads_attached(), 1);
        assert_eq!(jvm().thread_attach_guard_level(), 1);
        // `detach_current_thread()` should fail while there are guards
        assert!(jvm().detach_current_thread().is_err());

        // Call a Java method after nested guard has been dropped to double
        // check that this thread has not been detached.
        let val = call_java_abs(env, -3);
        assert_eq!(val, 3);

        // `detach_current_thread()` should fail while there are guards
        assert!(jvm().detach_current_thread().is_err());
    }

    // Since we requested a permanent attachment then dropping the guard
    // shouldn't detach the thread
    assert_eq!(jvm().threads_attached(), 1);

    // At this point we have no `AttachGuard` and the thread would normally
    // detach automatically when the internal `TLSAttachGuard` is dropped.
    assert_eq!(jvm().thread_attach_guard_level(), 0);

    // Should be OK to detach a thread before it terminates so long as there are
    // no active AttachGuards...
    jvm().detach_current_thread().unwrap();

    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}