//! Tests for native method ABI checking functionality.
//!
//! These tests verify that the `abi_check` parameter correctly detects
//! mismatches between Java method declarations (static vs instance) and Rust registrations.

mod native_methods_utils;
mod util;

use jni::errors::Error;
use jni::objects::JClass;
use jni::sys::jint;
use jni::{Env, native_method};
use rusty_fork::rusty_fork_test;

// ====================================================================================
// Test: Instance method registered as static (Always mode)
// ====================================================================================

native_method_test! {
    test_name: test_abi_instance_registered_as_static_wrapped,
    java_class: "com/example/TestNativeAbiCheck.java",
    methods: |class| &[
        native_method! {
            static fn native_method(value: jint) -> jint,
            fn = native_method_registered_as_static,
            abi_check = Always,
        },
    ],
    expect_exception: "ABI check detected instance method called with static registration",
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let _result = call_int_method!(env, &obj, "callMethod", 42)?;
        Ok(())
    }
}

fn native_method_registered_as_static<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>, // Static registration expects JClass
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 2)
}

// ====================================================================================
// Test: Static method registered as instance (Always mode)
// ====================================================================================

native_method_test! {
    test_name: test_abi_static_registered_as_instance_wrapped,
    java_class: "com/example/TestNativeAbiCheck.java",
    methods: |class| &[
        native_method! {
            fn native_static_method(value: jint) -> jint,
            fn = native_static_method_registered_as_instance,
            abi_check = Always,
        },
    ],
    expect_exception: "ABI check detected static method called with instance registration",
    test_body: |env, class| {
        let _result = call_static_int_method!(env, class, "callStaticMethod", 42)?;
        Ok(())
    }
}

fn native_static_method_registered_as_instance<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>, // Instance registration expects JObject
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 3)
}

// ====================================================================================
// Test: Correct instance and static methods work (Always mode)
// ====================================================================================

native_method_test! {
    test_name: test_abi_correct_registration_wrapped,
    java_class: "com/example/TestNativeAbiCheck.java",
    methods: |class| &[
        native_method! {
            fn native_method(value: jint) -> jint,
            fn = native_method_correct,
            abi_check = Always,
        },
        native_method! {
            static fn native_static_method(value: jint) -> jint,
            fn = native_static_method_correct,
            abi_check = Always,
        },
    ],
    test_body: |env, class| {
        // Correctly declared instance method
        let obj = new_object!(env, class)?;
        let result = call_int_method!(env, &obj, "callMethod", 5)?;
        assert_eq!(result, 10);

        // Correctly declared static method
        let result = call_static_int_method!(env, class, "callStaticMethod", 7)?;
        assert_eq!(result, 21);

        Ok(())
    }
}

fn native_method_correct<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 2)
}

fn native_static_method_correct<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 3)
}

// ====================================================================================
// Test: Per-method ABI check override (UnsafeNever disables check)
// ====================================================================================

native_method_test! {
    test_name: test_abi_unsafe_never_wrapped,
    java_class: "com/example/TestNativeAbiCheck.java",
    methods: |class| &[
        native_method! {
            // Incorrectly registered as an instance method
            fn native_static_method(value: jint) -> jint,
            fn = native_static_method_registered_as_instance,
            abi_check = UnsafeNever,  // But check is disabled
        },
    ],
    test_body: |env, class| {
        // Call static method - should NOT panic because check is disabled
        // Note: This is unsafe and wrong, but we're testing that the check is disabled
        let result = call_static_int_method!(env, class, "callStaticMethod", 42)?;
        assert_eq!(result, 126);
        Ok(())
    }
}

// ====================================================================================
// Test: UnsafeDebugOnly mode (checks only in debug builds)
// ====================================================================================

#[cfg(debug_assertions)]
native_method_test! {
    test_name: test_abi_debug_only_in_debug_mode_wrapped,
    java_class: "com/example/TestNativeAbiCheck.java",
    methods: |class| &[
        native_method! {
            // Incorrectly registered as a static method
            static fn native_method(value: jint) -> jint,
            fn = native_method_registered_as_static,
            abi_check = UnsafeDebugOnly,
        },
    ],
    expect_exception: "UnsafeDebugOnly correctly detected ABI mismatch in debug build",
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let _result = call_int_method!(env, &obj, "callMethod", 42)?;
        Ok(())
    }
}

#[cfg(not(debug_assertions))]
native_method_test! {
    test_name: test_abi_debug_only_in_release_mode_wrapped,
    java_class: "com/example/TestNativeAbiCheck.java",
    methods: |class| &[
        native_method! {
            // Incorrectly registered as a static method
            static fn native_method(value: jint) -> jint,
            fn = native_method_registered_as_static,
            abi_check = UnsafeDebugOnly,  // But disabled in release builds
        },
    ],
    test_body: |env, class| {
        // Try to call instance method - should NOT panic in release mode
        let obj = new_object!(env, class)?;
        let result = call_int_method!(env, &obj, "callMethod", 42)?;
        assert_eq!(result, 84);
        Ok(())
    }
}

// ====================================================================================
// Test: UnsafeDebugOnly mode (checks only in debug builds, raw)
// ====================================================================================

// This fixture test is designed to abort when run in debug mode
#[cfg(debug_assertions)]
native_method_test! {
    #[ignore = "This is run in a separate cargo process by test_abi_debug_only_in_debug_mode_raw"]
    test_name: test_abi_debug_only_in_debug_mode_raw_impl,
    java_class: "com/example/TestNativeAbiCheck.java",
    methods: |class| &[
        native_method! {
            // Incorrectly registered as a static method
            static raw fn native_method(value: jint) -> jint,
            fn = raw_native_method_registered_as_static,
            abi_check = UnsafeDebugOnly,
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        // This will trigger ABI check and abort because it's a raw function
        let _result = call_int_method!(env, &obj, "callMethod", 42)?;

        // We should never reach here
        eprintln!("ERROR: Should have aborted!");
        std::process::exit(1);
        #[allow(unreachable_code)]
        Ok(())
    }
}

#[cfg(debug_assertions)]
#[test]
fn test_abi_debug_only_in_debug_mode_raw() {
    // Run the fixture test that has UnsafeDebugOnly ABI check on a raw function.
    // The ABI check should detect the mismatch and abort the process in debug builds.

    // Use cargo test to run the specific fixture test
    let mut cmd = std::process::Command::new(env!("CARGO"));
    cmd.arg("test")
        .arg("--test")
        .arg("native_methods_abi_check")
        .arg("--")
        .arg("--include-ignored")
        .arg("--exact")
        .arg("test_abi_debug_only_in_debug_mode_raw_impl")
        .arg("--nocapture");

    let output = cmd.output().expect("Failed to execute fixture test");

    // The process should have failed (non-zero exit code)
    assert!(
        !output.status.success(),
        "Expected process to abort/fail due to ABI check"
    );

    // Check the combined output (stderr and stdout) for the ABI check message
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);

    // The ABI check assertion message should appear in the output
    assert!(
        combined.contains("was registered as static but called as instance method")
            || combined.contains("nativeMethod"),
        "Expected ABI check error message in output, got stdout: {}\nstderr: {}",
        stdout,
        stderr
    );

    // The test should have failed
    assert!(
        combined.contains("test test_abi_debug_only_in_debug_mode_raw_impl ... FAILED"),
        "Expected test failure in output, got: {}",
        combined
    );
}

#[cfg(not(debug_assertions))]
native_method_test! {
    test_name: test_abi_debug_only_in_release_mode_raw,
    java_class: "com/example/TestNativeAbiCheck.java",
    methods: |class| &[
        native_method! {
            name = "nativeMethod",
            sig = (value: jint) -> jint,
            fn = raw_native_method_registered_as_static,
            static = true,  // Mismatched: Rust says static, Java says instance
            raw = true,
            abi_check = UnsafeDebugOnly,  // But disabled in release builds
        },
    ],
    test_body: |env, class| {
        // Try to call instance method - should NOT panic in release mode
        let obj = new_object!(env, class)?;
        let result = call_int_method!(env, &obj, "callMethod", 42)?;
        assert_eq!(result, 84);
        Ok(())
    }
}

fn raw_native_method_registered_as_static<'local>(
    _env: jni::EnvUnowned<'local>,
    _class: JClass<'local>,
    value: jint,
) -> jint {
    value * 2
}
