#![cfg(feature = "invocation")]
//! Tests for the interaction matrix between `raw`, `export`, and `fn=` features.
//!
//! This test file focuses on verifying that different combinations of:
//! - `raw = true` (property syntax) and `raw` qualifier syntax
//! - `export = true` (property syntax) and `extern` qualifier syntax
//! - `fn = path` (direct function pointer) vs trait implementation
//!
//! work correctly together.
//!
//! ## Test Matrix
//!
//! | Test | Method Type | Raw | Export | Implementation | Syntax Style | Notes |
//! |------|-------------|-----|--------|----------------|--------------|-------|
//! | 1    | Instance    | ✓   | ✓      | Trait          | Property     | |
//! | 2    | Instance    | ✓   | ✓      | Trait          | Qualifier    | |
//! | 3    | Instance    | ✓   | ✓      | fn=            | Property     | |
//! | 4    | Instance    | ✗   | ✓      | fn=            | Property     | |
//! | 5    | Static      | ✓   | ✓      | Trait          | Property     | |
//! | 6    | Static      | ✓   | ✓      | fn=            | Qualifier    | |
//! | 7    | Static      | ✗   | ✓      | Trait          | Qualifier    | |
//! | 8    | Static      | ✗   | ✗      | fn=            | Property     | |
//! | 9    | Instance    | ✗   | ✗      | Trait          | Default      | |
//! | 10   | Static      | ✗   | ✗      | Trait          | Default      | |
//! | 11   | Mixed       | ✗   | ✗      | fn= + Trait    | Mixed        | Both not raw |
//! | 12   | Mixed       | ✓   | ✗      | fn= + Trait    | Mixed        | Both raw |
//! | 13   | -           | -   | -      | -              | Symbol check | Verifies exports |

#[macro_use]
mod bind_native_methods_utils;
mod util;

use jni::objects::JClass;
use jni::sys::jint;
use jni::{Env, EnvUnowned, bind_java_type};
use rusty_fork::rusty_fork_test;

// ====================================================================================
// Test 1: Trait + Raw (property syntax) + Export (property syntax)
// ====================================================================================

bind_java_type! {
    rust_type = TestCombo1,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        pub fn method1 {
            sig = (value: jint) -> jint,
            raw = true,
            export = true,
        },
    }
}

impl TestCombo1NativeInterface for TestCombo1API {
    type Error = jni::errors::Error;

    fn method1<'local>(_env: EnvUnowned<'local>, _this: TestCombo1<'local>, value: jint) -> jint {
        value * 2
    }
}

native_method_test! {
    test_name: test_trait_raw_prop_export_prop,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo1API,
    test_body: |env| {
        let obj = TestCombo1::new(env)?;
        let result = obj.method1(env, 10)?;
        assert_eq!(result, 20);
        Ok(())
    }
}

// ====================================================================================
// Test 2: Trait + Raw (qualifier syntax) + Export (qualifier syntax)
// ====================================================================================

bind_java_type! {
    rust_type = TestCombo2,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        pub raw extern fn method2(value: jint) -> jint,
    }
}

impl TestCombo2NativeInterface for TestCombo2API {
    type Error = jni::errors::Error;

    fn method2<'local>(_env: EnvUnowned<'local>, _this: TestCombo2<'local>, value: jint) -> jint {
        value * 3
    }
}

native_method_test! {
    test_name: test_trait_raw_qual_export_qual,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo2API,
    test_body: |env| {
        let obj = TestCombo2::new(env)?;
        let result = obj.method2(env, 10)?;
        assert_eq!(result, 30);
        Ok(())
    }
}

// ====================================================================================
// Test 3: fn= + Raw (property syntax) + Export (property syntax)
// ====================================================================================

extern "system" fn method3_impl<'local>(
    _env: EnvUnowned<'local>,
    _this: TestCombo3<'local>,
    value: jint,
) -> jint {
    value * 4
}

bind_java_type! {
    rust_type = TestCombo3,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        pub fn method3 {
            sig = (value: jint) -> jint,
            fn = method3_impl,
            raw = true,
            export = true,
        },
    }
}

native_method_test! {
    test_name: test_fn_raw_prop_export_prop,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo3API,
    test_body: |env| {
        let obj = TestCombo3::new(env)?;
        let result = obj.method3(env, 10)?;
        assert_eq!(result, 40);
        Ok(())
    }
}

// ====================================================================================
// Test 4: fn= + Not Raw + Export (property syntax)
// ====================================================================================

fn method4_impl<'local>(
    _env: &mut Env<'local>,
    _this: TestCombo4<'local>,
    value: jint,
) -> Result<jint, jni::errors::Error> {
    // Non-raw with fn= takes &mut Env and returns Result
    Ok(value * 5)
}

bind_java_type! {
    rust_type = TestCombo4,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        pub fn method4 {
            sig = (value: jint) -> jint,
            fn = method4_impl,
            export = true,
        },
    }
}

native_method_test! {
    test_name: test_fn_not_raw_export_prop,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo4API,
    test_body: |env| {
        let obj = TestCombo4::new(env)?;
        let result = obj.method4(env, 10)?;
        assert_eq!(result, 50);
        Ok(())
    }
}

// ====================================================================================
// Test 5: Static Trait + Raw + Export (mixed syntax)
// ====================================================================================

bind_java_type! {
    rust_type = TestCombo5,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    native_methods {
        pub static fn static_method1 {
            sig = (value: jint) -> jint,
            raw = true,
            export = true,
        },
    }
}

impl TestCombo5NativeInterface for TestCombo5API {
    type Error = jni::errors::Error;

    fn static_method1<'local>(
        _env: EnvUnowned<'local>,
        _class: JClass<'local>,
        value: jint,
    ) -> jint {
        value * 10
    }
}

native_method_test! {
    test_name: test_static_trait_raw_export,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo5API,
    test_body: |env| {
        let result = TestCombo5::static_method1(env, 10)?;
        assert_eq!(result, 100);
        Ok(())
    }
}

// ====================================================================================
// Test 6: Static fn= + Raw (qualifier) + Export (qualifier)
// ====================================================================================

extern "system" fn static_method2_impl<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    value: jint,
) -> jint {
    value * 20
}

bind_java_type! {
    rust_type = TestCombo6,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    native_methods {
        pub static raw extern fn static_method2 {
            sig = (value: jint) -> jint,
            fn = static_method2_impl,
        },
    }
}

native_method_test! {
    test_name: test_static_fn_raw_qual_export_qual,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo6API,
    test_body: |env| {
        let result = TestCombo6::static_method2(env, 10)?;
        assert_eq!(result, 200);
        Ok(())
    }
}

// ====================================================================================
// Test 7: Trait + Not Raw + Export (qualifier syntax)
// ====================================================================================

bind_java_type! {
    rust_type = TestCombo7,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    native_methods {
        pub static extern fn static_method3(value: jint) -> jint,
    }
}

impl TestCombo7NativeInterface for TestCombo7API {
    type Error = jni::errors::Error;

    fn static_method3<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 30)
    }
}

native_method_test! {
    test_name: test_static_trait_not_raw_export_qual,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo7API,
    test_body: |env| {
        let result = TestCombo7::static_method3(env, 10)?;
        assert_eq!(result, 300);
        Ok(())
    }
}

// ====================================================================================
// Test 8: fn= + Not Raw + Not Exported
// ====================================================================================

fn static_method_non_exported_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    value: jint,
) -> Result<jint, jni::errors::Error> {
    // Non-raw with fn= takes &mut Env and returns Result
    Ok(value * 40)
}

bind_java_type! {
    rust_type = TestCombo8,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    native_methods {
        pub static fn static_method_non_exported {
            sig = (value: jint) -> jint,
            fn = static_method_non_exported_impl,
        },
    }
}

native_method_test! {
    test_name: test_static_fn_not_raw_not_exported,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo8API,
    test_body: |env| {
        let result = TestCombo8::static_method_non_exported(env, 10)?;
        assert_eq!(result, 400);
        Ok(())
    }
}

// ====================================================================================
// Test 9: Instance Trait + Not Raw + Not Exported
// ====================================================================================

bind_java_type! {
    rust_type = TestCombo10,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        pub fn method_non_exported(value: jint) -> jint,  // No qualifiers
    }
}

impl TestCombo10NativeInterface for TestCombo10API {
    type Error = jni::errors::Error;

    fn method_non_exported<'local>(
        _env: &mut Env<'local>,
        _this: TestCombo10<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 50)
    }
}

native_method_test! {
    test_name: test_instance_trait_not_raw_not_exported,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo10API,
    test_body: |env| {
        let obj = TestCombo10::new(env)?;
        let result = obj.method_non_exported(env, 10)?;
        assert_eq!(result, 500);
        Ok(())
    }
}

// ====================================================================================
// Test 10: Static Trait + Not Raw + Not Exported
// ====================================================================================

bind_java_type! {
    rust_type = TestCombo11,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    native_methods {
        pub static fn static_method_non_exported(value: jint) -> jint,
    }
}

impl TestCombo11NativeInterface for TestCombo11API {
    type Error = jni::errors::Error;

    fn static_method_non_exported<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 60)
    }
}

native_method_test! {
    test_name: test_static_trait_not_raw_not_exported,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo11API,
    test_body: |env| {
        let result = TestCombo11::static_method_non_exported(env, 10)?;
        assert_eq!(result, 600);
        Ok(())
    }
}

// ====================================================================================
// Test 11: Mixed - fn= (not raw) + Trait (not raw) in one binding
// ====================================================================================

fn method_non_exported_fn_impl<'local>(
    _env: &mut Env<'local>,
    _this: TestCombo12<'local>,
    value: jint,
) -> Result<jint, jni::errors::Error> {
    Ok(value * 70)
}

bind_java_type! {
    rust_type = TestCombo12,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        // fn= implementation
        pub fn method_non_exported {
            sig = (value: jint) -> jint,
            fn = method_non_exported_fn_impl,
        },
        // Trait implementation
        pub fn method_non_exported2(value: jint) -> jint,
    }
}

// Trait should only require method_non_exported2 to be implemented (method_non_exported uses fn=)
impl TestCombo12NativeInterface for TestCombo12API {
    type Error = jni::errors::Error;

    fn method_non_exported2<'local>(
        _env: &mut Env<'local>,
        _this: TestCombo12<'local>,
        value: jint,
    ) -> Result<jint, Self::Error> {
        Ok(value * 80)
    }
}

native_method_test! {
    test_name: test_mixed_fn_and_trait_not_raw,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo12API,
    test_body: |env| {
        let obj = TestCombo12::new(env)?;

        // Test fn= method
        let result = obj.method_non_exported(env, 10)?;
        assert_eq!(result, 700);

        // Test trait method
        let result = obj.method_non_exported2(env, 10)?;
        assert_eq!(result, 800);

        Ok(())
    }
}

// ====================================================================================
// Test 12: Mixed - fn= (raw) + Trait (raw) in one binding
// ====================================================================================

fn static_method_non_exported_fn_impl<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    value: jint,
) -> jint {
    value * 90
}

bind_java_type! {
    rust_type = TestCombo13,
    java_type = "com.example.TestNativeCombinations",
    native_methods_export = false,
    native_methods {
        // fn= implementation with raw
        pub static fn static_method_non_exported {
            sig = (value: jint) -> jint,
            fn = static_method_non_exported_fn_impl,
            raw = true,
        },
        // Trait implementation with raw
        pub static raw fn static_method_non_exported2(value: jint) -> jint,
    }
}

// Trait should only require static_method_non_exported2 to be implemented (static_method_non_exported uses fn=)
impl TestCombo13NativeInterface for TestCombo13API {
    type Error = jni::errors::Error;

    fn static_method_non_exported2<'local>(
        _env: EnvUnowned<'local>,
        _class: JClass<'local>,
        value: jint,
    ) -> jint {
        value * 100
    }
}

native_method_test! {
    test_name: test_mixed_fn_and_trait_raw,
    java_class: "com/example/TestNativeCombinations.java",
    api: TestCombo13API,
    test_body: |env| {
        // Test fn= raw method
        let result = TestCombo13::static_method_non_exported(env, 10)?;
        assert_eq!(result, 900);

        // Test trait raw method
        let result = TestCombo13::static_method_non_exported2(env, 10)?;
        assert_eq!(result, 1000);

        Ok(())
    }
}

// ====================================================================================
// Test 13: Verify exported symbols exist
// ====================================================================================

#[test]
fn test_exported_symbols_exist() {
    let _fn1 = Java_com_example_TestNativeCombinations_method1__I;
    let _fn2 = Java_com_example_TestNativeCombinations_method2__I;
    let _fn3 = Java_com_example_TestNativeCombinations_method3__I;
    let _fn4 = Java_com_example_TestNativeCombinations_method4__I;
    let _fn5 = Java_com_example_TestNativeCombinations_staticMethod1__I;
    let _fn6 = Java_com_example_TestNativeCombinations_staticMethod2__I;
    let _fn7 = Java_com_example_TestNativeCombinations_staticMethod3__I;

    println!("All exported symbols verified!");
}
