//! Tests for native method ABI checking functionality.
//!
//! These tests verify that the `native_methods_abi_check` attribute correctly detects
//! mismatches between Java method declarations (static vs instance) and Rust registrations.
//!
//! Note: These tests use shim `callXyz` methods in Java to invoke the native methods,
//! instead of declaring the native methods with `pub` in order to call them directly.
//! This is because the generation of `pub` methods would also catch ABI mismatches
//! at runtime when `get_method_id` or `get_static_method_id` is called, which would
//! prevent testing the ABI check functionality.

mod bind_native_methods_utils;
mod util;

use jni::objects::JClass;
use jni::sys::jint;
use jni::{Env, bind_java_type};
use rusty_fork::rusty_fork_test;

// ====================================================================================
// Test: Instance method registered as static (Always mode)
// ====================================================================================

bind_java_type! {
    rust_type = TestAbiInstanceAsStatic,
    java_type = "com.example.TestNativeAbiCheck",
    abi_check = Always,
    native_methods_export = false,
    constructors { fn new() },
    methods { fn call_method(value: jint) -> jint },
    native_methods {
        static fn native_method(value: jint) -> jint,
    }
}

impl TestAbiInstanceAsStaticNativeInterface for TestAbiInstanceAsStaticAPI {
    type Error = jni::errors::Error;

    fn native_method<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 2)
    }
}

native_method_test! {
    test_name: test_abi_instance_registered_as_static,
    java_class: "com/example/TestNativeAbiCheck.java",
    api: TestAbiInstanceAsStaticAPI,
    expect_exception: "ABI check detected instance method called with static registration",
    test_body: |env| {
        let obj = TestAbiInstanceAsStatic::new(env)?;
        // Java declares as instance, but we registered as static -> should panic
        let _result = obj.call_method(env, 42)?;
        Ok(())
    }
}

// ====================================================================================
// Test: Static method registered as instance (Always mode)
// ====================================================================================

bind_java_type! {
    rust_type = TestAbiStaticAsInstance,
    java_type = "com.example.TestNativeAbiCheck",
    abi_check = Always,
    native_methods_export = false,
    methods { static fn call_static_method(value: jint) -> jint },
    native_methods {
        fn native_static_method(value: jint) -> jint,
    }
}

impl TestAbiStaticAsInstanceNativeInterface for TestAbiStaticAsInstanceAPI {
    type Error = jni::errors::Error;

    fn native_static_method<'local>(
        _env: &mut Env<'local>,
        _this: TestAbiStaticAsInstance<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 3)
    }
}

native_method_test! {
    test_name: test_abi_static_registered_as_instance,
    java_class: "com/example/TestNativeAbiCheck.java",
    api: TestAbiStaticAsInstanceAPI,
    expect_exception: "ABI check detected static method called with instance registration",
    test_body: |env| {
        // Java declares as static, but we registered as instance -> should panic
        let _result = TestAbiStaticAsInstance::call_static_method(env, 42)?;
        Ok(())
    }
}

// ====================================================================================
// Test: Correct instance and static methods work (Always mode)
// ====================================================================================

bind_java_type! {
    rust_type = TestAbiCorrect,
    java_type = "com.example.TestNativeAbiCheck",
    abi_check = Always,
    native_methods_export = false,
    constructors { fn new() },
    methods {
        fn call_method(value: jint) -> jint,
        static fn call_static_method(value: jint) -> jint,
    },
    native_methods {
        fn native_method(value: jint) -> jint,
        static fn native_static_method(value: jint) -> jint,
    }
}

impl TestAbiCorrectNativeInterface for TestAbiCorrectAPI {
    type Error = jni::errors::Error;

    fn native_method<'local>(
        _env: &mut Env<'local>,
        _this: TestAbiCorrect<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 10)
    }

    fn native_static_method<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 20)
    }
}

native_method_test! {
    test_name: test_abi_correct_methods_work,
    java_class: "com/example/TestNativeAbiCheck.java",
    api: TestAbiCorrectAPI,
    test_body: |env| {
        let obj = TestAbiCorrect::new(env)?;

        // Correctly declared instance method
        let result = obj.call_method(env, 5)?;
        assert_eq!(result, 50);

        // Correctly declared static method
        let result = TestAbiCorrect::call_static_method(env, 7)?;
        assert_eq!(result, 140);

        Ok(())
    }
}

// ====================================================================================
// Test: Per-method ABI check override (Always overrides UnsafeNever default)
// ====================================================================================

bind_java_type! {
    rust_type = TestAbiPerMethodAlways,
    java_type = "com.example.TestNativeAbiCheck",
    abi_check = UnsafeNever,  // Default: no checks
    native_methods_export = false,
    constructors { fn new() },
    methods { fn call_method(value: jint) -> jint },
    native_methods {
        // Incorrectly registered as static
        // Override: Always check this one even though default is UnsafeNever
        static fn native_method {
            sig = (value: jint) -> jint,
            abi_check = Always,
        },
    }
}

impl TestAbiPerMethodAlwaysNativeInterface for TestAbiPerMethodAlwaysAPI {
    type Error = jni::errors::Error;

    fn native_method<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 2)
    }
}

native_method_test! {
    test_name: test_abi_per_method_override_always,
    java_class: "com/example/TestNativeAbiCheck.java",
    api: TestAbiPerMethodAlwaysAPI,
    expect_exception: "Per-method abi_check=Always override correctly detected ABI mismatch",
    test_body: |env| {
        let obj = TestAbiPerMethodAlways::new(env)?;
        // Should panic because we set abi_check = Always for this method
        let _result = obj.call_method(env, 42)?;
        Ok(())
    }
}

// ====================================================================================
// Test: Per-method ABI check override (UnsafeNever disables check)
// ====================================================================================

bind_java_type! {
    rust_type = TestAbiPerMethodNever,
    java_type = "com.example.TestNativeAbiCheck",
    abi_check = UnsafeNever,  // Disable ABI checks by default
    native_methods_export = false,
    methods { static fn call_static_method(value: jint) -> jint },
    native_methods {
        // Incorrectly registered as instance method
        fn native_static_method(value: jint) -> jint,
    }
}

impl TestAbiPerMethodNeverNativeInterface for TestAbiPerMethodNeverAPI {
    type Error = jni::errors::Error;

    fn native_static_method<'local>(
        _env: &mut Env<'local>,
        _this: TestAbiPerMethodNever<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 3)
    }
}

native_method_test! {
    test_name: test_abi_per_method_unsafe_never,
    java_class: "com/example/TestNativeAbiCheck.java",
    api: TestAbiPerMethodNeverAPI,
    test_body: |env| {
        // Should NOT panic because native_static_method uses default (UnsafeNever)
        // Note: This is unsafe and wrong, but we're testing that the check is disabled
        let result = TestAbiPerMethodNever::call_static_method(env, 42)?;
        assert_eq!(result, 126);
        Ok(())
    }
}

// ====================================================================================
// Test: UnsafeDebugOnly mode (checks only in debug builds)
// ====================================================================================

bind_java_type! {
    rust_type = TestAbiDebugOnly,
    java_type = "com.example.TestNativeAbiCheck",
    abi_check = UnsafeDebugOnly,
    native_methods_export = false,
    constructors { fn new() },
    methods { fn call_method(value: jint) -> jint },
    native_methods {
        // Incorrectly registered as static
        static fn native_method(value: jint) -> jint,
    }
}

impl TestAbiDebugOnlyNativeInterface for TestAbiDebugOnlyAPI {
    type Error = jni::errors::Error;

    fn native_method<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 2)
    }
}

#[cfg(debug_assertions)]
native_method_test! {
    test_name: test_abi_debug_only_in_debug_mode,
    java_class: "com/example/TestNativeAbiCheck.java",
    api: TestAbiDebugOnlyAPI,
    expect_exception: "UnsafeDebugOnly mode correctly detected ABI mismatch in debug build",
    test_body: |env| {
        let obj = TestAbiDebugOnly::new(env)?;
        // In debug mode, this should panic
        let _result = obj.call_method(env, 42)?;
        Ok(())
    }
}

#[cfg(not(debug_assertions))]
native_method_test! {
    test_name: test_abi_debug_only_in_release_mode,
    java_class: "com/example/TestNativeAbiCheck.java",
    api: TestAbiDebugOnlyAPI,
    test_body: |env| {
        let obj = TestAbiDebugOnly::new(env)?;
        // In release mode, this should NOT panic (check is disabled)
        let result = obj.call_method(env, 42)?;
        assert_eq!(result, 84);
        Ok(())
    }
}
