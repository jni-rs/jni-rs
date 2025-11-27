#![allow(unused)]
mod native_methods_utils;
mod util;

use jni::Env;
use jni::native_method;
use jni::objects::JBooleanArray;
use jni::objects::{JClass, JObjectArray, JPrimitiveArray, JString};
use jni::refs::IntoAuto as _;
use jni::sys::{jboolean, jint};
use rusty_fork::rusty_fork_test;

// ====================================================================================
// Native method implementations
// ====================================================================================

fn native_add_impl<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
    a: jint,
    b: jint,
) -> Result<jint, jni::errors::Error> {
    Ok(a + b)
}

fn native_log_impl<'local>(
    _env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
    message: JString<'local>,
) -> Result<(), jni::errors::Error> {
    println!("Native log: {}", message);
    Ok(())
}

fn native_array_add_impl<'local>(
    env: &mut Env<'local>,
    _this: jni::objects::JObject<'local>,
    arr: JPrimitiveArray<'local, jint>,
    value: jint,
) -> Result<JPrimitiveArray<'local, jint>, jni::errors::Error> {
    unsafe {
        let mut elem = arr.get_elements(env, jni::elements::ReleaseMode::CopyBack)?;
        for i in 0..elem.len() {
            elem[i] += value;
        }
        elem.commit()?;
    }
    Ok(arr)
}

fn native_2d_array_invert_impl<'local>(
    env: &mut Env<'local>,
    _class: JClass<'local>,
    arr: JObjectArray<'local, JPrimitiveArray<'local, jboolean>>,
) -> Result<JObjectArray<'local, JPrimitiveArray<'local, jboolean>>, jni::errors::Error> {
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

fn native_set_counter_impl<'local>(
    env: &mut Env<'local>,
    this: jni::objects::JObject<'local>,
    value: jint,
) -> Result<(), jni::errors::Error> {
    use jni::{jni_sig, jni_str};
    env.call_method(
        this,
        jni_str!("setCounter"),
        jni_str!("(I)V"),
        //jni_sig!("(I)V"), TODO
        &[jni::objects::JValue::Int(value)],
    )?;
    Ok(())
}

fn native_get_message_impl<'local>(
    env: &mut Env<'local>,
    this: jni::objects::JObject<'local>,
) -> Result<JString<'local>, jni::errors::Error> {
    use jni::{jni_sig, jni_str};
    let result = env.call_method(
        this,
        jni_str!("getMessage"),
        jni_str!("()Ljava/lang/String;"),
        //jni_sig!("()Ljava/lang/String;"), TODO
        &[],
    )?;
    // Use cast_local to safely convert JObject to JString
    let result = JString::cast_local(result.l()?, env)?;
    Ok(result)
}

fn native_get_version_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
) -> Result<jint, jni::errors::Error> {
    Ok(100)
}

fn native_string_array_echo_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    arr: JObjectArray<'local, JString<'local>>,
) -> Result<JObjectArray<'local, JString<'local>>, jni::errors::Error> {
    Ok(arr)
}

fn native_2d_string_array_echo_impl<'local>(
    _env: &mut Env<'local>,
    _class: JClass<'local>,
    arr: JObjectArray<'local, JObjectArray<'local, JString<'local>>>,
) -> Result<JObjectArray<'local, JObjectArray<'local, JString<'local>>>, jni::errors::Error> {
    Ok(arr)
}

// ====================================================================================
// Test: Instance native method with primitive args and return
// ====================================================================================

native_method_test! {
    test_name: test_instance_primitive_args_and_return,
    java_class: "com/example/TestNativeMethods.java",
    methods: |class| &[
        native_method! {
            fn native_add(a: jint, b: jint) -> jint,
            fn = native_add_impl,
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        // Call via Java wrapper method
        use jni::{jni_sig, jni_str};
        let result = env.call_method(
            &obj,
            jni_str!("callNativeAdd"),
            jni_str!("(II)I"),
            //jni_sig!("(II)I"), TODO
            &[jni::objects::JValue::Int(10), jni::objects::JValue::Int(20)],
        )?.i()?;
        assert_eq!(result, 30);

        let result = env.call_method(
            &obj,
            jni_str!("callNativeAdd"),
            jni_str!("(II)I"),
            //jni_sig!("(II)I"), TODO
            &[jni::objects::JValue::Int(-5), jni::objects::JValue::Int(7)],
        )?.i()?;
        assert_eq!(result, 2);

        Ok(())
    }
}

// ====================================================================================
// Test: Instance native method with void return and String arg
// ====================================================================================

native_method_test! {
    test_name: test_instance_void_return_string_arg,
    java_class: "com/example/TestNativeMethods.java",
    methods: |class| &[
        native_method! {
            fn native_log(message: JString) -> void,
            fn = native_log_impl,
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        let message = JString::from_str(env, "test log message")?;

        use jni::{jni_sig, jni_str};
        env.call_method(
            &obj,
            jni_str!("nativeLog"),
            jni_str!("(Ljava/lang/String;)V"),
            //jni_sig!("(Ljava/lang/String;)V"), TODO
            &[jni::objects::JValue::Object(&message)],
        )?;

        Ok(())
    }
}

// ====================================================================================
// Test: Instance native method with primitive array arg and return
// ====================================================================================

native_method_test! {
    test_name: test_instance_primitive_array_arg_and_return,
    java_class: "com/example/TestNativeMethods.java",
    methods: |class| &[
        native_method! {
            fn native_array_add(arr: jint[], value: jint) -> jint[],
            fn = native_array_add_impl,
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        let arr = env.new_int_array(5)?;
        let data = [1, 2, 3, 4, 5];
        arr.set_region(env, 0, &data)?;

        use jni::{jni_sig, jni_str};
        let result = env.call_method(
            &obj,
            jni_str!("nativeArrayAdd"),
            jni_str!("([II)[I"),
            //jni_sig!("([II)[I"), TODO
            &[jni::objects::JValue::Object(&arr), jni::objects::JValue::Int(10)],
        )?.l()?;
        // Use cast_local to safely convert JObject to JPrimitiveArray
        let result = JPrimitiveArray::<jint>::cast_local(env, result)?;

        let mut result_data = [0; 5];
        result.get_region(env, 0, &mut result_data)?;
        assert_eq!(result_data, [11, 12, 13, 14, 15]);

        Ok(())
    }
}

// ====================================================================================
// Test: Instance native method with 2D primitive array
// ====================================================================================

native_method_test! {
    test_name: test_instance_2d_primitive_array,
    java_class: "com/example/TestNativeMethods.java",
    methods: |class| &[
        native_method! {
            static fn native_2d_array_invert(arr: jboolean[][]) -> jboolean[][],
            fn = native_2d_array_invert_impl,
        },
    ],
    test_body: |env, class| {
        // Create a 2D boolean array [[true, false], [false, true]]
        let inner1 = env.new_boolean_array(2)?;
        inner1.set_region(env, 0, &[true, false])?;
        let inner2 = env.new_boolean_array(2)?;
        inner2.set_region(env, 0, &[false, true])?;

        let outer = JObjectArray::<JBooleanArray>::new(env, 2, inner1)?;
        outer.set_element(env, 1, inner2)?;

        // Invert the array
        use jni::{jni_sig, jni_str};
        let result = env.call_static_method(
            class,
            jni_str!("native2DArrayInvert"),
            jni_str!("([[Z)[[Z"),
            //jni_sig!("([[Z)[[Z"), TODO
            &[jni::objects::JValue::Object(&outer)],
        )?.l()?;
        // Use cast_local to safely convert JObject to JObjectArray
        let result = JObjectArray::<JBooleanArray>::cast_local(env, result)?;

        // Check the result [[false, true], [true, false]]
        let row1: JBooleanArray = result.get_element(env, 0)?;
        let mut row1_data = [false; 2];
        row1.get_region(env, 0, &mut row1_data)?;
        assert_eq!(row1_data, [false, true]);

        let row2: JBooleanArray = result.get_element(env, 1)?;
        let mut row2_data = [false; 2];
        row2.get_region(env, 0, &mut row2_data)?;
        assert_eq!(row2_data, [true, false]);

        Ok(())
    }
}

// ====================================================================================
// Test: Instance native method can call Java methods
// ====================================================================================

native_method_test! {
    test_name: test_instance_call_java_methods,
    java_class: "com/example/TestNativeMethods.java",
    methods: |class| &[
        native_method! {
            fn native_set_counter(value: jint) -> void,
            fn = native_set_counter_impl,
        },
        native_method! {
            fn native_get_message() -> JString,
            fn = native_get_message_impl,
        },
    ],
    test_body: |env, class| {
        let obj = new_object!(env, class)?;

        // Test native method can call Java setter (via method call)
        use jni::{jni_sig, jni_str};
        env.call_method(
            &obj,
            jni_str!("nativeSetCounter"),
            jni_str!("(I)V"),
            //jni_sig!("(I)V"), TODO
            &[jni::objects::JValue::Int(42)],
        )?;

        let counter = env.call_method(&obj, jni_str!("getCounter"), jni_str!("()I"), &[])?.i()?;
        //let counter = env.call_method(&obj, jni_str!("getCounter"), jni_sig!("()I"), &[])?.i()?;
        assert_eq!(counter, 42);

        // Test native method can call Java getter (via method call)
        let message = env.call_method(
            &obj,
            jni_str!("nativeGetMessage"),
            jni_str!("()Ljava/lang/String;"),
            //jni_sig!("()Ljava/lang/String;"), TODO
            &[],
        )?.l()?;
        // Use cast_local to safely convert JObject to JString
        let message = JString::cast_local(message, env)?;
        assert_eq!(message.to_string(), "initial");

        Ok(())
    }
}

// ====================================================================================
// Test: Static native method with no args and primitive return
// ====================================================================================

native_method_test! {
    test_name: test_static_no_args_primitive_return,
    java_class: "com/example/TestNativeMethods.java",
    methods: |class| &[
        native_method! {
            static fn native_get_version() -> jint,
            fn = native_get_version_impl,
        },
    ],
    test_body: |env, class| {
        use jni::{jni_sig, jni_str};
        let version = env.call_static_method(
            class,
            jni_str!("callNativeGetVersion"),
            jni_str!("()I"),
            //jni_sig!("()I"), TODO
            &[],
        )?.i()?;
        assert_eq!(version, 100);

        Ok(())
    }
}

// ====================================================================================
// Test: Static native method with String array arg and return
// ====================================================================================

native_method_test! {
    test_name: test_static_string_array_arg_and_return,
    java_class: "com/example/TestNativeMethods.java",
    methods: |class| &[
        native_method! {
            static fn native_string_array_echo(arr: JString[]) -> JString[],
            fn = native_string_array_echo_impl,
        },
    ],
    test_body: |env, class| {
        let str1 = JString::from_str(env, "hello")?;
        let str2 = JString::from_str(env, "world")?;

        let arr = JObjectArray::<JString>::new(env, 2, &str1)?;
        arr.set_element(env, 1, &str2)?;

        use jni::{jni_sig, jni_str};
        let result = env.call_static_method(
            class,
            jni_str!("nativeStringArrayEcho"),
            jni_str!("([Ljava/lang/String;)[Ljava/lang/String;"),
            //jni_sig!("([Ljava/lang/String;)[Ljava/lang/String;"), TODO
            &[jni::objects::JValue::Object(&arr)],
        )?.l()?;
        // Use cast_local to safely convert JObject to JObjectArray
        let result = JObjectArray::<JString>::cast_local(env, result)?;

        let result1: JString = result.get_element(env, 0)?;
        assert_eq!(result1.to_string(), "hello");

        let result2: JString = result.get_element(env, 1)?;
        assert_eq!(result2.to_string(), "world");

        Ok(())
    }
}

// ====================================================================================
// Test: Static native method with 2D String array
// ====================================================================================

native_method_test! {
    test_name: test_static_2d_string_array,
    java_class: "com/example/TestNativeMethods.java",
    methods: |class| &[
        native_method! {
            static fn native_2d_string_array_echo(arr: JString[][]) -> JString[][],
            fn = native_2d_string_array_echo_impl,
        },
    ],
    test_body: |env, class| {
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
        use jni::{jni_sig, jni_str};
        let result = env.call_static_method(
            class,
            jni_str!("native2DStringArrayEcho"),
            jni_str!("([[Ljava/lang/String;)[[Ljava/lang/String;"),
            //jni_sig!("([[Ljava/lang/String;)[[Ljava/lang/String;"), TODO
            &[jni::objects::JValue::Object(&arr)],
        )?.l()?;
        let result = JObjectArray::<JObjectArray<JString>>::cast_local(env, result)?;

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
