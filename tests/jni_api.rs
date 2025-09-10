#![cfg(feature = "invocation")]
use std::convert::TryFrom;

use assert_matches::assert_matches;

use jni::{
    descriptors::Desc,
    errors::{CharToJavaError, Error},
    objects::{
        AutoElements, IntoAuto as _, JByteBuffer, JList, JObject, JObjectRef as _, JString,
        JThrowable, JValue, ReleaseMode, Weak,
    },
    signature::{JavaType, Primitive, ReturnType},
    strings::{JNIStr, JNIString},
    sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jobject, jshort, jsize},
    Env,
};

mod util;
use util::{attach_current_thread, unwrap};

use rusty_fork::rusty_fork_test;

static ARRAYLIST_CLASS: &JNIStr = JNIStr::from_cstr(c"java/util/ArrayList");
static EXCEPTION_CLASS: &JNIStr = JNIStr::from_cstr(c"java/lang/Exception");
static ARITHMETIC_EXCEPTION_CLASS: &JNIStr = JNIStr::from_cstr(c"java/lang/ArithmeticException");
static RUNTIME_EXCEPTION_CLASS: &JNIStr = JNIStr::from_cstr(c"java/lang/RuntimeException");
static INTEGER_CLASS: &JNIStr = JNIStr::from_cstr(c"java/lang/Integer");
static MATH_CLASS: &JNIStr = JNIStr::from_cstr(c"java/lang/Math");
static STRING_CLASS: &JNIStr = JNIStr::from_cstr(c"java/lang/String");
static MATH_ABS_METHOD_NAME: &JNIStr = JNIStr::from_cstr(c"abs");
static MATH_TO_INT_METHOD_NAME: &JNIStr = JNIStr::from_cstr(c"toIntExact");
static MATH_ABS_SIGNATURE: &JNIStr = JNIStr::from_cstr(c"(I)I");
static MATH_TO_INT_SIGNATURE: &JNIStr = JNIStr::from_cstr(c"(J)I");
static TEST_EXCEPTION_MESSAGE: &JNIStr = JNIStr::from_cstr(c"Default exception thrown");
static TESTING_OBJECT_STR: &JNIStr = JNIStr::from_cstr(c"TESTING OBJECT");

#[test]
pub fn call_method_returning_null() {
    attach_current_thread(|env| {
        // Create an Exception with no message
        let obj = unwrap(env.new_object(EXCEPTION_CLASS, c"()V", &[]), env).auto();
        // Call Throwable#getMessage must return null
        let message = unwrap(
            env.call_method(&obj, c"getMessage", c"()Ljava/lang/String;", &[]),
            env,
        );
        let message_ref = unwrap(message.l(), env).auto();

        assert!(message_ref.is_null());
        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_instance_of_same_class() {
    attach_current_thread(|env| {
        let obj = unwrap(env.new_object(EXCEPTION_CLASS, c"()V", &[]), env).auto();
        assert!(unwrap(env.is_instance_of(&obj, EXCEPTION_CLASS), env));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_instance_of_superclass() {
    attach_current_thread(|env| {
        let obj = unwrap(env.new_object(ARITHMETIC_EXCEPTION_CLASS, c"()V", &[]), env).auto();
        assert!(unwrap(env.is_instance_of(&obj, EXCEPTION_CLASS), env));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_instance_of_subclass() {
    attach_current_thread(|env| {
        let obj = unwrap(env.new_object(EXCEPTION_CLASS, c"()V", &[]), env).auto();
        assert!(!unwrap(
            env.is_instance_of(&obj, ARITHMETIC_EXCEPTION_CLASS),
            env,
        ));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_instance_of_not_superclass() {
    attach_current_thread(|env| {
        let obj = unwrap(env.new_object(ARITHMETIC_EXCEPTION_CLASS, c"()V", &[]), env).auto();
        assert!(!unwrap(env.is_instance_of(&obj, ARRAYLIST_CLASS), env));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_instance_of_null() {
    attach_current_thread(|env| {
        let obj = JObject::null();
        assert!(unwrap(env.is_instance_of(&obj, ARRAYLIST_CLASS), env));
        assert!(unwrap(env.is_instance_of(&obj, EXCEPTION_CLASS), env));
        assert!(unwrap(
            env.is_instance_of(&obj, ARITHMETIC_EXCEPTION_CLASS),
            env,
        ));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_same_object_diff_references() {
    attach_current_thread(|env| {
        let string = env.new_string(TESTING_OBJECT_STR).unwrap();
        let ref_from_string = unwrap(env.new_local_ref(&string), env);
        assert!(env.is_same_object(&string, &ref_from_string));
        env.delete_local_ref(ref_from_string);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_same_object_same_reference() {
    attach_current_thread(|env| {
        let string = env.new_string(TESTING_OBJECT_STR).unwrap();
        assert!(env.is_same_object(&string, &string));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_not_same_object() {
    attach_current_thread(|env| {
        let string = env.new_string(TESTING_OBJECT_STR).unwrap();
        let same_src_str = env.new_string(TESTING_OBJECT_STR).unwrap();
        assert!(!env.is_same_object(string, same_src_str));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn is_not_same_object_null() {
    attach_current_thread(|env| {
        assert!(env.is_same_object(JObject::null(), JObject::null()));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_public_field() {
    attach_current_thread(|env| {
        // Create a new Point(5, 10)
        let point = unwrap(
            env.new_object(
                c"java/awt/Point",
                c"(II)V",
                &[JValue::Int(5), JValue::Int(10)],
            ),
            env,
        )
        .auto();

        // Get the x field value
        let x_value = env.get_field(&point, c"x", c"I").unwrap().i().unwrap();

        assert_eq!(x_value, 5);

        // Get the y field value
        let y_value = env.get_field(&point, c"y", c"I").unwrap().i().unwrap();

        assert_eq!(y_value, 10);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_public_field_by_id() {
    attach_current_thread(|env| {
        // Create a new Point(5, 10)
        let point = unwrap(
            env.new_object(
                c"java/awt/Point",
                c"(II)V",
                &[JValue::Int(5), JValue::Int(10)],
            ),
            env,
        )
        .auto();

        // Get the field ID for x field
        let field_type = c"I";
        let field_id = env
            .get_field_id(c"java/awt/Point", c"x", field_type)
            .unwrap();

        let field_type = ReturnType::Primitive(Primitive::Int);
        // Safety: we have just looked up the field ID based on the given class and field_type
        unsafe {
            let x_value = env
                .get_field_unchecked(&point, field_id, field_type)
                .unwrap()
                .i()
                .unwrap();

            assert_eq!(x_value, 5);
        }

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn set_public_field() {
    attach_current_thread(|env| {
        // Create a new Point(5, 10)
        let point = unwrap(
            env.new_object(
                c"java/awt/Point",
                c"(II)V",
                &[JValue::Int(5), JValue::Int(10)],
            ),
            env,
        )
        .auto();

        // Set the x field to a new value
        env.set_field(&point, c"x", c"I", JValue::Int(15)).unwrap();

        // Verify the field was set
        let x_value = env.get_field(&point, c"x", c"I").unwrap().i().unwrap();

        assert_eq!(x_value, 15);

        // Set the y field to a new value
        env.set_field(&point, c"y", c"I", JValue::Int(25)).unwrap();

        // Verify the field was set
        let y_value = env.get_field(&point, c"y", c"I").unwrap().i().unwrap();

        assert_eq!(y_value, 25);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn set_public_field_by_id() {
    attach_current_thread(|env| {
        // Create a new Point(5, 10)
        let point = unwrap(
            env.new_object(
                c"java/awt/Point",
                c"(II)V",
                &[JValue::Int(5), JValue::Int(10)],
            ),
            env,
        )
        .auto();

        // Get the field ID for x field
        let field_type = c"I";
        let field_id = env
            .get_field_id(c"java/awt/Point", c"x", field_type)
            .unwrap();

        // Set the x field using the field ID
        // Safety: we have just looked up the field ID based on the given field name and type
        unsafe {
            env.set_field_unchecked(&point, field_id, JValue::Int(15))
                .unwrap();
        }

        // Verify the field was set
        let x_value = env.get_field(&point, c"x", c"I").unwrap().i().unwrap();

        assert_eq!(x_value, 15);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_static_public_field() {
    attach_current_thread(|env| {
        let min_int_value = env
            .get_static_field(INTEGER_CLASS, c"MIN_VALUE", c"I")
            .unwrap()
            .i()
            .unwrap();

        assert_eq!(min_int_value, i32::MIN);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_static_public_field_by_id() {
    attach_current_thread(|env| {
        // One can't pass a JavaType::Primitive(Primitive::Int) to
        //   `get_static_field_id` unfortunately: #137
        let field_type = c"I";
        let field_id = env
            .get_static_field_id(INTEGER_CLASS, c"MIN_VALUE", field_type)
            .unwrap();

        let field_type = JavaType::Primitive(Primitive::Int);
        // Safety: we have just looked up the field ID based on the given class and field_type
        unsafe {
            let min_int_value = env
                .get_static_field_unchecked(INTEGER_CLASS, field_id, field_type)
                .unwrap()
                .i()
                .unwrap();

            assert_eq!(min_int_value, i32::MIN);
        }

        Ok(())
    })
    .unwrap();
}

rusty_fork_test! {
#[test]
fn set_static_public_field() {
    attach_current_thread(|env| {
        // We'll use System.in which is a mutable static field

        // Get the original System.in value
        let original_in = env
            .get_static_field(c"java/lang/System", c"in", c"Ljava/io/InputStream;")
            .unwrap()
            .l()
            .unwrap();

        // Create a new ByteArrayInputStream as a different InputStream
        let byte_array = env.new_byte_array(10).unwrap();
        let new_input_stream = env
            .new_object(
                c"java/io/ByteArrayInputStream",
                c"([B)V",
                &[JValue::from(&byte_array)],
            )
            .unwrap();

        // Set System.in to our new ByteArrayInputStream
        env.set_static_field(
            c"java/lang/System",
            c"in",
            c"Ljava/io/InputStream;",
            JValue::from(&new_input_stream),
        )
        .unwrap();

        // Verify the field was set by getting it again and checking it's no longer null
        let current_in = env
            .get_static_field(c"java/lang/System", c"in", c"Ljava/io/InputStream;")
            .unwrap()
            .l()
            .unwrap();

        // The field should not be null after setting
        assert!(!current_in.is_null());

        // Restore the original System.in
        env.set_static_field(
            c"java/lang/System",
            c"in",
            c"Ljava/io/InputStream;",
            JValue::from(&original_in),
        )
        .unwrap();

        // Verify restoration worked - the field should still not be null
        let restored_in = env
            .get_static_field(c"java/lang/System", c"in", c"Ljava/io/InputStream;")
            .unwrap()
            .l()
            .unwrap();

        assert!(!restored_in.is_null());

        Ok(())
    })
    .unwrap();
}
}

rusty_fork_test! {
#[test]
fn set_static_public_field_by_id() {
    attach_current_thread(|env| {
        // Get the original System.in value
        let original_in = env
            .get_static_field(c"java/lang/System", c"in", c"Ljava/io/InputStream;")
            .unwrap()
            .l()
            .unwrap();

        // Get the field ID for System.in
        let field_type = c"Ljava/io/InputStream;";
        let field_id = env
            .get_static_field_id(c"java/lang/System", c"in", field_type)
            .unwrap();

        // Create a new ByteArrayInputStream as a different InputStream
        let byte_array = env.new_byte_array(10).unwrap();
        let new_input_stream = env
            .new_object(
                c"java/io/ByteArrayInputStream",
                c"([B)V",
                &[JValue::from(&byte_array)],
            )
            .unwrap();

        // Set System.in to our new ByteArrayInputStream using the field ID
        // Safety: we have just looked up the field ID based on the given field name and type
        unsafe {
            env.set_static_field_unchecked(
                c"java/lang/System",
                field_id,
                JValue::from(&new_input_stream),
            )
            .unwrap();
        }

        // Verify the field was set by getting it again (this ensures the set operation worked)
        let current_in = env
            .get_static_field(c"java/lang/System", c"in", c"Ljava/io/InputStream;")
            .unwrap()
            .l()
            .unwrap();

        // Verify that we can successfully retrieve a non-null input stream
        assert!(!current_in.is_null());

        // Restore the original System.in using the unchecked method
        // Safety: we have the correct field ID and value type
        unsafe {
            env.set_static_field_unchecked(
                c"java/lang/System",
                field_id,
                JValue::from(&original_in),
            )
            .unwrap();
        }

        // Verify restoration worked
        let restored_in = env
            .get_static_field(c"java/lang/System", c"in", c"Ljava/io/InputStream;")
            .unwrap()
            .l()
            .unwrap();

        // The restored value should be the same as original
        assert!(env.is_same_object(&original_in, &restored_in));

        Ok(())
    })
    .unwrap();
}
}

/*
#[test]
pub fn pop_local_frame_pending_exception() {
attach_current_thread(|env| {
        env.push_local_frame(16).unwrap();

        env.throw_new(RUNTIME_EXCEPTION_CLASS, "Test Exception")
            .unwrap();

        // Pop the local frame with a pending exception
        unsafe { env.pop_local_frame(&JObject::null()) }
            .expect("Env#pop_local_frame must work in case of pending exception");

        env.exception_clear();

        Ok(())
    }).unwrap();
    }

#[test]
pub fn push_local_frame_pending_exception() {
attach_current_thread(|env| {
        env.throw_new(RUNTIME_EXCEPTION_CLASS, "Test Exception")
            .unwrap();

        // Push a new local frame with a pending exception
        env.push_local_frame(16)
            .expect("Env#push_local_frame must work in case of pending exception");

        env.exception_clear();

        unsafe { env.pop_local_frame(&JObject::null()) }.unwrap();

        Ok(())
    }).unwrap();
    }

#[test]
pub fn push_local_frame_too_many_refs() {
attach_current_thread(|env| {
        // Try to push a new local frame with a ridiculous size
        let frame_size = i32::MAX;
        env.push_local_frame(frame_size)
            .expect_err("push_local_frame(2B) must Err");

        unsafe { env.pop_local_frame(&JObject::null()) }.unwrap();

        Ok(())
    }).unwrap();
    }
*/

#[test]
pub fn with_local_frame() {
    attach_current_thread(|env| {
        let s = env
            .with_local_frame_returning_local::<_, JObject, jni::errors::Error>(16, |env| {
                let res = env.new_string(c"Test")?;
                Ok(res.into())
            })
            .unwrap();
        let s = env.cast_local::<JString>(s).unwrap();

        let s = s
            .mutf8_chars(env)
            .expect("The object returned from the local frame must remain valid");
        assert_eq!(s.to_str(), "Test");

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn with_local_frame_pending_exception() {
    attach_current_thread(|env| {
        env.throw_new(RUNTIME_EXCEPTION_CLASS, c"Test Exception")
            .unwrap();

        // Try to allocate a frame of locals
        env.with_local_frame(16, |_| -> Result<_, Error> { Ok(()) })
            .expect("Env#with_local_frame must work in case of pending exception");

        env.exception_clear();

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn call_method_ok() {
    attach_current_thread(|env| {
        let s = env.new_string(TESTING_OBJECT_STR).unwrap();

        let v: jint = env
            .call_method(s, c"indexOf", c"(I)I", &[JValue::Int('S' as i32)])
            .expect("Env#call_method should return JValue")
            .i()
            .unwrap();

        assert_eq!(v, 2);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn call_method_with_bad_args_errs() {
    attach_current_thread(|env| {
        let s = env.new_string(TESTING_OBJECT_STR).unwrap();

        let is_bad_typ = env
            .call_method(
                &s,
                c"indexOf",
                c"(I)I",
                &[JValue::Float(std::f32::consts::PI)],
            )
            .map_err(|error| matches!(error, Error::InvalidArgList(_)))
            .expect_err("Env#callmethod with bad arg type should err");

        assert!(
            is_bad_typ,
            "ErrorKind::InvalidArgList expected when passing bad value type"
        );

        let is_bad_len = env
            .call_method(
                &s,
                c"indexOf",
                c"(I)I",
                &[JValue::Int('S' as i32), JValue::Long(3)],
            )
            .map_err(|error| matches!(error, Error::InvalidArgList(_)))
            .expect_err("Env#call_method with bad arg lengths should err");

        assert!(
            is_bad_len,
            "ErrorKind::InvalidArgList expected when passing bad argument lengths"
        );

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn call_static_method_ok() {
    attach_current_thread(|env| {
        let x = JValue::from(-10);
        let val: jint = env
            .call_static_method(MATH_CLASS, MATH_ABS_METHOD_NAME, MATH_ABS_SIGNATURE, &[x])
            .expect("Env#call_static_method should return JValue")
            .i()
            .unwrap();

        assert_eq!(val, 10);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn call_static_method_unchecked_ok() {
    attach_current_thread(|env| {
        let x = JValue::from(-10);
        let math_class = env.find_class(MATH_CLASS).unwrap();
        let abs_method_id = env
            .get_static_method_id(&math_class, MATH_ABS_METHOD_NAME, MATH_ABS_SIGNATURE)
            .unwrap();
        let val: jint = unsafe {
            env.call_static_method_unchecked(
                &math_class,
                abs_method_id,
                ReturnType::Primitive(Primitive::Int),
                &[x.as_jni()],
            )
        }
        .expect("Env#call_static_method_unchecked should return JValue")
        .i()
        .unwrap();

        assert_eq!(val, 10);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn call_new_object_unchecked_ok() {
    attach_current_thread(|env| {
        let test_str = env.new_string(TESTING_OBJECT_STR).unwrap();
        let string_class = env.find_class(STRING_CLASS).unwrap();

        let ctor_method_id = env
            .get_method_id(&string_class, c"<init>", c"(Ljava/lang/String;)V")
            .unwrap();
        let val: JObject = unsafe {
            env.new_object_unchecked(
                &string_class,
                ctor_method_id,
                &[JValue::from(&test_str).as_jni()],
            )
        }
        .expect("Env#new_object_unchecked should return JValue");

        let jstr = env.cast_local::<JString>(val).unwrap();
        let javastr = jstr.mutf8_chars(env).unwrap();
        let jnistr: &JNIStr = javastr.as_ref();
        assert_eq!(jnistr, TESTING_OBJECT_STR);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn call_new_object_with_bad_args_errs() {
    attach_current_thread(|env| {
        let string_class = env.find_class(STRING_CLASS).unwrap();

        let is_bad_typ = env
            .new_object(&string_class, c"(Ljava/lang/String;)V", &[JValue::Int(2)])
            .map_err(|error| matches!(error, Error::InvalidArgList(_)))
            .expect_err("Env#new_object with bad arg type should err");

        assert!(
            is_bad_typ,
            "ErrorKind::InvalidArgList expected when passing bad value type"
        );

        let s = env.new_string(TESTING_OBJECT_STR).unwrap();

        let is_bad_len = env
            .new_object(
                &string_class,
                c"(Ljava/lang/String;)V",
                &[JValue::from(&s), JValue::Int(2)],
            )
            .map_err(|error| matches!(error, Error::InvalidArgList(_)))
            .expect_err("Env#new_object with bad arg type should err");

        assert!(
            is_bad_len,
            "ErrorKind::InvalidArgList expected when passing bad argument lengths"
        );

        Ok(())
    })
    .unwrap();
}

/// Check that we get a runtime error if trying to instantiate with an array class.
///
/// Although the JNI spec for `NewObjectA` states that the class "must not refer to an array class"
/// (and could therefor potentially trigger undefined behaviour if that rule is violated) we
/// expect that `Env::new_object()` shouldn't ever get as far as calling `NewObjectA` since
/// it will first fail (with a safe, runtime error) to lookup a method ID for any constructor.
/// (consistent with how [getConstructors()](https://docs.oracle.com/en/java/javase/17/docs/api/java.base/java/lang/Class.html#getConstructors())
/// doesn't expose constructors for array classes)
#[test]
pub fn call_new_object_with_array_class() {
    attach_current_thread(|env| {
        let byte_array = env.new_byte_array(16).unwrap();
        let array_class = env.get_object_class(byte_array).unwrap();
        // We just make up a plausible constructor signature
        let result = env.new_object(&array_class, c"(I)[B", &[JValue::Int(16)]);

        assert!(result.is_err());

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn call_static_method_throws() {
    attach_current_thread(|env| {
        let x = JValue::Long(4_000_000_000);
        let is_java_exception = env
            .call_static_method(
                MATH_CLASS,
                MATH_TO_INT_METHOD_NAME,
                MATH_TO_INT_SIGNATURE,
                &[x],
            )
            .map_err(|error| matches!(error, Error::JavaException))
            .expect_err("Env#call_static_method_unsafe should return error");

        // Throws a java.lang.ArithmeticException: integer overflow
        assert!(
            is_java_exception,
            "ErrorKind::JavaException expected as error"
        );
        assert_pending_java_exception(env);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn call_static_method_with_bad_args_errs() {
    attach_current_thread(|env| {
        let x = JValue::Double(4.567_891_23);
        let is_bad_typ = env
            .call_static_method(
                MATH_CLASS,
                MATH_TO_INT_METHOD_NAME,
                MATH_TO_INT_SIGNATURE,
                &[x],
            )
            .map_err(|error| matches!(error, Error::InvalidArgList(_)))
            .expect_err("Env#call_static_method with bad arg type should err");

        assert!(
            is_bad_typ,
            "ErrorKind::InvalidArgList expected when passing bad value type"
        );

        let is_bad_len = env
            .call_static_method(
                MATH_CLASS,
                MATH_TO_INT_METHOD_NAME,
                MATH_TO_INT_SIGNATURE,
                &[JValue::Int(2), JValue::Int(3)],
            )
            .map_err(|error| matches!(error, Error::InvalidArgList(_)))
            .expect_err("Env#call_static_method with bad arg lengths should err");

        assert!(
            is_bad_len,
            "ErrorKind::InvalidArgList expected when passing bad argument lengths"
        );

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_reflected_method_from_id() {
    attach_current_thread(|env| {
        let ctor_method_id = env
            .get_method_id(INTEGER_CLASS, c"<init>", c"(Ljava/lang/String;)V")
            .expect("constructor from string exists");
        let ctor = env
            .to_reflected_method(INTEGER_CLASS, ctor_method_id)
            .unwrap();

        let value = {
            let jstr = env.new_string(c"55").unwrap();
            let vargs = env
                .new_object_array(1, c"java/lang/Object", jstr)
                .expect("can create array");

            env.call_method(
                ctor,
                c"newInstance",
                c"([Ljava/lang/Object;)Ljava/lang/Object;",
                &[JValue::from(&vargs)],
            )
            .expect("can invoke Constructor.newInstance")
            .l()
            .expect("return value is an Integer")
        };

        let int_value = env
            .call_method(value, c"intValue", c"()I", &[])
            .unwrap()
            .i()
            .unwrap();
        assert_eq!(int_value, 55);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_reflected_static_method_from_id() {
    attach_current_thread(|env| {
        let x = JValue::from(-10);
        let math_class = env.find_class(MATH_CLASS).unwrap();
        let abs_method_id = env
            .get_static_method_id(&math_class, MATH_ABS_METHOD_NAME, MATH_ABS_SIGNATURE)
            .unwrap();

        let abs_method = env
            .to_reflected_static_method(&math_class, abs_method_id)
            .unwrap();

        let arg = env.new_object(INTEGER_CLASS, c"(I)V", &[x]).unwrap();
        let vargs = env.new_object_array(1, c"java/lang/Object", arg).unwrap();
        let val = env
            .call_method(
                abs_method,
                c"invoke",
                c"(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
                &[JValue::from(&JObject::null()), JValue::from(&vargs)],
            )
            .expect("can call Method.invoke")
            .l()
            .expect("return value is an int");
        let val = env
            .call_method(val, c"intValue", c"()I", &[])
            .unwrap()
            .i()
            .unwrap();

        assert_eq!(val, 10);
        Ok(())
    })
    .unwrap();
}

#[test]
pub fn java_byte_array_from_slice() {
    attach_current_thread(|env| {
        let buf: &[u8] = &[1, 2, 3];
        let java_array = env
            .byte_array_from_slice(buf)
            .expect("Env#byte_array_from_slice must create a java array from slice")
            .auto();

        assert!(!java_array.is_null());
        let mut res: [i8; 3] = [0; 3];
        java_array.get_region(env, 0, &mut res).unwrap();
        assert_eq!(res[0], 1);
        assert_eq!(res[1], 2);
        assert_eq!(res[2], 3);

        Ok(())
    })
    .unwrap();
}

macro_rules! test_auto_array_read_write {
    ( $test_name:tt, $jni_type:ty, $new_array:tt, $value_a:tt, $value_b:tt ) => {
        #[test]
        pub fn $test_name() {
            attach_current_thread(|env| {
                // Create original Java array
                let buf: &[$jni_type] = &[$value_a as $jni_type, $value_b as $jni_type];
                let java_array = env
                    .$new_array(2)
                    .expect(stringify!(Env #$new_array must create a Java $jni_type array with given size));

                // Insert array elements
                let _ = java_array.set_region(env, 0, buf);

                // Use a scope to test Drop
                {
                    // Redundantly push a new JNI stack frame to verify that AutoElements is not
                    // tied to the lifetime of the `env` that it's got from (env.get_array_elements()
                    // doesn't involve creating a new reference, it associates a pointer with
                    // an existing array reference)
                    let mut auto_ptr = env.with_local_frame(10, |env| -> jni::errors::Result<_> {
                        // Get byte array elements auto wrapper
                        let auto_ptr: AutoElements<$jni_type, _> = unsafe {
                            java_array.get_elements(env, ReleaseMode::CopyBack).unwrap()
                        };
                        Ok(auto_ptr)
                    }).unwrap();

                    // Check array size
                    assert_eq!(auto_ptr.len(), 2);

                    // Check pointer access
                    let ptr = auto_ptr.as_ptr();
                    assert_eq!(unsafe { *ptr.offset(0) }, $value_a);
                    assert_eq!(unsafe { *ptr.offset(1) }, $value_b);

                    // Check pointer From access
                    let ptr: *mut $jni_type = std::convert::From::from(&auto_ptr);
                    assert_eq!(unsafe { *ptr.offset(0) }, $value_a);
                    assert_eq!(unsafe { *ptr.offset(1) }, $value_b);

                    // Check pointer into() access
                    let ptr: *mut $jni_type = (&auto_ptr).into();
                    assert_eq!(unsafe { *ptr.offset(0) }, $value_a);
                    assert_eq!(unsafe { *ptr.offset(1) }, $value_b);

                    // Check slice access
                    //
                    // # Safety
                    //
                    // We make sure that the slice is dropped before also testing access via `Deref`
                    // (to ensure we don't have aliased references)
                    unsafe {
                        let slice = std::slice::from_raw_parts(auto_ptr.as_ptr(), auto_ptr.len());
                        assert_eq!(slice[0], $value_a);
                        assert_eq!(slice[1], $value_b);
                    }

                    // Check access via Deref
                    assert_eq!(auto_ptr[0], $value_a);
                    assert_eq!(auto_ptr[1], $value_b);

                    // Modify via DerefMut
                    let tmp = auto_ptr[1];
                    auto_ptr[1] = auto_ptr[0];
                    auto_ptr[0] = tmp;

                    // Commit would be necessary here, if there were no closure
                    //auto_ptr.commit().unwrap();
                }

                // Confirm modification of original Java array
                let mut res: [$jni_type; 2] = [$value_a as $jni_type; 2];
                java_array.get_region(env, 0, &mut res).unwrap();
                assert_eq!(res[0], $value_b);
                assert_eq!(res[1], $value_a);
                Ok(())
            }).unwrap();
        }
    };
}

// Test generic get_array_elements
test_auto_array_read_write!(get_array_elements, jint, new_int_array, 0, 1);

// Test type-specific array accessors
test_auto_array_read_write!(get_int_array_elements, jint, new_int_array, 0, 1);

test_auto_array_read_write!(get_long_array_elements, jlong, new_long_array, 0, 1);

test_auto_array_read_write!(get_byte_array_elements, jbyte, new_byte_array, 0, 1);

test_auto_array_read_write!(
    get_boolean_array_elements,
    jboolean,
    new_boolean_array,
    true,
    false
);

test_auto_array_read_write!(get_char_array_elements, jchar, new_char_array, 0, 1);

test_auto_array_read_write!(get_short_array_elements, jshort, new_short_array, 0, 1);

test_auto_array_read_write!(get_float_array_elements, jfloat, new_float_array, 0.0, 1.0);

test_auto_array_read_write!(
    get_double_array_elements,
    jdouble,
    new_double_array,
    0.0,
    1.0
);

#[test]
#[ignore] // Disabled until issue #283 is resolved
pub fn get_long_array_elements_commit() {
    attach_current_thread(|env| {
        // Create original Java array
        let buf: &[i64] = &[1, 2, 3];
        let java_array = env
            .new_long_array(3)
            .expect("Env#new_long_array must create a java array with given size");

        // Insert array elements
        let _ = java_array.set_region(env, 0, buf);

        // Get long array elements auto wrapper
        let mut auto_ptr = unsafe { java_array.get_elements(env, ReleaseMode::CopyBack).unwrap() };

        // Copying the array depends on the VM vendor/version/GC combinations.
        // If the wrapped array is not being copied, we can skip the test.
        if !auto_ptr.is_copy() {
            return Ok(());
        }

        // Check pointer access
        let ptr = auto_ptr.as_ptr();

        // Modify
        unsafe {
            *ptr.offset(0) += 1;
            *ptr.offset(1) += 1;
            *ptr.offset(2) += 1;
        }

        // Check that original Java array is unmodified
        let mut res: [i64; 3] = [0; 3];
        java_array.get_region(env, 0, &mut res).unwrap();
        assert_eq!(res[0], 1);
        assert_eq!(res[1], 2);
        assert_eq!(res[2], 3);

        auto_ptr.commit().unwrap();

        // Confirm modification of original Java array
        java_array.get_region(env, 0, &mut res).unwrap();
        assert_eq!(res[0], 2);
        assert_eq!(res[1], 3);
        assert_eq!(res[2], 4);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_array_elements_critical() {
    attach_current_thread(|env| {
        // Create original Java array
        let buf: &[u8] = &[1, 2, 3];
        let java_array = env
            .byte_array_from_slice(buf)
            .expect("Env#byte_array_from_slice must create a java array from slice");

        // Use a scope to test Drop
        {
            // Get primitive array elements auto wrapper
            let mut auto_ptr = unsafe {
                java_array
                    .get_elements_critical(env, ReleaseMode::CopyBack)
                    .unwrap()
            };

            // Check array size
            assert_eq!(auto_ptr.len(), 3);

            // Convert void pointer to a &[i8] slice, without copy
            //
            // # Safety
            //
            // We make sure that the slice is dropped before also testing access via `Deref`
            // (to ensure we don't have aliased references)
            unsafe {
                let slice = std::slice::from_raw_parts(auto_ptr.as_ptr(), auto_ptr.len());
                assert_eq!(slice[0], 1);
                assert_eq!(slice[1], 2);
                assert_eq!(slice[2], 3);
            }

            // Also check access via `Deref`
            assert_eq!(auto_ptr[0], 1);
            assert_eq!(auto_ptr[1], 2);
            assert_eq!(auto_ptr[2], 3);

            // Modify via `DerefMut`
            auto_ptr[0] += 1;
            auto_ptr[1] += 1;
            auto_ptr[2] += 1;
        }

        // Confirm modification of original Java array
        let mut res: [i8; 3] = [0; 3];
        java_array.get_region(env, 0, &mut res).unwrap();
        assert_eq!(res[0], 2);
        assert_eq!(res[1], 3);
        assert_eq!(res[2], 4);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_object_class() {
    attach_current_thread(|env| {
        let string = env.new_string(c"test").unwrap();
        let result = env.get_object_class(string);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_object_class_null_arg() {
    attach_current_thread(|env| {
        let null_obj = JObject::null();
        let result = env
            .get_object_class(null_obj)
            .map_err(|error| matches!(error, Error::NullPtr(_)))
            .expect_err("Env#get_object_class should return error for null argument");
        assert!(result, "ErrorKind::NullPtr expected as error");

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn new_direct_byte_buffer() {
    attach_current_thread(|env| {
        let vec: Vec<u8> = vec![0, 1, 2, 3];
        let (addr, len) = {
            // (would use buf.into_raw_parts() on nightly)
            let buf = vec.leak();
            (buf.as_mut_ptr(), buf.len())
        };
        let result = unsafe { env.new_direct_byte_buffer(addr, len) };
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());
        Ok(())
    })
    .unwrap();
}

#[test]
pub fn new_direct_byte_buffer_invalid_addr() {
    attach_current_thread(|env| {
        let result = unsafe { env.new_direct_byte_buffer(std::ptr::null_mut(), 5) };
        assert!(result.is_err());

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_direct_buffer_capacity_ok() {
    attach_current_thread(|env| {
        let vec: Vec<u8> = vec![0, 1, 2, 3];
        let (addr, len) = {
            // (would use buf.into_raw_parts() on nightly)
            let buf = vec.leak();
            (buf.as_mut_ptr(), buf.len())
        };
        let result = unsafe { env.new_direct_byte_buffer(addr, len) }.unwrap();
        assert!(!result.is_null());

        let capacity = env.get_direct_buffer_capacity(&result).unwrap();
        assert_eq!(capacity, 4);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_direct_buffer_capacity_wrong_arg() {
    attach_current_thread(|env| {
        let wrong_obj =
            unsafe { JByteBuffer::from_raw(env.new_string(c"wrong").unwrap().into_raw()) };
        let capacity = env.get_direct_buffer_capacity(&wrong_obj);
        assert!(capacity.is_err());

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_direct_buffer_capacity_null_arg() {
    attach_current_thread(|env| {
        let result = env.get_direct_buffer_capacity(&Default::default());
        assert!(result.is_err());

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_direct_buffer_address_ok() {
    attach_current_thread(|env| {
        let vec: Vec<u8> = vec![0, 1, 2, 3];
        let (addr, len) = {
            // (would use buf.into_raw_parts() on nightly)
            let buf = vec.leak();
            (buf.as_mut_ptr(), buf.len())
        };
        let result = unsafe { env.new_direct_byte_buffer(addr, len) }.unwrap();
        assert!(!result.is_null());

        let dest_buffer = env.get_direct_buffer_address(&result).unwrap();
        assert_eq!(addr, dest_buffer);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_direct_buffer_address_wrong_arg() {
    attach_current_thread(|env| {
        let wrong_obj: JObject = env.new_string(c"wrong").unwrap().into();

        // SAFETY: This is not a valid cast and not generally safe but `GetDirectBufferAddress` is
        // documented to return a null pointer in case the "given object is not a direct java.nio.Buffer".
        let wrong_obj = unsafe { JByteBuffer::from_raw(wrong_obj.into_raw()) };
        let result = env.get_direct_buffer_address(&wrong_obj);
        assert!(result.is_err());

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn get_direct_buffer_address_null_arg() {
    attach_current_thread(|env| {
        let result = env.get_direct_buffer_address(&JByteBuffer::null());
        assert!(result.is_err());

        Ok(())
    })
    .unwrap();
}

// Group test for testing the family of new_PRIMITIVE_array functions with correct arguments
#[test]
pub fn new_primitive_array_ok() {
    attach_current_thread(|env| {
        const SIZE: jsize = 16;

        let result = env.new_boolean_array(SIZE);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        let result = env.new_byte_array(SIZE);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        let result = env.new_char_array(SIZE);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        let result = env.new_short_array(SIZE);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        let result = env.new_int_array(SIZE);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        let result = env.new_long_array(SIZE);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        let result = env.new_float_array(SIZE);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        let result = env.new_double_array(SIZE);
        assert!(result.is_ok());
        assert!(!result.unwrap().is_null());

        Ok(())
    })
    .unwrap();
}

// Group test for testing the family of new_PRIMITIVE_array functions with wrong arguments
#[test]
pub fn new_primitive_array_wrong() {
    attach_current_thread(|env| {
        const WRONG_SIZE: jsize = -1;

        let result = env.new_boolean_array(WRONG_SIZE).map(|arr| arr.as_raw());
        assert_exception(&result, "Env#new_boolean_array should throw exception");
        assert_pending_java_exception(env);

        let result = env.new_byte_array(WRONG_SIZE).map(|arr| arr.as_raw());
        assert_exception(&result, "Env#new_byte_array should throw exception");
        assert_pending_java_exception(env);

        let result = env.new_char_array(WRONG_SIZE).map(|arr| arr.as_raw());
        assert_exception(&result, "Env#new_char_array should throw exception");
        assert_pending_java_exception(env);

        let result = env.new_short_array(WRONG_SIZE).map(|arr| arr.as_raw());
        assert_exception(&result, "Env#new_short_array should throw exception");
        assert_pending_java_exception(env);

        let result = env.new_int_array(WRONG_SIZE).map(|arr| arr.as_raw());
        assert_exception(&result, "Env#new_int_array should throw exception");
        assert_pending_java_exception(env);

        let result = env.new_long_array(WRONG_SIZE).map(|arr| arr.as_raw());
        assert_exception(&result, "Env#new_long_array should throw exception");
        assert_pending_java_exception(env);

        let result = env.new_float_array(WRONG_SIZE).map(|arr| arr.as_raw());
        assert_exception(&result, "Env#new_float_array should throw exception");
        assert_pending_java_exception(env);

        let result = env.new_double_array(WRONG_SIZE).map(|arr| arr.as_raw());
        assert_exception(&result, "Env#new_double_array should throw exception");
        assert_pending_java_exception(env);

        Ok(())
    })
    .unwrap();
}

#[test]
fn get_super_class_ok() {
    attach_current_thread(|env| {
        let result = env.get_superclass(ARRAYLIST_CLASS);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        Ok(())
    })
    .unwrap();
}

#[test]
fn get_super_class_null() {
    attach_current_thread(|env| {
        let result = env.get_superclass(c"java/lang/Object");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        Ok(())
    })
    .unwrap();
}

#[test]
fn convert_byte_array() {
    attach_current_thread(|env| {
        let src: Vec<u8> = vec![1, 2, 3, 4];
        let java_byte_array = env.byte_array_from_slice(&src).unwrap();

        let dest = env.convert_byte_array(java_byte_array);
        assert!(dest.is_ok());
        assert_eq!(dest.unwrap(), src);

        Ok(())
    })
    .unwrap();
}

#[test]
fn local_ref_null() {
    attach_current_thread(|env| {
        let null_obj = JObject::null();

        let result = env.new_local_ref::<&JObject>(&null_obj);
        assert!(result.is_ok());
        assert!(result.unwrap().is_null());

        // "delete" null reference
        env.delete_local_ref(null_obj);

        Ok(())
    })
    .unwrap();
}

#[test]
fn new_global_ref_null() {
    attach_current_thread(|env| {
        let null_obj = JObject::null();
        let result = env.new_global_ref(null_obj);
        assert!(result.is_ok());
        assert!(result.unwrap().is_null());

        Ok(())
    })
    .unwrap();
}

#[test]
fn new_weak_ref_null() {
    attach_current_thread(|env| {
        let null_obj = JObject::null();
        let result = env.new_weak_ref(null_obj);
        assert!(matches!(result, Err(Error::ObjectFreed)));

        let null_weak: Weak<JObject<'static>> = Weak::null();
        assert!(null_weak.is_garbage_collected(env));

        Ok(())
    })
    .unwrap();
}

#[test]
fn auto_null() {
    let null_obj = JObject::null();
    {
        let auto_ref = null_obj.auto();
        assert!(auto_ref.is_null());
    }
}

#[test]
fn test_call_nonvirtual_method() {
    attach_current_thread(|env| {
        let a_string = JObject::from(env.new_string(c"test").unwrap());
        let another_string = JObject::from(env.new_string(c"test").unwrap());

        // The `equals` method in java/lang/Object will compare the reference
        let obj_class = env.find_class(c"java/lang/Object").unwrap();
        let object_result = env
            .call_nonvirtual_method(
                &a_string,
                &obj_class,
                c"equals",
                c"(Ljava/lang/Object;)Z",
                &[JValue::from(&another_string)],
            )
            .unwrap()
            .z()
            .unwrap();
        assert!(!object_result);

        // However, java/lang/String overrided it and it now compares the content.
        let string_class = env.find_class(c"java/lang/String").unwrap();
        let string_result = env
            .call_nonvirtual_method(
                &a_string,
                &string_class,
                c"equals",
                c"(Ljava/lang/Object;)Z",
                &[JValue::from(&another_string)],
            )
            .unwrap()
            .z()
            .unwrap();
        assert!(string_result);

        Ok(())
    })
    .unwrap();
}

#[test]
fn short_lifetime_with_local_frame() {
    attach_current_thread(|env| {
        let object = short_lifetime_with_local_frame_sub_fn(env);
        assert!(object.is_ok());

        Ok(())
    })
    .unwrap();
}

fn short_lifetime_with_local_frame_sub_fn<'local>(
    env: &'_ mut Env<'local>,
) -> Result<JObject<'local>, Error> {
    env.with_local_frame_returning_local::<_, JObject, _>(16, |env| {
        env.new_object(INTEGER_CLASS, c"(I)V", &[JValue::from(5)])
    })
}

#[test]
fn short_lifetime_list() {
    attach_current_thread(|env| {
        let first_list_object = short_lifetime_list_sub_fn(env).unwrap();
        let value = env.call_method(first_list_object, c"intValue", c"()I", &[]);
        assert_eq!(value.unwrap().i().unwrap(), 1);

        Ok(())
    })
    .unwrap();
}

fn short_lifetime_list_sub_fn<'local>(env: &'_ mut Env<'local>) -> Result<JObject<'local>, Error> {
    let list_object = env.new_object(ARRAYLIST_CLASS, c"()V", &[])?;
    let list = env.as_cast::<JList>(&list_object)?;
    let element = env.new_object(INTEGER_CLASS, c"(I)V", &[JValue::from(1)])?;
    list.add(env, &element)?;
    short_lifetime_list_sub_fn_get_first_element(&list, env)
}

fn short_lifetime_list_sub_fn_get_first_element<'list_local, 'env_local>(
    list: &'_ JList<'list_local>,
    env: &'_ mut Env<'env_local>,
) -> Result<JObject<'env_local>, Error> {
    let iterator = list.iter(env)?;
    Ok(iterator.next(env)?.unwrap())
}

#[test]
fn get_object_array_element() {
    attach_current_thread(|env| {
        let array = env
            .new_object_array(1, STRING_CLASS, JObject::null())
            .unwrap();
        assert!(!array.is_null());
        assert!(env.get_object_array_element(&array, 0).unwrap().is_null());
        let test_str = env.new_string(c"test").unwrap();
        env.set_object_array_element(&array, 0, test_str).unwrap();
        assert!(!env.get_object_array_element(&array, 0).unwrap().is_null());

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn throw_new() {
    attach_current_thread(|env| {
        let result = env.throw_new(RUNTIME_EXCEPTION_CLASS, c"Test Exception");
        assert!(result.is_ok());
        assert_pending_java_exception_detailed(
            env,
            Some(RUNTIME_EXCEPTION_CLASS),
            Some(JNIStr::from_cstr(c"Test Exception")),
        );

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn throw_new_fail() {
    attach_current_thread(|env| {
        let result = env.throw_new(c"java/lang/NonexistentException", c"Test Exception");
        assert!(result.is_err());
        // Just to clear the java.lang.NoClassDefFoundError
        assert_pending_java_exception(env);

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn throw_defaults() {
    attach_current_thread(|env| {
        test_throwable_descriptor_with_default_type(env, TEST_EXCEPTION_MESSAGE);
        test_throwable_descriptor_with_default_type(env, TEST_EXCEPTION_MESSAGE.to_owned());
        test_throwable_descriptor_with_default_type(env, JNIString::from(TEST_EXCEPTION_MESSAGE));

        Ok(())
    })
    .unwrap();
}

#[test]
pub fn test_conversion() {
    attach_current_thread(|env| {
        let orig_obj: JObject = env.new_string(c"Hello, world!").unwrap().into();

        let obj: JObject = unwrap(env.new_local_ref(&orig_obj), env);

        let string = env.cast_local::<JString>(obj).unwrap();
        let actual = JObject::from(string);
        assert!(env.is_same_object(&orig_obj, actual));

        let global_ref = env.new_global_ref(&orig_obj).unwrap();
        assert!(env.is_same_object(&orig_obj, global_ref));

        let weak_ref = unwrap(env.new_weak_ref(&orig_obj), env);
        let actual =
            unwrap(weak_ref.upgrade_local(env), env).expect("weak ref should not have been GC'd");
        assert!(env.is_same_object(&orig_obj, actual));

        let auto_local = unwrap(env.new_local_ref(&orig_obj), env).auto();
        assert!(env.is_same_object(&orig_obj, auto_local));

        Ok(())
    })
    .unwrap();
}

rusty_fork_test! {
#[test]
fn test_jstring_conversion() {
    // Even while JNI is not initialized a nul should be formatted as "<NULL>"
    let null = JString::null();
    assert_eq!(null.to_string(), "<NULL>");

    // XXX: this is highly unsafe but the expectation here is that the implementation
    // will not get as far as attempting to dereference the invalid pointer because
    // JavaVM::singleton is not yet initialized.
    //
    // Alternatively the only other way we could test this case would be with a
    // native method callback which is hard to reproduce here.
    let invalid = unsafe { JString::from_raw(1 as _) };
    assert_eq!(invalid.to_string(), "<JNI Not Initialized>");

    attach_current_thread(|env| {
        let hello: JString = env.new_string(c"Hello, world!").unwrap();

        assert_eq!(hello.to_string(), "Hello, world!");
        assert_eq!(hello.to_string(), hello.try_to_string(env).unwrap());

        Ok(())
    })
    .unwrap();
}
}

#[test]
pub fn test_null_string_mutf8_chars() {
    attach_current_thread(|env| {
        let s = unsafe { JString::from_raw(std::ptr::null_mut() as _) };
        let ret = s.mutf8_chars(env);
        assert!(ret.is_err());

        Ok(())
    })
    .unwrap();
}

fn test_throwable_descriptor_with_default_type<'local, D>(env: &mut Env<'local>, descriptor: D)
where
    D: Desc<'local, JThrowable<'local>>,
{
    let result = descriptor.lookup(env);
    assert!(result.is_ok());
    let exception = result.unwrap();
    let exception = exception.as_ref();

    assert_exception_type(env, exception, RUNTIME_EXCEPTION_CLASS);
    assert_exception_message(env, exception, TEST_EXCEPTION_MESSAGE);
}

// Helper method that asserts that result is Error and the cause is JavaException.
fn assert_exception(res: &Result<jobject, Error>, expect_message: &str) {
    assert!(res.is_err());
    assert!(res
        .as_ref()
        .map_err(|error| matches!(error, Error::JavaException))
        .expect_err(expect_message));
}

// Shortcut to `assert_pending_java_exception_detailed()` without checking for expected  type and
// message of exception.
fn assert_pending_java_exception(env: &mut Env) {
    assert_pending_java_exception_detailed(env, None, None)
}

// Helper method that asserts there is a pending Java exception of `expected_type` with
// `expected_message` and clears it if any.
fn assert_pending_java_exception_detailed(
    env: &mut Env,
    expected_type: Option<&JNIStr>,
    expected_message: Option<&JNIStr>,
) {
    assert!(env.exception_check());
    let exception = env.exception_occurred().expect("Unable to get exception");
    env.exception_clear();

    if let Some(expected_type) = expected_type {
        assert_exception_type(env, &exception, expected_type);
    }

    if let Some(expected_message) = expected_message {
        assert_exception_message(env, &exception, expected_message);
    }
}

// Asserts that exception is of `expected_type` type.
fn assert_exception_type(env: &mut Env, exception: &JThrowable, expected_type: &JNIStr) {
    assert!(env.is_instance_of(exception, expected_type).unwrap());
}

// Asserts that exception's message is `expected_message`.
fn assert_exception_message(env: &mut Env, exception: &JThrowable, expected_message: &JNIStr) {
    let message = env
        .call_method(exception, c"getMessage", c"()Ljava/lang/String;", &[])
        .unwrap()
        .l()
        .unwrap();
    let message = env.cast_local::<JString>(message).unwrap();
    let msg_rust: JNIString = message.mutf8_chars(env).unwrap().into();
    assert_eq!(msg_rust, expected_message);
}

#[test]
fn test_java_char_conversion() {
    attach_current_thread(|env| {
        // Make a Java `StringBuilder`.
        let sb = unwrap(env.new_object(c"java/lang/StringBuilder", c"()V", &[]), env);

        // U+1F913 is not representable in a single UTF-16 unit, so this conversion should fail.
        assert_matches!(JValue::try_from('🤓'), Err(CharToJavaError { char: '🤓' }));

        // It is of course representable in a single UTF-32 unit.
        unwrap(
            env.call_method(
                &sb,
                c"appendCodePoint",
                c"(I)Ljava/lang/StringBuilder;",
                &[JValue::int_from_char('🤓')],
            ),
            env,
        );

        // U+2603, on the other hand, *is* representable in a single UTF-16 unit.
        unwrap(
            env.call_method(
                &sb,
                c"append",
                c"(C)Ljava/lang/StringBuilder;",
                &[JValue::try_from('☃').unwrap()],
            ),
            env,
        );

        // Finish the `StringBuilder` and get a Java `String`.
        let s = unwrap(
            env.call_method(&sb, c"toString", c"()Ljava/lang/String;", &[]),
            env,
        )
        .l()
        .unwrap();

        env.delete_local_ref(sb);

        {
            // The first character in the string is U+1F913, which is not representable in a single UTF-16 unit.

            // Get the first Java `char` and try to unwrap it to a Rust `char`.
            let c = unwrap(
                env.call_method(&s, c"charAt", c"(I)C", &[JValue::Int(0)]),
                env,
            )
            .c_char();

            // That should fail.
            let c = assert_matches!(
                c,
                Err(Error::InvalidUtf16 { source })
                => source
            );

            // The unpaired surrogate should be correct.
            assert_eq!(c.unpaired_surrogate(), 0xd83e);
        }

        {
            // The first character in the string *is* representable in a single UTF-32 unit.

            // Get the UTF-32 unit and unwrap it.
            let c = unwrap(
                env.call_method(&s, c"codePointAt", c"(I)I", &[JValue::Int(0)]),
                env,
            )
            .i_char()
            .unwrap();

            // It should be correct.
            assert_eq!(c, '🤓');
        }

        {
            // The second character in the string *is* representable in a single UTF-16 unit.

            // Get it and unwrap it. It should succeed.
            let c = unwrap(
                env.call_method(
                    &s,
                    c"charAt",
                    c"(I)C",
                    // The first character is represented in UTF-16 as a surrogate pair, so the second character occurs at index 2 instead of 1.
                    &[JValue::Int(2)],
                ),
                env,
            )
            .c_char()
            .unwrap();

            // It should be correct.
            assert_eq!(c, '☃');
        }

        Ok(())
    })
    .unwrap();
}
