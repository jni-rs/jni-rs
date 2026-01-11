#![cfg(feature = "invocation")]
//! Tests for `catch_unwind = false` behavior.
//!
//! This test file verifies the behavior difference between `catch_unwind =
//! false` and `catch_unwind = true` (the default).
//!
//! When `catch_unwind = false`:
//! - Native method implementations use `EnvUnowned::with_env_no_catch`
//! - Panics are NOT caught and converted to Java exceptions
//!
//! When `catch_unwind = true` (default):
//! - Native method wrappers use `EnvUnowned::with_env` (including catch_unwind)
//! - Panics are converted to Java exceptions via the error policy
//!
//! Testing `catch_unwind = false` is tricky because the native method
//! represents an extern "system" FFI boundary which panics cannot cross without
//! aborting the process. To work around this, we spawn a separate `cargo test`
//! process to run the code that can abort so we can check the exit status.

mod native_methods_utils;
mod util;

use jni::errors::Error;
use jni::sys::jint;
use jni::{Env, native_method};
use rusty_fork::rusty_fork_test;
use std::panic;

// ====================================================================================
// Test: catch_unwind = false (panics are NOT caught by error policy)
// ====================================================================================

// This test is designed to abort when run
native_method_test! {
    #[ignore = "This is run in a separate cargo process by test_catch_unwind_false_causes_abort"]
    test_name: test_catch_unwind_false_causes_abort_impl,
    java_class: "com/example/TestNativeCatchUnwind.java",
    methods: |class| &[
        native_method! {
            extern fn method_that_panics0() -> jint,
            fn = method_that_panics_no_catch,
            java_type = "com.example.TestNativeCatchUnwind",
            catch_unwind = false,  // This will cause abort when panic tries to escape
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        // Call the method - this will panic and abort because catch_unwind = false
        unsafe {
            let unowned = jni::EnvUnowned::from_raw(env.get_raw());
            let _ret = Java_com_example_TestNativeCatchUnwind_methodThatPanics0__(unowned, obj.as_raw());
        }

        // We should never reach here
        eprintln!("ERROR: Should have aborted!");
        std::process::exit(1);
        #[allow(unreachable_code)]
        Ok(())
    }
}

fn method_that_panics_no_catch<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
) -> Result<jint, Error> {
    panic!("This panic should cause abort because catch_unwind = false");
}

// Declare the exported symbol so we can call it from Rust
unsafe extern "system" {
    fn Java_com_example_TestNativeCatchUnwind_methodThatPanics0__(
        env: jni::EnvUnowned,
        this: jni::sys::jobject,
    ) -> jint;
}

#[test]
fn test_catch_unwind_false_causes_abort() {
    // Run the fixture test that has catch_unwind = false and panics.
    // The panic should cause the process to abort because it tries to
    // escape through an extern "system" FFI boundary that cannot unwind.

    // Use cargo test to run the specific fixture test
    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.arg("test")
        .arg("--features=invocation")
        .arg("--test")
        .arg("native_methods_catch_unwind")
        .arg("--")
        .arg("--include-ignored")
        .arg("--exact")
        .arg("test_catch_unwind_false_causes_abort_impl")
        .arg("--nocapture");

    let output = cmd.output().expect("Failed to execute fixture test");

    // The process should have failed (non-zero exit code)
    assert!(!output.status.success(), "Expected process to abort/fail");

    // Check the combined output (stderr and stdout) for the panic message
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("This panic should cause abort because catch_unwind = false"),
        "Expected panic message in output, got stdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // The test should have failed
    assert!(
        combined.contains("test test_catch_unwind_false_causes_abort_impl ... FAILED"),
        "Expected test failure in output, got: {}",
        combined
    );
}

// ====================================================================================
// Test: catch_unwind = true (default - panics ARE caught by error policy)
// ====================================================================================

native_method_test! {
    test_name: test_catch_unwind_true_catches_panic,
    java_class: "com/example/TestNativeCatchUnwind.java",
    methods: |class| &[
        native_method! {
            extern fn method_that_panics1() -> jint,
            fn = method_that_panics_with_catch,
            java_type = "com.example.TestNativeCatchUnwind",
            catch_unwind = true,  // Explicit, but this is the default
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        // Call the exported symbol directly from Rust with our own catch_unwind.
        // With catch_unwind = true (default), the panic should be caught by the
        // error policy inside the native method wrapper and converted to a Java exception.
        // Since we're calling from Rust (not Java), we won't see the Java exception,
        // but the important thing is that it doesn't panic.
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            let unowned = jni::EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeCatchUnwind_methodThatPanics1__(unowned, obj.as_raw())
        }));

        // The panic should have been caught by the error policy, so our catch_unwind
        // should NOT catch a panic (result should be Ok)
        assert!(result.is_ok(), "Expected panic to be caught by error policy when catch_unwind = true");

        Ok(())
    }
}

fn method_that_panics_with_catch<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
) -> Result<jint, Error> {
    panic!("This panic SHOULD be caught by error policy");
}

// Declare the exported symbol so we can call it from Rust
unsafe extern "system" {
    fn Java_com_example_TestNativeCatchUnwind_methodThatPanics1__(
        env: jni::EnvUnowned,
        this: jni::sys::jobject,
    ) -> jint;
}
