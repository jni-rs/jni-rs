//! Tests for native method export symbol generation.
//!
//! These tests verify that the `bind_java_type!` macro correctly generates
//! exported JNI symbols with proper name mangling for various scenarios.

#[macro_use]
mod bind_native_methods_utils;
mod util;

use jni::objects::{JByteArray, JIntArray, JObject, JObjectArray, JString};
use jni::sys::{jboolean, jint, jlong};
use jni::{Env, EnvUnowned, bind_java_type};
use rusty_fork::rusty_fork_test;

// ====================================================================================
// Test: No arguments
// ====================================================================================

bind_java_type! {
    rust_type = TestNoArgs,
    java_type = "com.example.TestNativeExports",
    constructors { fn new() },
    native_methods {
        extern fn no_args {
            sig = () -> jint,
        },
    }
}

impl TestNoArgsNativeInterface for TestNoArgsAPI {
    type Error = jni::errors::Error;

    fn no_args<'local>(
        _env: &mut Env<'local>,
        _this: TestNoArgs<'local>,
    ) -> Result<jint, Self::Error> {
        Ok(1234)
    }
}

native_method_test! {
    test_name: test_export_no_args,
    java_class: "com/example/TestNativeExports.java",
    api: TestNoArgsAPI,
    test_body: |env| {
        let obj = TestNoArgs::new(env)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let ret = Java_com_example_TestNativeExports_noArgs__(unowned, obj);
            assert_eq!(ret, 1234);
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Primitive arguments
// ====================================================================================

bind_java_type! {
    rust_type = TestPrimitiveArgs,
    java_type = "com.example.TestNativeExports",
    constructors { fn new() },
    native_methods {
        fn primitive_args {
            sig = (a: jint, b: jlong, c: jboolean) -> jint,
            export = true,
        },
    }
}

impl TestPrimitiveArgsNativeInterface for TestPrimitiveArgsAPI {
    type Error = jni::errors::Error;

    fn primitive_args<'local>(
        _env: &mut Env<'local>,
        _this: TestPrimitiveArgs<'local>,
        a: jint,
        b: jlong,
        c: jboolean,
    ) -> Result<jint, Self::Error> {
        Ok(a + b as jint + c as jint)
    }
}

native_method_test! {
    test_name: test_export_primitive_args,
    java_class: "com/example/TestNativeExports.java",
    api: TestPrimitiveArgsAPI,
    test_body: |env| {
        let obj = TestPrimitiveArgs::new(env)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let ret = Java_com_example_TestNativeExports_primitiveArgs__IJZ(unowned, obj, 10, 20, true as jboolean);
            assert_eq!(ret, 31);
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Primitive array arguments
// ====================================================================================

bind_java_type! {
    rust_type = TestPrimitiveArrayArgs,
    java_type = "com.example.TestNativeExports",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        extern fn primitive_array_args(arr: jint[], bytes: jbyte[]),
    }
}

impl TestPrimitiveArrayArgsNativeInterface for TestPrimitiveArrayArgsAPI {
    type Error = jni::errors::Error;

    fn primitive_array_args<'local>(
        _env: &mut Env<'local>,
        _this: TestPrimitiveArrayArgs<'local>,
        _arr: JIntArray<'local>,
        _bytes: JByteArray<'local>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

native_method_test! {
    test_name: test_export_primitive_array_args,
    java_class: "com/example/TestNativeExports.java",
    api: TestPrimitiveArrayArgsAPI,
    test_body: |env| {
        let obj = TestPrimitiveArrayArgs::new(env)?;
        let int_arr = env.new_int_array(5)?;
        let byte_arr = env.new_byte_array(3)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeExports_primitiveArrayArgs___3I_3B(
                unowned, obj, int_arr, byte_arr
            );
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Object arguments
// ====================================================================================

bind_java_type! {
    rust_type = TestObjectArgs,
    java_type = "com.example.TestNativeExports",
    constructors { fn new() },
    native_methods {
        fn object_args {
            sig = (str: JString, obj: JObject) -> JString,
            export = true,
        },
    }
}

impl TestObjectArgsNativeInterface for TestObjectArgsAPI {
    type Error = jni::errors::Error;

    fn object_args<'local>(
        _env: &mut Env<'local>,
        _this: TestObjectArgs<'local>,
        str: JString<'local>,
        _obj: JObject<'local>,
    ) -> Result<JString<'local>, Self::Error> {
        Ok(str)
    }
}

native_method_test! {
    test_name: test_export_object_args,
    java_class: "com/example/TestNativeExports.java",
    api: TestObjectArgsAPI,
    test_body: |env| {
        let obj = TestObjectArgs::new(env)?;
        let str_arg = env.new_string("test")?;
        let obj_arg = env.new_string("object")?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let result = Java_com_example_TestNativeExports_objectArgs__Ljava_lang_String_2Ljava_lang_Object_2(
                unowned, obj, str_arg, obj_arg.into()
            );
            assert!(!result.is_null());
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Object array arguments
// ====================================================================================

bind_java_type! {
    rust_type = TestObjectArrayArgs,
    java_type = "com.example.TestNativeExports",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        extern fn object_array_args {
            sig = (strings: JString[], objects: JObject[]) -> (),
        },
    }
}

impl TestObjectArrayArgsNativeInterface for TestObjectArrayArgsAPI {
    type Error = jni::errors::Error;

    fn object_array_args<'local>(
        _env: &mut Env<'local>,
        _this: TestObjectArrayArgs<'local>,
        _strings: JObjectArray<'local, JString<'local>>,
        _objects: JObjectArray<'local, JObject<'local>>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

native_method_test! {
    test_name: test_export_object_array_args,
    java_class: "com/example/TestNativeExports.java",
    api: TestObjectArrayArgsAPI,
    test_body: |env| {
        let obj = TestObjectArrayArgs::new(env)?;
        let str_arr = JObjectArray::<JString>::new(env, 2, JString::null())?;
        let obj_arr = JObjectArray::<JObject>::new(env, 2, JObject::null())?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeExports_objectArrayArgs___3Ljava_lang_String_2_3Ljava_lang_Object_2(
                unowned, obj, str_arr, obj_arr
            );
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Overloaded methods (no args)
// ====================================================================================

bind_java_type! {
    rust_type = TestOverloadedNoArgs,
    java_type = "com.example.TestNativeExports",
    constructors { fn new() },
    native_methods {
        fn overloaded {
            sig = () -> jint,
            export = true,
        },
    }
}

impl TestOverloadedNoArgsNativeInterface for TestOverloadedNoArgsAPI {
    type Error = jni::errors::Error;

    fn overloaded<'local>(
        _env: &mut Env<'local>,
        _this: TestOverloadedNoArgs<'local>,
    ) -> Result<jint, Self::Error> {
        Ok(1)
    }
}

native_method_test! {
    test_name: test_export_overloaded_no_args,
    java_class: "com/example/TestNativeExports.java",
    api: TestOverloadedNoArgsAPI,
    test_body: |env| {
        let obj = TestOverloadedNoArgs::new(env)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let result = Java_com_example_TestNativeExports_overloaded__(unowned, obj);
            assert_eq!(result, 1);
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Overloaded methods (int arg)
// ====================================================================================

bind_java_type! {
    rust_type = TestOverloadedInt,
    java_type = "com.example.TestNativeExports",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        extern fn overloaded {
            sig = (x: jint) -> jint,
        },
    }
}

impl TestOverloadedIntNativeInterface for TestOverloadedIntAPI {
    type Error = jni::errors::Error;

    fn overloaded<'local>(
        _env: &mut Env<'local>,
        _this: TestOverloadedInt<'local>,
        x: jint,
    ) -> Result<jint, Self::Error> {
        Ok(x * 2)
    }
}

native_method_test! {
    test_name: test_export_overloaded_int,
    java_class: "com/example/TestNativeExports.java",
    api: TestOverloadedIntAPI,
    test_body: |env| {
        let obj = TestOverloadedInt::new(env)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            let result = Java_com_example_TestNativeExports_overloaded__I(unowned, obj, 5);
            assert_eq!(result, 10);
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Overloaded methods (String arg)
// ====================================================================================

bind_java_type! {
    rust_type = TestOverloadedString,
    java_type = "com.example.TestNativeExports",
    constructors { fn new() },
    native_methods {
        fn overloaded {
            sig = (s: JString) -> jint,
            export = true,
        },
    }
}

impl TestOverloadedStringNativeInterface for TestOverloadedStringAPI {
    type Error = jni::errors::Error;

    fn overloaded<'local>(
        _env: &mut Env<'local>,
        _this: TestOverloadedString<'local>,
        _s: JString<'local>,
    ) -> Result<jint, Self::Error> {
        // Just return a fixed value - we're testing exports, not string handling
        Ok(5)
    }
}

native_method_test! {
    test_name: test_export_overloaded_string,
    java_class: "com/example/TestNativeExports.java",
    api: TestOverloadedStringAPI,
    test_body: |env| {
        let obj = TestOverloadedString::new(env)?;
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

// ====================================================================================
// Test: Unicode in method name (needs mangling)
// ====================================================================================

bind_java_type! {
    rust_type = TestUnicode,
    java_type = "com.example.TestNativeExports",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        extern fn méthod {
            sig = () -> (),
        },
    }
}

impl TestUnicodeNativeInterface for TestUnicodeAPI {
    type Error = jni::errors::Error;

    fn méthod<'local>(
        _env: &mut Env<'local>,
        _this: TestUnicode<'local>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

native_method_test! {
    test_name: test_export_unicode_method_name,
    java_class: "com/example/TestNativeExports.java",
    api: TestUnicodeAPI,
    test_body: |env| {
        let obj = TestUnicode::new(env)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeExports_m_000e9thod__(unowned, obj);
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Underscore in method name (needs mangling)
// ====================================================================================

bind_java_type! {
    rust_type = TestUnderscore,
    java_type = "com.example.TestNativeExports",
    constructors { fn new() },
    native_methods {
        fn method_with_underscore {
            name = "method_with_underscore",
            sig = () -> (),
            export = true,
        },
    }
}

impl TestUnderscoreNativeInterface for TestUnderscoreAPI {
    type Error = jni::errors::Error;

    fn method_with_underscore<'local>(
        _env: &mut Env<'local>,
        _this: TestUnderscore<'local>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

native_method_test! {
    test_name: test_export_underscore_method_name,
    java_class: "com/example/TestNativeExports.java",
    api: TestUnderscoreAPI,
    test_body: |env| {
        let obj = TestUnderscore::new(env)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeExports_method_1with_1underscore__(unowned, obj);
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Manual export symbol
// ====================================================================================

bind_java_type! {
    rust_type = TestManualExport,
    java_type = "com.example.TestNativeExports",
    native_methods_export = false,
    constructors { fn new() },
    native_methods {
        fn manual_export {
            sig = () -> (),
            export = "Java_com_example_TestNativeExports_manualExport",
        },
    }
}

impl TestManualExportNativeInterface for TestManualExportAPI {
    type Error = jni::errors::Error;

    fn manual_export<'local>(
        _env: &mut Env<'local>,
        _this: TestManualExport<'local>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

native_method_test! {
    test_name: test_export_manual_symbol,
    java_class: "com/example/TestNativeExports.java",
    api: TestManualExportAPI,
    test_body: |env| {
        let obj = TestManualExport::new(env)?;

        unsafe {
            let unowned = EnvUnowned::from_raw(env.get_raw());
            Java_com_example_TestNativeExports_manualExport(unowned, obj);
        }
        Ok(())
    }
}

// ====================================================================================
// Test: Inner class with '$' in class name
// ====================================================================================

bind_java_type! {
    rust_type = TestInnerClass,
    java_type = "com.example.TestNativeExports$Inner",
    constructors { fn new() },
    native_methods {
        fn inner_method {
            sig = () -> (),
            export = true,
        },
    }
}

impl TestInnerClassNativeInterface for TestInnerClassAPI {
    type Error = jni::errors::Error;

    fn inner_method<'local>(
        _env: &mut Env<'local>,
        _this: TestInnerClass<'local>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

rusty_fork::rusty_fork_test! {
    #[test]
    fn test_export_inner_class() {
        let out_dir = util::setup_test_output("test_export_inner_class");

        javac::Build::new()
            .file("tests/java/com/example/TestNativeExports.java")
            .output_dir(&out_dir)
            .compile();

        util::attach_current_thread(|env| {
            // Load both outer and inner class
            util::load_test_class(env, &out_dir, "TestNativeExports")?;
            util::load_test_class(env, &out_dir, "TestNativeExports$Inner")?;

            let loader = jni::refs::LoaderContext::default();
            TestInnerClassAPI::get(env, &loader)?;

            let obj = TestInnerClass::new(env)?;

            unsafe {
                let unowned = EnvUnowned::from_raw(env.get_raw());
                Java_com_example_TestNativeExports_00024Inner_innerMethod__(unowned, obj);
            }
            Ok(())
        })
        .expect("test_export_inner_class failed");
    }
}
