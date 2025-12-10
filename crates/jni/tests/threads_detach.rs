#![cfg(feature = "invocation")]

mod util;
use jni::{JavaVM, errors::Error};
use std::thread::spawn;
use util::{
    attach_current_thread, attach_current_thread_for_scope, call_java_abs, detach_current_thread,
    jvm,
};

use rusty_fork::rusty_fork_test;

rusty_fork_test! {
#[test]
fn thread_detaches_for_tls_attachment_when_finished() {
    let thread = spawn(|| {
        attach_current_thread(|env| {
            let val = call_java_abs(env, -2);
            assert_eq!(val, 2);
            assert_eq!(jvm().threads_attached(), 1);

            Ok(())
        }).unwrap();
    });

    thread.join().unwrap();
    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}

rusty_fork_test! {
#[test]
fn thread_detaches_for_scoped_attachment() {
    // A newly created VM will be attached
    assert_eq!(jvm().threads_attached(), 1);
    jvm().detach_current_thread().unwrap();
    assert_eq!(jvm().threads_attached(), 0);

    attach_current_thread_for_scope(|env| {
        assert_eq!(jvm().threads_attached(), 1);
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        Ok(())
    }).unwrap();
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
    // A newly created VM will be attached
    assert_eq!(jvm().threads_attached(), 1);
    jvm().detach_current_thread().unwrap();
    assert_eq!(jvm().threads_attached(), 0);

    attach_current_thread_for_scope(|env| {
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);

        // It's not safe to detach a thread while there's an AttachGuard in-use
        assert!(matches!(detach_current_thread(), Err(Error::ThreadAttachmentGuarded)));

        assert_eq!(jvm().threads_attached(), 1);
        assert!(util::is_thread_attached());

        Ok(())
    }).unwrap();

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
    // A newly created VM will be attached
    assert_eq!(jvm().threads_attached(), 1);
    jvm().detach_current_thread().unwrap();
    assert_eq!(jvm().threads_attached(), 0);

    attach_current_thread(|env| {
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);

        // It's not safe to detach a thread while there's an AttachGuard in-use
        assert!(matches!(detach_current_thread(), Err(Error::ThreadAttachmentGuarded)));

        assert_eq!(jvm().threads_attached(), 1);
        assert!(JavaVM::thread_attach_guard_level() > 0);

        Ok(())
    }).unwrap();
    assert_eq!(jvm().threads_attached(), 1);
    assert!(util::is_thread_attached());

    assert!(detach_current_thread().is_ok());

    assert_eq!(jvm().threads_attached(), 0);
    assert_eq!(JavaVM::thread_attach_guard_level(), 0);
    assert!(!util::is_thread_attached());

    assert!(detach_current_thread().is_ok());

    assert_eq!(JavaVM::thread_attach_guard_level(), 0);
    assert!(!util::is_thread_attached());
}
}

rusty_fork_test! {
#[test]
fn threads_scoped_attachments_nest() {
    // A newly created VM will be attached
    assert_eq!(jvm().threads_attached(), 1);
    jvm().detach_current_thread().unwrap();
    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());

    attach_current_thread_for_scope(|env| {
        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);

        // Create another scoped attachment, which should re-use the existing
        // attachment and not detach the thread when dropped
        attach_current_thread_for_scope(|env| {
            let val = call_java_abs(env, -2);
            assert_eq!(val, 2);
            assert_eq!(jvm().threads_attached(), 1);

            Ok(())
        }).unwrap();

        assert_eq!(jvm().threads_attached(), 1);
        assert!(util::is_thread_attached());

        // Call a Java method after nested guard has been dropped to double
        // check that this thread has not been detached.
        let val = call_java_abs(env, -3);
        assert_eq!(val, 3);

        Ok(())
        // Outer attachment should detach the thread here
    }).unwrap();

    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}

rusty_fork_test! {
#[test]
fn threads_scope_attachments_block_tls_attachments() {
    // A newly created VM will be attached
    assert_eq!(jvm().threads_attached(), 1);
    jvm().detach_current_thread().unwrap();
    assert_eq!(jvm().threads_attached(), 0);

    attach_current_thread_for_scope(|_| {

        attach_current_thread_for_scope(|env| {
            let val = call_java_abs(env, -1);
            assert_eq!(val, 1);
            assert_eq!(jvm().threads_attached(), 1);
            Ok(())
        }).unwrap();

        // Request a permanent AttachGuard, which should re-use the existing
        // attachment and should therefore not register a permanent TLS
        // attachment guard
        attach_current_thread(|env| {
            let val = call_java_abs(env, -4);
            assert_eq!(val, 4);
            assert_eq!(jvm().threads_attached(), 1);
            Ok(())
        }).unwrap();

        assert_eq!(jvm().threads_attached(), 1);

        Ok(())
    }).unwrap();

    // Check that after guard is dropped the thread is properly detached
    // despite the request to create a nested permanent attachment.
    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}

rusty_fork_test! {
#[test]
fn threads_guard_nesting_blocks_explicit_detachment() {
    // A newly created VM will be attached
    assert_eq!(jvm().threads_attached(), 1);
    // There are initially no AttachGuards in place
    assert_eq!(JavaVM::thread_attach_guard_level(), 0);
    jvm().detach_current_thread().unwrap();
    assert_eq!(jvm().threads_attached(), 0);

    // While there are no guards and no attachment then `detach_current_thread`
    // is a no-op that shouldn't return an error.
    assert!(jvm().detach_current_thread().is_ok());

    attach_current_thread(|env| {

        let val = call_java_abs(env, -1);
        assert_eq!(val, 1);
        assert_eq!(jvm().threads_attached(), 1);
        let guard_level_1 = JavaVM::thread_attach_guard_level();
        assert!(guard_level_1 > 0);

        // `detach_current_thread()` should fail while there are guards
        assert!(jvm().detach_current_thread().is_err());

        attach_current_thread(|env| {
            let val = call_java_abs(env, -2);
            assert_eq!(val, 2);
            assert_eq!(jvm().threads_attached(), 1);
            let guard_level_2 = JavaVM::thread_attach_guard_level();
            assert!(guard_level_2 > guard_level_1);
            Ok(())
        }).unwrap();

        assert_eq!(jvm().threads_attached(), 1);
        assert_eq!(JavaVM::thread_attach_guard_level(), guard_level_1);
        // `detach_current_thread()` should fail while there are guards
        assert!(jvm().detach_current_thread().is_err());

        // Call a Java method after nested guard has been dropped to double
        // check that this thread has not been detached.
        let val = call_java_abs(env, -3);
        assert_eq!(val, 3);

        // Create nested attachment
        attach_current_thread_for_scope(|env| {
            let val = call_java_abs(env, -2);
            assert_eq!(val, 2);
            assert_eq!(jvm().threads_attached(), 1);
            let guard_level_2 = JavaVM::thread_attach_guard_level();
            assert!(guard_level_2 > guard_level_1);
            Ok(())
        }).unwrap();

        assert_eq!(jvm().threads_attached(), 1);
        assert_eq!(JavaVM::thread_attach_guard_level(), guard_level_1);
        // `detach_current_thread()` should fail while there are guards
        assert!(jvm().detach_current_thread().is_err());

        // Call a Java method after nested guard has been dropped to double
        // check that this thread has not been detached.
        let val = call_java_abs(env, -3);
        assert_eq!(val, 3);

        // `detach_current_thread()` should fail while there are guards
        assert!(jvm().detach_current_thread().is_err());

        Ok(())
    }).unwrap();

    // Since we requested a permanent attachment then dropping the guard
    // shouldn't detach the thread
    assert_eq!(jvm().threads_attached(), 1);

    // At this point we have no `AttachGuard` and the thread would normally
    // detach automatically when the internal `TLSAttachGuard` is dropped.
    assert_eq!(JavaVM::thread_attach_guard_level(), 0);

    // Should be OK to detach a thread before it terminates so long as there are
    // no active AttachGuards...
    jvm().detach_current_thread().unwrap();

    assert_eq!(jvm().threads_attached(), 0);
    assert!(!util::is_thread_attached());
}
}
