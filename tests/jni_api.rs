#![cfg(feature = "invocation")]

extern crate error_chain;
extern crate jni;

use jni::objects::{AutoLocal, JObject};

mod util;
use util::{attach_current_thread, unwrap};

static ARRAYLIST_CLASS: &str = "java/util/ArrayList";
static EXCEPTION_CLASS: &str = "java/lang/Exception";
static ARITHMETIC_EXCEPTION_CLASS: &str = "java/lang/ArithmeticException";
static INTEGER_CLASS: &str = "java/lang/Integer";

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
