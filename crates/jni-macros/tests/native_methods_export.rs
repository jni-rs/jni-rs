//! Tests for native method export symbol generation.
//!
//! These tests verify that the `native_method!` macro correctly generates
//! exported JNI symbols with proper name mangling for various scenarios.
//!
//! Note: This is a representative subset of export tests. Most name mangling
//! variations are not included since they're handled by the same underlying
//! code used by both bind_java_type! and native_method! macros.

#[macro_use]
mod native_methods_utils;
mod util;

use jni::objects::{JObject, JString};
use jni::sys::{jboolean, jint, jlong};
use jni::{Env, EnvUnowned, native_method};
use rusty_fork::rusty_fork_test;

// Declare the exported JNI symbols that the native_method! macro creates
// These use EnvUnowned and JNI reference types which are FFI-safe
unsafe extern "system" {
    fn Java_com_example_TestNativeExports_noArgs__<'local>(
        env: EnvUnowned<'local>,
        this: JObject<'local>,
    ) -> jint;
    fn Java_com_example_TestNativeExports_primitiveArgs__IJZ<'local>(
        env: EnvUnowned<'local>,
        this: JObject<'local>,
        a: jint,
        b: jlong,
        c: jboolean,
    ) -> jint;
    fn Java_com_example_TestNativeExports_objectArgs__Ljava_lang_String_2Ljava_lang_Object_2<
        'local,
    >(
        env: EnvUnowned<'local>,
        this: JObject<'local>,
        str: JString<'local>,
        obj: JObject<'local>,
    ) -> JString<'local>;
    fn Java_com_example_TestNativeExports_overloaded__<'local>(
        env: EnvUnowned<'local>,
        this: JObject<'local>,
    ) -> jint;
    fn Java_com_example_TestNativeExports_overloaded__I<'local>(
        env: EnvUnowned<'local>,
        this: JObject<'local>,
        x: jint,
    ) -> jint;
    fn Java_com_example_TestNativeExports_overloaded__Ljava_lang_String_2<'local>(
        env: EnvUnowned<'local>,
        this: JObject<'local>,
        s: JString<'local>,
    ) -> jint;
    fn Java_com_example_TestNativeExports_method_1with_1underscore__<'local>(
        env: EnvUnowned<'local>,
        this: JObject<'local>,
    );
    fn Java_com_example_TestNativeExports_manualExport<'local>(
        env: EnvUnowned<'local>,
        this: JObject<'local>,
    );
}

// ====================================================================================
// Test: No arguments with extern keyword (implicit export)
// ====================================================================================

native_method_test! {
    test_name: test_export_no_args,
    java_class: "com/example/TestNativeExports.java",
    methods: |class| &[
        native_method! {
            extern fn no_args() -> jint,
            fn = no_args_impl,
            java_type = "com.example.TestNativeExports",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        // Call through Java to ensure the native implementation is linked
        use jni::{jni_sig, jni_str};
        let result_via_java = env.call_method(&obj, jni_str!("noArgs"), jni_sig!("()I"), &[])?.i()?;
        assert_eq!(result_via_java, 1234);

        // Also test calling the exported symbol directly
        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let ret = Java_com_example_TestNativeExports_noArgs__(unowned, obj);
            assert_eq!(ret, 1234);
        }
        Ok(())
    }
}

fn no_args_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
) -> Result<jint, jni::errors::Error> {
    Ok(1234)
}

// ====================================================================================
// Test: Primitive arguments with explicit export = true
// ====================================================================================

native_method_test! {
    test_name: test_export_primitive_args,
    java_class: "com/example/TestNativeExports.java",
    methods: |class| &[
        native_method! {
            fn primitive_args(a: jint, b: jlong, c: jboolean) -> jint,
            fn = primitive_args_impl,
            export = true,
            java_type = "com.example.TestNativeExports",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let ret = Java_com_example_TestNativeExports_primitiveArgs__IJZ(
                unowned, obj, 10, 20, true as jboolean
            );
            assert_eq!(ret, 31);
        }
        Ok(())
    }
}

fn primitive_args_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    a: jint,
    b: jlong,
    c: jboolean,
) -> Result<jint, jni::errors::Error> {
    Ok(a + b as jint + c as jint)
}

// ====================================================================================
// Test: Object arguments
// ====================================================================================

native_method_test! {
    test_name: test_export_object_args,
    java_class: "com/example/TestNativeExports.java",
    methods: |class| &[
        native_method! {
            fn object_args(str: JString, obj: JObject) -> JString,
            fn = object_args_impl,
            export = true,
            java_type = "com.example.TestNativeExports",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let str_arg = env.new_string("test")?;
        let obj_arg = env.new_string("object")?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let result = Java_com_example_TestNativeExports_objectArgs__Ljava_lang_String_2Ljava_lang_Object_2(
                unowned, obj, str_arg, obj_arg.into()
            );
            assert!(!result.as_raw().is_null());
        }
        Ok(())
    }
}

fn object_args_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    str: JString<'local>,
    _obj: JObject<'local>,
) -> Result<JString<'local>, jni::errors::Error> {
    Ok(str)
}

// ====================================================================================
// Test: Overloaded methods with different signatures
// ====================================================================================

native_method_test! {
    test_name: test_export_overloaded_no_args,
    java_class: "com/example/TestNativeExports.java",
    methods: |class| &[
        native_method! {
            fn overloaded() -> jint,
            fn = overloaded_no_args_impl,
            export = true,
            java_type = "com.example.TestNativeExports",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let result = Java_com_example_TestNativeExports_overloaded__(unowned, obj);
            assert_eq!(result, 1);
        }
        Ok(())
    }
}

fn overloaded_no_args_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
) -> Result<jint, jni::errors::Error> {
    Ok(1)
}

native_method_test! {
    test_name: test_export_overloaded_int,
    java_class: "com/example/TestNativeExports.java",
    methods: |class| &[
        native_method! {
            extern fn overloaded(x: jint) -> jint,
            fn = overloaded_int_impl,
            java_type = "com.example.TestNativeExports",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let result = Java_com_example_TestNativeExports_overloaded__I(unowned, obj, 5);
            assert_eq!(result, 10);
        }
        Ok(())
    }
}

fn overloaded_int_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    x: jint,
) -> Result<jint, jni::errors::Error> {
    Ok(x * 2)
}

native_method_test! {
    test_name: test_export_overloaded_string,
    java_class: "com/example/TestNativeExports.java",
    methods: |class| &[
        native_method! {
            fn overloaded(s: JString) -> jint,
            fn = overloaded_string_impl,
            export = true,
            java_type = "com.example.TestNativeExports",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;
        let input = env.new_string("hello")?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let result = Java_com_example_TestNativeExports_overloaded__Ljava_lang_String_2(
                unowned, obj, input
            );
            assert_eq!(result, 5);
        }
        Ok(())
    }
}

fn overloaded_string_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
    _s: JString<'local>,
) -> Result<jint, jni::errors::Error> {
    // Just return a fixed value - we're testing exports, not string handling
    Ok(5)
}

// ====================================================================================
// Test: Underscore in method name (needs mangling)
// ====================================================================================

native_method_test! {
    test_name: test_export_underscore_method_name,
    java_class: "com/example/TestNativeExports.java",
    methods: |class| &[
        native_method! {
            fn method_with_underscore() -> (),
            fn = method_with_underscore_impl,
            name = "method_with_underscore",  // Explicit name needed since Java method has underscores
            export = true,
            java_type = "com.example.TestNativeExports",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeExports_method_1with_1underscore__(unowned, obj);
        }
        Ok(())
    }
}

fn method_with_underscore_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
) -> Result<(), jni::errors::Error> {
    Ok(())
}

// ====================================================================================
// Test: Manual export symbol
// ====================================================================================

native_method_test! {
    test_name: test_export_manual_symbol,
    java_class: "com/example/TestNativeExports.java",
    methods: |class| &[
        native_method! {
            fn manual_export() -> (),
            fn = manual_export_impl,
            export = "Java_com_example_TestNativeExports_manualExport",
            java_type = "com.example.TestNativeExports",
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeExports_manualExport(unowned, obj);
        }
        Ok(())
    }
}

fn manual_export_impl<'local>(
    _env: &mut Env<'local>,
    _this: JObject<'local>,
) -> Result<(), jni::errors::Error> {
    Ok(())
}
