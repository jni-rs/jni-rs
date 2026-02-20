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

mod bind_native_methods_utils;
mod util;

use jni::sys::jint;
use jni::{Env, EnvUnowned, bind_java_type};
use rusty_fork::rusty_fork_test;
use std::panic;

// ====================================================================================
// Test: catch_unwind = false (panics are NOT caught by error policy)
// ====================================================================================

bind_java_type! {
    rust_type = TestCatchUnwindFalse,
    java_type = "com.example.TestNativeCatchUnwind",
    constructors { fn new() },
    native_methods {
        fn method_that_panics0 {
            sig = () -> jint,
            export = true,
            catch_unwind = false,  // This will cause abort when panic tries to escape
        },
    }
}

impl TestCatchUnwindFalseNativeInterface for TestCatchUnwindFalseAPI {
    type Error = jni::errors::Error;

    fn method_that_panics0<'local>(
        _env: &mut Env<'local>,
        _this: TestCatchUnwindFalse<'local>,
    ) -> Result<jint, Self::Error> {
        panic!("This panic should cause abort because catch_unwind = false");
    }
}

// This test is designed to abort when run
native_method_test! {
    #[ignore = "This is run in a separate cargo process by test_catch_unwind_false_causes_abort"]
    test_name: test_catch_unwind_false_causes_abort_impl,
    java_class: "com/example/TestNativeCatchUnwind.java",
    api: TestCatchUnwindFalseAPI,
    test_body: |env| {
        let obj = TestCatchUnwindFalse::new(env)?;

        // Call the method - this will panic and abort because catch_unwind = false
        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let _ret = Java_com_example_TestNativeCatchUnwind_methodThatPanics0__(unowned, obj);
        }

        // We should never reach here
        eprintln!("ERROR: Should have aborted!");
        std::process::exit(1);
        #[allow(unreachable_code)]
        Ok(())
    }
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
        .arg("bind_native_methods_catch_unwind")
        .arg("--")
        .arg("--include-ignored")
        .arg("--exact")
        .arg("test_catch_unwind_false_causes_abort_impl")
        .arg("--nocapture");

    let output = cmd.output().expect("Failed to execute fixture test");

    if output.status.success() {
        eprintln!("Expected the test to abort/fail, but it succeeded:");
        eprintln!("Ran: {:?}", cmd);
        eprintln!("------------------");
        eprintln!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
        eprintln!("------------------");
    }
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

bind_java_type! {
    rust_type = TestCatchUnwindTrue,
    java_type = "com.example.TestNativeCatchUnwind",
    constructors { fn new() },
    native_methods {
        fn method_that_panics1 {
            sig = () -> jint,
            catch_unwind = true,  // Explicit, but this is the default
        },
    }
}

impl TestCatchUnwindTrueNativeInterface for TestCatchUnwindTrueAPI {
    type Error = jni::errors::Error;

    fn method_that_panics1<'local>(
        _env: &mut Env<'local>,
        _this: TestCatchUnwindTrue<'local>,
    ) -> Result<jint, Self::Error> {
        panic!("This panic SHOULD be caught by error policy");
    }
}

native_method_test! {
    test_name: test_catch_unwind_true_catches_panic,
    java_class: "com/example/TestNativeCatchUnwind.java",
    api: TestCatchUnwindTrueAPI,
    expect_exception: "Rust panic: This panic SHOULD be caught by error policy",
    test_body: |env| {
        let obj = TestCatchUnwindTrue::new(env)?;

        // Call the exported symbol directly from Rust with our own catch_unwind.
        // With catch_unwind = true (default), the panic should be caught by the
        // error policy inside the native method wrapper and converted to a Java exception.
        // Since we're calling from Rust (not Java), we won't see the Java exception,
        // but the important thing is that it doesn't panic.
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeCatchUnwind_methodThatPanics1__(unowned, obj)
        }));

        // The panic should have been caught by the error policy, so our catch_unwind
        // should NOT catch a panic (result should be Ok)
        assert!(result.is_ok(), "Expected panic to be caught by error policy when catch_unwind = true");

        Ok(())
    }
}
