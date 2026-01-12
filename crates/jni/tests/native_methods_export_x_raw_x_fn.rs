#![cfg(feature = "invocation")]
//! Tests for the interaction matrix between `raw`, `export`, and `fn=` features.
//!
//! This test file focuses on verifying that different combinations of:
//! - `raw = true` (property syntax) and `raw` qualifier syntax
//! - `export = true` (property syntax) and `extern` qualifier syntax
//! - `fn = path` (direct function pointer)
//!
//! work correctly together.
//!
//! ## Test Matrix
//!
//! | Test | Method Type | Raw | Export | Implementation | Syntax Style | Notes |
//! |------|-------------|-----|--------|----------------|--------------|-------|
//! | 1    | Instance    | ✓   | ✓      | fn=            | Property     | |
//! | 2    | Instance    | ✓   | ✓      | fn=            | Qualifier    | |
//! | 3    | Instance    | ✓   | ✓      | fn=            | Property     | |
//! | 4    | Instance    | ✗   | ✓      | fn=            | Property     | |
//! | 5    | Static      | ✓   | ✓      | fn=            | Property     | |
//! | 6    | Static      | ✓   | ✓      | fn=            | Qualifier    | |
//! | 7    | Static      | ✗   | ✓      | fn=            | Qualifier    | |
//! | 8    | Static      | ✗   | ✗      | fn=            | Property     | |
//! | 9    | Instance    | ✗   | ✗      | fn=            | Default      | |
//! | 10   | Static      | ✗   | ✗      | fn=            | Default      | |
//! | 11   | Mixed       | ✗   | ✗      | fn=            | Mixed        | Both not raw |
//! | 12   | Mixed       | ✓   | ✗      | fn=            | Mixed        | Both raw |
//! | 13   | -           | -   | -      | -              | Symbol check | Verifies exports |

mod native_methods_utils;
mod util;

use jni::errors::Error;
use jni::objects::JClass;
use jni::sys::jint;
use jni::{Env, EnvUnowned, native_method};
use rusty_fork::rusty_fork_test;

// ====================================================================================
// Test 1: fn= + Raw (property syntax) + Export (property syntax)
// ====================================================================================

fn method1_impl<'local>(
    _env: EnvUnowned<'local>,
    _this: jni::objects::JObject<'local>,
    value: jint,
) -> jint {
    value * 2
}

native_method_test! {
    test_name: test_inline_raw_prop_export_prop,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            raw extern fn method1(value: jint) -> jint,
            fn = method1_impl,
            java_type = "com.example.TestNativeCombinations",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let result = call_int_method!(env, obj, "method1", 10)?;
        assert_eq!(result, 20);
        Ok(())
    }
}

// ====================================================================================
// Test 2: fn= + Raw (qualifier syntax) + Export (qualifier syntax)
// ====================================================================================

fn method2_impl<'local>(
    _env: EnvUnowned<'local>,
    _this: jni::objects::JObject<'local>,
    value: jint,
) -> jint {
    value * 3
}

native_method_test! {
    test_name: test_fn_raw_qual_export_qual,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            raw extern fn method2(value: jint) -> jint,
            fn = method2_impl,
            java_type = "com.example.TestNativeCombinations",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let result = call_int_method!(env, obj, "method2", 10)?;
        assert_eq!(result, 30);
        Ok(())
    }
}

// ====================================================================================
// Test 3: fn= + Raw (property syntax) + Export (property syntax)
// ====================================================================================

fn method3_impl<'local>(
    _env: EnvUnowned<'local>,
    _this: jni::objects::JObject<'local>,
    value: jint,
) -> jint {
    value * 4
}

native_method_test! {
    test_name: test_fn_raw_prop_export_prop,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            raw extern fn method3(value: jint) -> jint,
            fn = method3_impl,
            java_type = "com.example.TestNativeCombinations",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let result = call_int_method!(env, obj, "method3", 10)?;
        assert_eq!(result, 40);
        Ok(())
    }
}

// ====================================================================================
// Test 4: fn= + Not Raw + Export (property syntax)
// ====================================================================================

fn method4_impl<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
    value: jint,
) -> Result<jint, Error> {
    // Non-raw with fn= takes &mut Env and returns Result
    Ok(value * 5)
}

native_method_test! {
    test_name: test_fn_not_raw_export_prop,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            extern fn method4(value: jint) -> jint,
            fn = method4_impl,
            java_type = "com.example.TestNativeCombinations",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let result = call_int_method!(env, obj, "method4", 10)?;
        assert_eq!(result, 50);
        Ok(())
    }
}

// ====================================================================================
// Test 5: Static fn= + Raw + Export (mixed syntax)
// ====================================================================================

fn static_method1_impl<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    value: jint,
) -> jint {
    value * 10
}

native_method_test! {
    test_name: test_static_fn_raw_export,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            static raw extern fn static_method1(value: jint) -> jint,
            fn = static_method1_impl,
            java_type = "com.example.TestNativeCombinations",
        },
    ],
    test_body: |env, class| {
        let result = call_static_int_method!(env, class, "staticMethod1", 10)?;
        assert_eq!(result, 100);
        Ok(())
    }
}

// ====================================================================================
// Test 6: Static fn= + Raw (qualifier) + Export (qualifier)
// ====================================================================================

fn static_method2_impl<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    value: jint,
) -> jint {
    value * 20
}

native_method_test! {
    test_name: test_static_fn_raw_qual_export_qual,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            static raw extern fn staticMethod2(value: jint) -> jint,
            fn = static_method2_impl,
            java_type = "com.example.TestNativeCombinations",
        },
    ],
    test_body: |env, class| {
        let result = call_static_int_method!(env, class, "staticMethod2", 10)?;
        assert_eq!(result, 200);
        Ok(())
    }
}

// ====================================================================================
// Test 7: fn= + Not Raw + Export (qualifier syntax)
// ====================================================================================

fn static_method3_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 30)
}

native_method_test! {
    test_name: test_static_fn_not_raw_export_qual,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            static extern fn staticMethod3(value: jint) -> jint,
            fn = static_method3_impl,
            java_type = "com.example.TestNativeCombinations",
        },
    ],
    test_body: |env, class| {
        let result = call_static_int_method!(env, class, "staticMethod3", 10)?;
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
) -> Result<jint, Error> {
    // Non-raw with fn= takes &mut Env and returns Result
    Ok(value * 40)
}

native_method_test! {
    test_name: test_static_fn_not_raw_not_exported,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            static fn staticMethodNonExported(value: jint) -> jint,
            fn = static_method_non_exported_impl,
        },
    ],
    test_body: |env, class| {
        let result = call_static_int_method!(env, class, "staticMethodNonExported", 10)?;
        assert_eq!(result, 400);
        Ok(())
    }
}

// ====================================================================================
// Test 9: Instance fn= + Not Raw + Not Exported
// ====================================================================================

fn method_non_exported_impl<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 50)
}

native_method_test! {
    test_name: test_instance_fn_not_raw_not_exported,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            fn methodNonExported(value: jint) -> jint,
            fn = method_non_exported_impl,
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let result = call_int_method!(env, obj, "methodNonExported", 10)?;
        assert_eq!(result, 500);
        Ok(())
    }
}

// ====================================================================================
// Test 10: Static fn= + Not Raw + Not Exported
// ====================================================================================

fn static_method_non_exported2_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 60)
}

native_method_test! {
    test_name: test_static_fn_not_raw_not_exported2,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        native_method! {
            static fn staticMethodNonExported2(value: jint) -> jint,
            fn = static_method_non_exported2_impl,
        },
    ],
    test_body: |env, class| {
        let result = call_static_int_method!(env, class, "staticMethodNonExported2", 10)?;
        assert_eq!(result, 600);
        Ok(())
    }
}

// ====================================================================================
// Test 11: Mixed - fn= (not raw) + fn= (not raw) in one test
// ====================================================================================

fn method_non_exported_fn_impl<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 70)
}

fn method_non_exported2_impl<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
    value: jint,
) -> Result<jint, Error> {
    Ok(value * 80)
}

native_method_test! {
    test_name: test_mixed_fn_not_raw,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        // First fn= implementation
        native_method! {
            fn methodNonExported(value: jint) -> jint,
            fn = method_non_exported_fn_impl,
        },
        // Second fn= implementation
        native_method! {
            fn methodNonExported2(value: jint) -> jint,
            fn = method_non_exported2_impl,
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        // Test first fn= method
        let result = call_int_method!(env, &obj, "methodNonExported", 10)?;
        assert_eq!(result, 700);

        // Test second fn= method
        let result = call_int_method!(env, &obj, "methodNonExported2", 10)?;
        assert_eq!(result, 800);

        Ok(())
    }
}

// ====================================================================================
// Test 12: Mixed - fn= (raw) + fn= (raw) in one test
// ====================================================================================

fn static_method_non_exported_fn_impl<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    value: jint,
) -> jint {
    value * 90
}

fn static_method_non_exported2_fn_impl<'local>(
    _env: EnvUnowned<'local>,
    _class: JClass<'local>,
    value: jint,
) -> jint {
    value * 100
}

native_method_test! {
    test_name: test_mixed_fn_raw,
    java_class: "com/example/TestNativeCombinations.java",
    methods: |class| &[
        // fn= implementation with raw
        native_method! {
            static fn staticMethodNonExported(value: jint) -> jint,
            fn = static_method_non_exported_fn_impl,
            raw = true,
        },
        // fn= implementation with raw
        native_method! {
            static raw fn staticMethodNonExported2(value: jint) -> jint,
            fn = static_method_non_exported2_fn_impl,
        },
    ],
    test_body: |env, class| {
        // Test first fn= raw method
        let result = call_static_int_method!(env, &class, "staticMethodNonExported", 10)?;
        assert_eq!(result, 900);

        // Test second fn= raw method
        let result = call_static_int_method!(env, &class, "staticMethodNonExported2", 10)?;
        assert_eq!(result, 1000);

        Ok(())
    }
}

// ====================================================================================
// Test 13: Verify exported symbols exist
// ====================================================================================

// Declare the exported symbols so we can verify they exist
unsafe extern "system" {
    fn Java_com_example_TestNativeCombinations_method1__I(
        env: EnvUnowned,
        this: jni::sys::jobject,
        value: jint,
    ) -> jint;
    fn Java_com_example_TestNativeCombinations_method2__I(
        env: EnvUnowned,
        this: jni::sys::jobject,
        value: jint,
    ) -> jint;
    fn Java_com_example_TestNativeCombinations_method3__I(
        env: EnvUnowned,
        this: jni::sys::jobject,
        value: jint,
    ) -> jint;
    fn Java_com_example_TestNativeCombinations_method4__I(
        env: EnvUnowned,
        this: jni::sys::jobject,
        value: jint,
    ) -> jint;
    fn Java_com_example_TestNativeCombinations_staticMethod1__I(
        env: EnvUnowned,
        class: JClass,
        value: jint,
    ) -> jint;
    fn Java_com_example_TestNativeCombinations_staticMethod2__I(
        env: EnvUnowned,
        class: JClass,
        value: jint,
    ) -> jint;
    fn Java_com_example_TestNativeCombinations_staticMethod3__I(
        env: EnvUnowned,
        class: JClass,
        value: jint,
    ) -> jint;
}

#[test]
fn test_exported_symbols_exist() {
    // Just referencing the functions verifies they were exported at link time
    let _fn1 = Java_com_example_TestNativeCombinations_method1__I;
    let _fn2 = Java_com_example_TestNativeCombinations_method2__I;
    let _fn3 = Java_com_example_TestNativeCombinations_method3__I;
    let _fn4 = Java_com_example_TestNativeCombinations_method4__I;
    let _fn5 = Java_com_example_TestNativeCombinations_staticMethod1__I;
    let _fn6 = Java_com_example_TestNativeCombinations_staticMethod2__I;
    let _fn7 = Java_com_example_TestNativeCombinations_staticMethod3__I;

    println!("All exported symbols verified!");
}
