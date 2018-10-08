#![cfg(feature = "invocation")]

extern crate error_chain;
extern crate jni;

use jni::{
    errors::ErrorKind,
    objects::{AutoLocal, JObject, JValue},
    sys::jint,
};

mod util;
use util::{attach_current_thread, unwrap};

static ARRAYLIST_CLASS: &str = "java/util/ArrayList";
static EXCEPTION_CLASS: &str = "java/lang/Exception";
static ARITHMETIC_EXCEPTION_CLASS: &str = "java/lang/ArithmeticException";
static INTEGER_CLASS: &str = "java/lang/Integer";
static MATH_CLASS: &str = "java/lang/Math";
static MATH_ABS_METHOD_NAME: &str = "abs";
static MATH_TO_INT_METHOD_NAME: &str = "toIntExact";
static MATH_ABS_SIGNATURE: &str = "(I)I";
static MATH_TO_INT_SIGNATURE: &str = "(J)I";

#[test]
pub fn call_method_returning_null() {
    let env = attach_current_thread();
    // Create an Exception with no message
    let obj = AutoLocal::new(&env, unwrap(&env, env.new_object(EXCEPTION_CLASS, "()V", &[])));
    // Call Throwable#getMessage must return null
    let message = unwrap(&env, env.call_method(obj.as_obj(), "getMessage", "()Ljava/lang/String;", &[]));
    let message_ref = env.auto_local(unwrap(&env, message.l()));

    assert!(message_ref.as_obj().is_null());
}

#[test]
pub fn is_instance_of_same_class() {
    let env = attach_current_thread();
    let obj = AutoLocal::new(&env, unwrap(&env, env.new_object(EXCEPTION_CLASS, "()V", &[])));
    assert!(unwrap(&env, env.is_instance_of(obj.as_obj(), EXCEPTION_CLASS)));
}

#[test]
pub fn is_instance_of_superclass() {
    let env = attach_current_thread();
    let obj = AutoLocal::new(&env, unwrap(&env, env.new_object(ARITHMETIC_EXCEPTION_CLASS, "()V", &[])));
    assert!(unwrap(&env, env.is_instance_of(obj.as_obj(), EXCEPTION_CLASS)));
}

#[test]
pub fn is_instance_of_subclass() {
    let env = attach_current_thread();
    let obj = AutoLocal::new(&env, unwrap(&env, env.new_object(EXCEPTION_CLASS, "()V", &[])));
    assert!(!unwrap(&env, env.is_instance_of(obj.as_obj(), ARITHMETIC_EXCEPTION_CLASS)));
}

#[test]
pub fn is_instance_of_not_superclass() {
    let env = attach_current_thread();
    let obj = AutoLocal::new(&env, unwrap(&env, env.new_object(ARITHMETIC_EXCEPTION_CLASS, "()V", &[])));
    assert!(!unwrap(&env, env.is_instance_of(obj.as_obj(), ARRAYLIST_CLASS)));
}

#[test]
pub fn is_instance_of_null() {
    let env = attach_current_thread();
    let obj = JObject::null();
    assert!(unwrap(&env, env.is_instance_of(obj, ARRAYLIST_CLASS)));
    assert!(unwrap(&env, env.is_instance_of(obj, EXCEPTION_CLASS)));
    assert!(unwrap(&env, env.is_instance_of(obj, ARITHMETIC_EXCEPTION_CLASS)));
}

#[test]
pub fn get_static_public_field() {
    let env = attach_current_thread();

    let min_int_value = env.get_static_field(INTEGER_CLASS, "MIN_VALUE", "I")
        .unwrap()
        .i()
        .unwrap();

    assert_eq!(min_int_value, i32::min_value());
}

#[test]
pub fn pop_local_frame_pending_exception() {
    let env = attach_current_thread();

    env.push_local_frame(16).unwrap();

    env.throw_new("java/lang/RuntimeException", "Test Exception").unwrap();

    // Pop the local frame with a pending exception
    env.pop_local_frame(JObject::null())
        .expect("JNIEnv#pop_local_frame must work in case of pending exception");

    env.exception_clear().unwrap();
}

#[test]
pub fn push_local_frame_pending_exception() {
    let env = attach_current_thread();

    env.throw_new("java/lang/RuntimeException", "Test Exception").unwrap();

    // Push a new local frame with a pending exception
    env.push_local_frame(16)
        .expect("JNIEnv#push_local_frame must work in case of pending exception");

    env.exception_clear().unwrap();

    env.pop_local_frame(JObject::null()).unwrap();
}

#[test]
pub fn push_local_frame_too_many_refs() {
    let env = attach_current_thread();

    // Try to push a new local frame with a ridiculous size
    let frame_size = i32::max_value();
    env.push_local_frame(frame_size)
        .expect_err("push_local_frame(2B) must Err");

    env.pop_local_frame(JObject::null()).unwrap();
}

#[test]
pub fn with_local_frame() {
    let env = attach_current_thread();

    let s = env.with_local_frame(16, || {
        let res = env.new_string("Test").unwrap();
        Ok(res.into())
    }).unwrap();

    let s = env.get_string(s.into())
        .expect("The object returned from the local frame must remain valid");
    assert_eq!(s.to_str().unwrap(), "Test");
}

#[test]
pub fn with_local_frame_pending_exception() {
    let env = attach_current_thread();

    env.throw_new("java/lang/RuntimeException", "Test Exception").unwrap();

    // Try to allocate a frame of locals
    env.with_local_frame(16, || {
        Ok(JObject::null())
    }).expect("JNIEnv#with_local_frame must work in case of pending exception");

    env.exception_clear().unwrap();
}

#[test]
pub fn call_static_method_ok() {
    let env = attach_current_thread();

    let x = JValue::from(-10);
    let val: jint = env.call_static_method(MATH_CLASS, MATH_ABS_METHOD_NAME, MATH_ABS_SIGNATURE, &[x])
        .expect("JNIEnv#call_static_method_unsafe should return JValue").i().unwrap();

    assert_eq!(val, 10);
}

#[test]
pub fn call_static_method_throws() {
    let env = attach_current_thread();

    let x = JValue::Long(4_000_000_000);
    let is_java_exception = env
        .call_static_method(MATH_CLASS, MATH_TO_INT_METHOD_NAME, MATH_TO_INT_SIGNATURE, &[x])
        .map_err(|error| match error.0 {
            ErrorKind::JavaException => true,
            _ => false,
        }).expect_err("JNIEnv#call_static_method_unsafe should return error");

    assert!(is_java_exception, "ErrorKind::JavaException expected as error");
    assert!(env.exception_check().unwrap());
    env.exception_clear().unwrap();
}

#[test]
pub fn call_static_method_wrong_arg() {
    let env = attach_current_thread();

    let x = JValue::Double(4.56789123);
    env.call_static_method(MATH_CLASS, MATH_TO_INT_METHOD_NAME, MATH_TO_INT_SIGNATURE, &[x])
        .expect_err("JNIEnv#call_static_method_unsafe should return error");

    assert!(env.exception_check().unwrap());
    env.exception_clear().unwrap();
}

#[test]
pub fn java_byte_array_from_slice() {
    let env = attach_current_thread();
    let buf: &[u8] = &[1, 2, 3];
    let java_array = env.byte_array_from_slice(buf)
        .expect("JNIEnv#byte_array_from_slice must create a java array from slice");
    let obj = AutoLocal::new(&env, JObject::from(java_array));

    assert!(!obj.as_obj().is_null());
    let mut res: [i8; 3] = [0; 3];
    env.get_byte_array_region(java_array, 0, &mut res).unwrap();
    assert_eq!(res[0], 1);
    assert_eq!(res[1], 2);
    assert_eq!(res[2], 3);
}

#[test]
pub fn get_object_class() {
    let env = attach_current_thread();
    let string = env.new_string("test").unwrap();
    let result = env.get_object_class(string.into());
    assert!(result.is_ok());
    assert!(!result.unwrap().is_null());
}

#[test]
pub fn get_object_class_null_arg() {
    let env = attach_current_thread();
    let null_obj = JObject::null();
    let result = env.get_object_class(null_obj).map_err(|error| match *error.kind() {
        ErrorKind::NullPtr(_) => true,
        _ => false,
    }).expect_err("JNIEnv#get_object_class should return error for null argument");
    assert!(result, "ErrorKind::NullPtr expected as error");
}

#[test]
pub fn new_direct_byte_buffer() {
    let env = attach_current_thread();
    let mut vec: Vec<u8> = vec![0, 1, 2, 3];
    let buf = vec.as_mut_slice();
    let result = env.new_direct_byte_buffer(buf);
    assert!(result.is_ok());
    assert!(!result.unwrap().is_null());
}

#[test]
pub fn get_direct_buffer_capacity_ok() {
    let env = attach_current_thread();
    let mut vec: Vec<u8> = vec![0, 1, 2, 3];
    let buf = vec.as_mut_slice();
    let result = env.new_direct_byte_buffer(buf).unwrap();
    assert!(!result.is_null());

    let capacity = env.get_direct_buffer_capacity(result).unwrap();
    assert_eq!(capacity, 4);
}

#[test]
pub fn get_direct_buffer_capacity_wrong_arg() {
    let env = attach_current_thread();
    let capacity = env.get_direct_buffer_capacity(JObject::null().into());
    assert!(capacity.is_err());
}

#[test]
pub fn get_direct_buffer_address_ok() {
    let env = attach_current_thread();
    let mut vec: Vec<u8> = vec![0, 1, 2, 3];
    let buf = vec.as_mut_slice();
    let result = env.new_direct_byte_buffer(buf).unwrap();
    assert!(!result.is_null());

    let dest_buffer = env.get_direct_buffer_address(result).unwrap();
    assert_eq!(buf, dest_buffer);
}

#[test]
pub fn get_direct_buffer_address_wrong_arg() {
    let env = attach_current_thread();
    let wrong_obj: JObject = env.new_string("wrong").unwrap().into();
    let result = env.get_direct_buffer_address(wrong_obj.into());
    assert!(result.is_err());
}

#[test]
pub fn get_direct_buffer_address_null_arg() {
    let env = attach_current_thread();
    let result = env.get_direct_buffer_address(JObject::null().into());
    assert!(result.is_err());
}
