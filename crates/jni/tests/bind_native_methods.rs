#![cfg(feature = "invocation")]
mod bind_native_methods_utils;
mod util;

use jni::Env;
use jni::bind_java_type;
use jni::objects::JBooleanArray;
use jni::objects::{JClass, JObjectArray, JPrimitiveArray, JString};
use jni::refs::IntoAuto as _;
use jni::sys::{jboolean, jint};
use rusty_fork::rusty_fork_test;

bind_java_type! {
    rust_type = TestNativeMethods,
    java_type = "com.example.TestNativeMethods",
    constructors {
        fn new(),
    },
    methods {
        fn get_counter() -> jint,
        fn set_counter(value: jint) -> void,
        fn get_message() -> JString,
        // Wrapper methods to test calling native methods indirectly from Java
        fn call_native_add(a: jint, b: jint) -> jint,
        static fn call_native_get_version() -> jint,
    },
    native_methods {
        fn native_add(a: jint, b: jint) -> jint,
        pub fn native_log(message: JString) -> void,
        pub fn native_array_add {
            sig = (arr: jint[], value: jint) -> jint[],
        },
        pub fn native_set_counter(value: jint) -> void,
        pub fn native_get_message() -> JString,
        static fn native_get_version() -> jint,
        pub static fn native_2d_array_invert {
            sig = (arr: jboolean[][]) -> jboolean[][],
        },
        pub static fn native_string_array_echo(arr: JString[]) -> JString[],
        pub static fn native_2d_string_array_echo(arr: JString[][]) -> JString[][],
    }
}

// Implement the native methods trait
impl TestNativeMethodsNativeInterface for TestNativeMethodsAPI {
    type Error = jni::errors::Error;

    fn native_add<'local>(
        _env: &mut Env<'local>,
        _this: TestNativeMethods<'local>,
        a: jint,
        b: jint,
    ) -> Result<jint, Self::Error> {
        Ok(a + b)
    }

    fn native_log<'local>(
        _env: &mut Env<'local>,
        _this: TestNativeMethods<'local>,
        message: JString<'local>,
    ) -> Result<(), Self::Error> {
        println!("Native log: {}", message);
        Ok(())
    }

    fn native_array_add<'local>(
        env: &mut Env<'local>,
        _this: TestNativeMethods<'local>,
        arr: JPrimitiveArray<'local, jint>,
        value: jint,
    ) -> Result<JPrimitiveArray<'local, jint>, Self::Error> {
        unsafe {
            let mut elem = arr.get_elements(env, jni::elements::ReleaseMode::CopyBack)?;
            for i in 0..elem.len() {
                elem[i] += value;
            }
            elem.commit()?;
        }
        Ok(arr)
    }

    fn native_2d_array_invert<'local>(
        env: &mut Env<'local>,
        _class: JClass<'local>,
        arr: JObjectArray<'local, JPrimitiveArray<'local, jboolean>>,
    ) -> Result<JObjectArray<'local, JPrimitiveArray<'local, jboolean>>, Self::Error> {
        unsafe {
            for i in 0..arr.len(env)? {
                let row = arr.get_element(env, i)?.auto();
                let mut row_elems = row.get_elements(env, jni::elements::ReleaseMode::CopyBack)?;
                for j in 0..row_elems.len() {
                    row_elems[j] = !row_elems[j];
                }
                row_elems.commit()?;
            }
        }
        Ok(arr)
    }

    fn native_set_counter<'local>(
        env: &mut Env<'local>,
        this: TestNativeMethods<'local>,
        value: jint,
    ) -> Result<(), Self::Error> {
        this.set_counter(env, value)
    }

    fn native_get_message<'local>(
        env: &mut Env<'local>,
        this: TestNativeMethods<'local>,
    ) -> Result<JString<'local>, Self::Error> {
        this.get_message(env)
    }

    fn native_get_version<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
    ) -> Result<jint, Self::Error> {
        Ok(100)
    }

    fn native_string_array_echo<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        arr: JObjectArray<'local, JString<'local>>,
    ) -> Result<JObjectArray<'local, JString<'local>>, Self::Error> {
        Ok(arr)
    }

    fn native_2d_string_array_echo<'local>(
        _env: &mut Env<'local>,
        _class: JClass<'local>,
        arr: JObjectArray<'local, JObjectArray<'local, JString<'local>>>,
    ) -> Result<JObjectArray<'local, JObjectArray<'local, JString<'local>>>, Self::Error> {
        Ok(arr)
    }
}

native_method_test! {
    test_name: test_instance_primitive_args_and_return,
    java_class: "com/example/TestNativeMethods.java",
    api: TestNativeMethodsAPI,
    test_body: |env| {
        let obj = TestNativeMethods::new(env)?;

        let result = obj.call_native_add(env, 10, 20)?;
        assert_eq!(result, 30);

        let result = obj.call_native_add(env, -5, 7)?;
        assert_eq!(result, 2);

        Ok(())
    }
}

native_method_test! {
    test_name: test_instance_void_return_string_arg,
    java_class: "com/example/TestNativeMethods.java",
    api: TestNativeMethodsAPI,
    test_body: |env| {
        let obj = TestNativeMethods::new(env)?;

        let message = JString::from_str(env, "test log message")?;
        obj.native_log(env, &message)?;

        Ok(())
    }
}

native_method_test! {
    test_name: test_instance_primitive_array_arg_and_return,
    java_class: "com/example/TestNativeMethods.java",
    api: TestNativeMethodsAPI,
    test_body: |env| {
        let obj = TestNativeMethods::new(env)?;

        let arr = env.new_int_array(5)?;
        let data = [1, 2, 3, 4, 5];
        arr.set_region(env, 0, &data)?;

        let result = obj.native_array_add(env, arr, 10)?;

        let mut result_data = [0; 5];
        result.get_region(env, 0, &mut result_data)?;
        assert_eq!(result_data, [11, 12, 13, 14, 15]);

        Ok(())
    }
}

native_method_test! {
    test_name: test_instance_2d_primitive_array,
    java_class: "com/example/TestNativeMethods.java",
    api: TestNativeMethodsAPI,
    test_body: |env| {
        // Create a 2D boolean array [[true, false], [false, true]]
        let inner1 = env.new_boolean_array(2)?;
        inner1.set_region(env, 0, &[true, false])?;
        let inner2 = env.new_boolean_array(2)?;
        inner2.set_region(env, 0, &[false, true])?;

        let outer = JObjectArray::<JBooleanArray>::new(env, 2, inner1)?;
        outer.set_element(env, 1, inner2)?;

        // Invert the array
        let result = TestNativeMethods::native_2d_array_invert(env, outer)?;

        // Check the result [[false, true], [true, false]]
        let row1 = result.get_element(env, 0)?;
        let mut row1_data = [false; 2];
        row1.get_region(env, 0, &mut row1_data)?;
        assert_eq!(row1_data, [false, true]);

        let row2 = result.get_element(env, 1)?;
        let mut row2_data = [false; 2];
        row2.get_region(env, 0, &mut row2_data)?;
        assert_eq!(row2_data, [true, false]);

        Ok(())
    }
}

native_method_test! {
    test_name: test_instance_call_java_methods,
    java_class: "com/example/TestNativeMethods.java",
    api: TestNativeMethodsAPI,
    test_body: |env| {
        let obj = TestNativeMethods::new(env)?;

        // Test native method can call Java setter (via field access)
        obj.native_set_counter(env, 42)?;
        let counter = obj.get_counter(env)?;
        assert_eq!(counter, 42);

        // Test native method can call Java getter (via method call)
        let message = obj.native_get_message(env)?;
        assert_eq!(message.to_string(), "initial");

        Ok(())
    }
}

native_method_test! {
    test_name: test_static_no_args_primitive_return,
    java_class: "com/example/TestNativeMethods.java",
    api: TestNativeMethodsAPI,
    test_body: |env| {
        let version = TestNativeMethods::call_native_get_version(env)?;
        assert_eq!(version, 100);

        Ok(())
    }
}

native_method_test! {
    test_name: test_static_string_array_arg_and_return,
    java_class: "com/example/TestNativeMethods.java",
    api: TestNativeMethodsAPI,
    test_body: |env| {
        let str1 = JString::from_str(env, "hello")?;
        let str2 = JString::from_str(env, "world")?;

        let arr = JObjectArray::<JString>::new(env, 2, &str1)?;
        arr.set_element(env, 1, &str2)?;

        let result = TestNativeMethods::native_string_array_echo(env, arr)?;

        let result1 = result.get_element(env, 0)?;
        assert_eq!(result1.to_string(), "hello");

        let result2 = result.get_element(env, 1)?;
        assert_eq!(result2.to_string(), "world");

        Ok(())
    }
}

native_method_test! {
    test_name: test_static_2d_string_array,
    java_class: "com/example/TestNativeMethods.java",
    api: TestNativeMethodsAPI,
    test_body: |env| {
        let str1 = JString::from_str(env, "foo")?;
        let str2 = JString::from_str(env, "bar")?;
        let str3 = JString::from_str(env, "baz")?;
        let str4 = JString::from_str(env, "qux")?;

        let row1 = JObjectArray::<JString>::new(env, 2, &str1)?;
        row1.set_element(env, 1, &str2)?;

        let row2 = JObjectArray::<JString>::new(env, 2, &str3)?;
        row2.set_element(env, 1, &str4)?;

        let arr = JObjectArray::<JObjectArray<JString>>::new(env, 2, &row1)?;
        arr.set_element(env, 1, &row2)?;

        // Call native method
        let result = TestNativeMethods::native_2d_string_array_echo(env, arr)?;

        let result_row1 = result.get_element(env, 0)?;
        let r1c1 = result_row1.get_element(env, 0)?;
        let r1c2 = result_row1.get_element(env, 1)?;
        assert_eq!(r1c1.to_string(), "foo");
        assert_eq!(r1c2.to_string(), "bar");

        let result_row2 = result.get_element(env, 1)?;
        let r2c1 = result_row2.get_element(env, 0)?;
        let r2c2 = result_row2.get_element(env, 1)?;
        assert_eq!(r2c1.to_string(), "baz");
        assert_eq!(r2c2.to_string(), "qux");

        Ok(())
    }
}
