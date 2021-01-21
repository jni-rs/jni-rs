#![cfg(feature = "invocation")]

use std::str::FromStr;

use jni::{
    descriptors::Desc,
    errors::Error,
    objects::{
        AutoArray, AutoLocal, JByteBuffer, JList, JObject, JString, JThrowable, JValue, ReleaseMode,
    },
    signature::JavaType,
    strings::JNIString,
    sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jobject, jshort, jsize},
    JNIEnv,
};

mod util;
use util::{attach_current_thread, unwrap};

static ARRAYLIST_CLASS: &str = "java/util/ArrayList";
static EXCEPTION_CLASS: &str = "java/lang/Exception";
static ARITHMETIC_EXCEPTION_CLASS: &str = "java/lang/ArithmeticException";
static RUNTIME_EXCEPTION_CLASS: &str = "java/lang/RuntimeException";
static INTEGER_CLASS: &str = "java/lang/Integer";
static MATH_CLASS: &str = "java/lang/Math";
static STRING_CLASS: &str = "java/lang/String";
static MATH_ABS_METHOD_NAME: &str = "abs";
static MATH_TO_INT_METHOD_NAME: &str = "toIntExact";
static MATH_ABS_SIGNATURE: &str = "(I)I";
static MATH_TO_INT_SIGNATURE: &str = "(J)I";
static TEST_EXCEPTION_MESSAGE: &str = "Default exception thrown";
static TESTING_OBJECT_STR: &str = "TESTING OBJECT";

#[test]
pub fn call_method_returning_null() {
    let env = attach_current_thread();
    // Create an Exception with no message
    let obj = AutoLocal::new(
        &env,
        unwrap(&env, env.new_object(EXCEPTION_CLASS, "()V", &[])),
    );
    // Call Throwable#getMessage must return null
    let message = unwrap(
        &env,
        env.call_method(&obj, "getMessage", "()Ljava/lang/String;", &[]),
    );
    let message_ref = env.auto_local(unwrap(&env, message.l()));

    assert!(message_ref.as_obj().is_null());
}

#[test]
pub fn is_instance_of_same_class() {
    let env = attach_current_thread();
    let obj = AutoLocal::new(
        &env,
        unwrap(&env, env.new_object(EXCEPTION_CLASS, "()V", &[])),
    );
    assert!(unwrap(&env, env.is_instance_of(&obj, EXCEPTION_CLASS)));
}

#[test]
pub fn is_instance_of_superclass() {
    let env = attach_current_thread();
    let obj = AutoLocal::new(
        &env,
        unwrap(&env, env.new_object(ARITHMETIC_EXCEPTION_CLASS, "()V", &[])),
    );
    assert!(unwrap(&env, env.is_instance_of(&obj, EXCEPTION_CLASS)));
}

#[test]
pub fn is_instance_of_subclass() {
    let env = attach_current_thread();
    let obj = AutoLocal::new(
        &env,
        unwrap(&env, env.new_object(EXCEPTION_CLASS, "()V", &[])),
    );
    assert!(!unwrap(
        &env,
        env.is_instance_of(&obj, ARITHMETIC_EXCEPTION_CLASS)
    ));
}

#[test]
pub fn is_instance_of_not_superclass() {
    let env = attach_current_thread();
    let obj = AutoLocal::new(
        &env,
        unwrap(&env, env.new_object(ARITHMETIC_EXCEPTION_CLASS, "()V", &[])),
    );
    assert!(!unwrap(&env, env.is_instance_of(&obj, ARRAYLIST_CLASS)));
}

#[test]
pub fn is_instance_of_null() {
    let env = attach_current_thread();
    let obj = JObject::null();
    assert!(unwrap(&env, env.is_instance_of(obj, ARRAYLIST_CLASS)));
    assert!(unwrap(&env, env.is_instance_of(obj, EXCEPTION_CLASS)));
    assert!(unwrap(
        &env,
        env.is_instance_of(obj, ARITHMETIC_EXCEPTION_CLASS)
    ));
}

#[test]
pub fn is_same_object_diff_references() {
    let env = attach_current_thread();
    let string = env.new_string(TESTING_OBJECT_STR).unwrap();
    let ref_from_string = unwrap(&env, env.new_local_ref::<JObject>(string.into()));
    assert!(unwrap(&env, env.is_same_object(string, ref_from_string)));
    unwrap(&env, env.delete_local_ref(ref_from_string));
}

#[test]
pub fn is_same_object_same_reference() {
    let env = attach_current_thread();
    let string = env.new_string(TESTING_OBJECT_STR).unwrap();
    assert!(unwrap(&env, env.is_same_object(string, string)));
}

#[test]
pub fn is_not_same_object() {
    let env = attach_current_thread();
    let string = env.new_string(TESTING_OBJECT_STR).unwrap();
    let same_src_str = env.new_string(TESTING_OBJECT_STR).unwrap();
    assert!(!unwrap(&env, env.is_same_object(string, same_src_str)));
}

#[test]
pub fn is_not_same_object_null() {
    let env = attach_current_thread();
    assert!(unwrap(
        &env,
        env.is_same_object(JObject::null(), JObject::null())
    ));
}

#[test]
pub fn get_static_public_field() {
    let env = attach_current_thread();

    let min_int_value = env
        .get_static_field(INTEGER_CLASS, "MIN_VALUE", "I")
        .unwrap()
        .i()
        .unwrap();

    assert_eq!(min_int_value, i32::min_value());
}

#[test]
pub fn get_static_public_field_by_id() {
    let env = attach_current_thread();

    // One can't pass a JavaType::Primitive(Primitive::Int) to
    //   `get_static_field_id` unfortunately: #137
    let field_type = "I";
    let field_id = env
        .get_static_field_id(INTEGER_CLASS, "MIN_VALUE", field_type)
        .unwrap();

    let field_type = JavaType::from_str(field_type).unwrap();
    let min_int_value = env
        .get_static_field_unchecked(INTEGER_CLASS, field_id, field_type)
        .unwrap()
        .i()
        .unwrap();

    assert_eq!(min_int_value, i32::min_value());
}

#[test]
pub fn pop_local_frame_pending_exception() {
    let env = attach_current_thread();

    env.push_local_frame(16).unwrap();

    env.throw_new(RUNTIME_EXCEPTION_CLASS, "Test Exception")
        .unwrap();

    // Pop the local frame with a pending exception
    env.pop_local_frame(JObject::null())
        .expect("JNIEnv#pop_local_frame must work in case of pending exception");

    env.exception_clear().unwrap();
}

#[test]
pub fn push_local_frame_pending_exception() {
    let env = attach_current_thread();

    env.throw_new(RUNTIME_EXCEPTION_CLASS, "Test Exception")
        .unwrap();

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

    let s = env
        .with_local_frame(16, || {
            let res = env.new_string("Test").unwrap();
            Ok(res.into())
        })
        .unwrap();

    let s = env
        .get_string(s.into())
        .expect("The object returned from the local frame must remain valid");
    assert_eq!(s.to_str().unwrap(), "Test");
}

#[test]
pub fn with_local_frame_pending_exception() {
    let env = attach_current_thread();

    env.throw_new(RUNTIME_EXCEPTION_CLASS, "Test Exception")
        .unwrap();

    // Try to allocate a frame of locals
    env.with_local_frame(16, || Ok(JObject::null()))
        .expect("JNIEnv#with_local_frame must work in case of pending exception");

    env.exception_clear().unwrap();
}

#[test]
pub fn call_static_method_ok() {
    let env = attach_current_thread();

    let x = JValue::from(-10);
    let val: jint = env
        .call_static_method(MATH_CLASS, MATH_ABS_METHOD_NAME, MATH_ABS_SIGNATURE, &[x])
        .expect("JNIEnv#call_static_method_unsafe should return JValue")
        .i()
        .unwrap();

    assert_eq!(val, 10);
}

#[test]
pub fn call_static_method_throws() {
    let env = attach_current_thread();

    let x = JValue::Long(4_000_000_000);
    let is_java_exception = env
        .call_static_method(
            MATH_CLASS,
            MATH_TO_INT_METHOD_NAME,
            MATH_TO_INT_SIGNATURE,
            &[x],
        )
        .map_err(|error| matches!(error, Error::JavaException))
        .expect_err("JNIEnv#call_static_method_unsafe should return error");

    assert!(
        is_java_exception,
        "ErrorKind::JavaException expected as error"
    );
    assert_pending_java_exception(&env);
}

#[test]
pub fn call_static_method_wrong_arg() {
    let env = attach_current_thread();

    let x = JValue::Double(4.567_891_23);
    env.call_static_method(
        MATH_CLASS,
        MATH_TO_INT_METHOD_NAME,
        MATH_TO_INT_SIGNATURE,
        &[x],
    )
    .expect_err("JNIEnv#call_static_method_unsafe should return error");

    assert_pending_java_exception(&env);
}

#[test]
pub fn java_byte_array_from_slice() {
    let env = attach_current_thread();
    let buf: &[u8] = &[1, 2, 3];
    let java_array = env
        .byte_array_from_slice(buf)
        .expect("JNIEnv#byte_array_from_slice must create a java array from slice");
    let obj = AutoLocal::new(&env, JObject::from(java_array));

    assert!(!obj.as_obj().is_null());
    let mut res: [i8; 3] = [0; 3];
    env.get_byte_array_region(java_array, 0, &mut res).unwrap();
    assert_eq!(res[0], 1);
    assert_eq!(res[1], 2);
    assert_eq!(res[2], 3);
}

macro_rules! test_get_array_elements {
    ( $jni_get:tt, $jni_type:ty, $new_array:tt, $get_array:tt, $set_array:tt ) => {
        #[test]
        pub fn $jni_get() {
            let env = attach_current_thread();

            // Create original Java array
            let buf: &[$jni_type] = &[0 as $jni_type, 1 as $jni_type];
            let java_array = env
                .$new_array(2)
                .expect(stringify!(JNIEnv#$new_array must create a Java $jni_type array with given size));

            // Insert array elements
            let _ = env.$set_array(java_array, 0, buf);

            // Use a scope to test Drop
            {
                // Get byte array elements auto wrapper
                let auto_ptr: AutoArray<$jni_type> =
                    env.$jni_get(java_array, ReleaseMode::CopyBack).unwrap();

                // Check array size
                assert_eq!(auto_ptr.size().unwrap(), 2);

                // Check pointer access
                let ptr = auto_ptr.as_ptr();
                assert_eq!(unsafe { *ptr.offset(0) } as i32, 0);
                assert_eq!(unsafe { *ptr.offset(1) } as i32, 1);

                // Check pointer From access
                let ptr: *mut $jni_type = std::convert::From::from(&auto_ptr);
                assert_eq!(unsafe { *ptr.offset(0) } as i32, 0);
                assert_eq!(unsafe { *ptr.offset(1) } as i32, 1);

                // Check pointer into() access
                let ptr: *mut $jni_type = (&auto_ptr).into();
                assert_eq!(unsafe { *ptr.offset(0) } as i32, 0);
                assert_eq!(unsafe { *ptr.offset(1) } as i32, 1);

                // Modify
                unsafe {
                    *ptr.offset(0) += 1 as $jni_type;
                    *ptr.offset(1) -= 1 as $jni_type;
                }

                // Commit would be necessary here, if there were no closure
                //auto_ptr.commit().unwrap();
            }

            // Confirm modification of original Java array
            let mut res: [$jni_type; 2] = [0 as $jni_type; 2];
            env.$get_array(java_array, 0, &mut res).unwrap();
            assert_eq!(res[0] as i32, 1);
            assert_eq!(res[1] as i32, 0);
        }
    };
}

// Test generic get_array_elements
test_get_array_elements!(
    get_array_elements,
    jint,
    new_int_array,
    get_int_array_region,
    set_int_array_region
);

// Test type-specific array accessors
test_get_array_elements!(
    get_int_array_elements,
    jint,
    new_int_array,
    get_int_array_region,
    set_int_array_region
);

test_get_array_elements!(
    get_long_array_elements,
    jlong,
    new_long_array,
    get_long_array_region,
    set_long_array_region
);

test_get_array_elements!(
    get_byte_array_elements,
    jbyte,
    new_byte_array,
    get_byte_array_region,
    set_byte_array_region
);

test_get_array_elements!(
    get_boolean_array_elements,
    jboolean,
    new_boolean_array,
    get_boolean_array_region,
    set_boolean_array_region
);

test_get_array_elements!(
    get_char_array_elements,
    jchar,
    new_char_array,
    get_char_array_region,
    set_char_array_region
);

test_get_array_elements!(
    get_short_array_elements,
    jshort,
    new_short_array,
    get_short_array_region,
    set_short_array_region
);

test_get_array_elements!(
    get_float_array_elements,
    jfloat,
    new_float_array,
    get_float_array_region,
    set_float_array_region
);

test_get_array_elements!(
    get_double_array_elements,
    jdouble,
    new_double_array,
    get_double_array_region,
    set_double_array_region
);

#[test]
#[ignore] // Disabled until issue #283 is resolved
pub fn get_long_array_elements_commit() {
    let env = attach_current_thread();

    // Create original Java array
    let buf: &[i64] = &[1, 2, 3];
    let java_array = env
        .new_long_array(3)
        .expect("JNIEnv#new_long_array must create a java array with given size");

    // Insert array elements
    let _ = env.set_long_array_region(java_array, 0, buf);

    // Get long array elements auto wrapper
    let auto_ptr = env
        .get_long_array_elements(java_array, ReleaseMode::CopyBack)
        .unwrap();

    // Copying the array depends on the VM vendor/version/GC combinations.
    // If the wrapped array is not being copied, we can skip the test.
    if !auto_ptr.is_copy() {
        return;
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
    env.get_long_array_region(java_array, 0, &mut res).unwrap();
    assert_eq!(res[0], 1);
    assert_eq!(res[1], 2);
    assert_eq!(res[2], 3);

    auto_ptr.commit().unwrap();

    // Confirm modification of original Java array
    env.get_long_array_region(java_array, 0, &mut res).unwrap();
    assert_eq!(res[0], 2);
    assert_eq!(res[1], 3);
    assert_eq!(res[2], 4);
}

#[test]
pub fn get_primitive_array_critical() {
    let env = attach_current_thread();

    // Create original Java array
    let buf: &[u8] = &[1, 2, 3];
    let java_array = env
        .byte_array_from_slice(buf)
        .expect("JNIEnv#byte_array_from_slice must create a java array from slice");

    // Use a scope to test Drop
    {
        // Get primitive array elements auto wrapper
        let auto_ptr = env
            .get_primitive_array_critical(java_array, ReleaseMode::CopyBack)
            .unwrap();

        // Check array size
        assert_eq!(auto_ptr.size().unwrap(), 3);

        // Get pointer
        let ptr = auto_ptr.as_ptr();

        // Convert void pointer to an unsigned byte array, without copy
        let mut vec;
        unsafe { vec = Vec::from_raw_parts(ptr as *mut u8, 3, 3) }

        // Check
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);

        // Modify
        vec[0] += 1;
        vec[1] += 1;
        vec[2] += 1;

        // Release
        // Make sure vec's destructor doesn't free the data it thinks it owns when it goes out
        // of scope (avoid double free)
        std::mem::forget(vec);
    }

    // Confirm modification of original Java array
    let mut res: [i8; 3] = [0; 3];
    env.get_byte_array_region(java_array, 0, &mut res).unwrap();
    assert_eq!(res[0], 2);
    assert_eq!(res[1], 3);
    assert_eq!(res[2], 4);
}

#[test]
pub fn get_object_class() {
    let env = attach_current_thread();
    let string = env.new_string("test").unwrap();
    let result = env.get_object_class(string);
    assert!(result.is_ok());
    assert!(!result.unwrap().is_null());
}

#[test]
pub fn get_object_class_null_arg() {
    let env = attach_current_thread();
    let null_obj = JObject::null();
    let result = env
        .get_object_class(null_obj)
        .map_err(|error| matches!(error, Error::NullPtr(_)))
        .expect_err("JNIEnv#get_object_class should return error for null argument");
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
    let wrong_obj = JByteBuffer::from(env.new_string("wrong").unwrap().into_inner());
    let capacity = env.get_direct_buffer_capacity(wrong_obj);
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

// Group test for testing the family of new_PRIMITIVE_array functions with correct arguments
#[test]
pub fn new_primitive_array_ok() {
    let env = attach_current_thread();
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
}

// Group test for testing the family of new_PRIMITIVE_array functions with wrong arguments
#[test]
pub fn new_primitive_array_wrong() {
    let env = attach_current_thread();
    const WRONG_SIZE: jsize = -1;

    let result = env.new_boolean_array(WRONG_SIZE);
    assert_exception(&result, "JNIEnv#new_boolean_array should throw exception");
    assert_pending_java_exception(&env);

    let result = env.new_byte_array(WRONG_SIZE);
    assert_exception(&result, "JNIEnv#new_byte_array should throw exception");
    assert_pending_java_exception(&env);

    let result = env.new_char_array(WRONG_SIZE);
    assert_exception(&result, "JNIEnv#new_char_array should throw exception");
    assert_pending_java_exception(&env);

    let result = env.new_short_array(WRONG_SIZE);
    assert_exception(&result, "JNIEnv#new_short_array should throw exception");
    assert_pending_java_exception(&env);

    let result = env.new_int_array(WRONG_SIZE);
    assert_exception(&result, "JNIEnv#new_int_array should throw exception");
    assert_pending_java_exception(&env);

    let result = env.new_long_array(WRONG_SIZE);
    assert_exception(&result, "JNIEnv#new_long_array should throw exception");
    assert_pending_java_exception(&env);

    let result = env.new_float_array(WRONG_SIZE);
    assert_exception(&result, "JNIEnv#new_float_array should throw exception");
    assert_pending_java_exception(&env);

    let result = env.new_double_array(WRONG_SIZE);
    assert_exception(&result, "JNIEnv#new_double_array should throw exception");
    assert_pending_java_exception(&env);
}

#[test]
fn get_super_class_ok() {
    let env = attach_current_thread();
    let result = env.get_superclass(ARRAYLIST_CLASS);
    assert!(result.is_ok());
    assert!(!result.unwrap().is_null());
}

#[test]
fn get_super_class_null() {
    let env = attach_current_thread();
    let result = env.get_superclass("java/lang/Object");
    assert!(result.is_ok());
    assert!(result.unwrap().is_null());
}

#[test]
fn convert_byte_array() {
    let env = attach_current_thread();
    let src: Vec<u8> = vec![1, 2, 3, 4];
    let java_byte_array = env.byte_array_from_slice(&src).unwrap();

    let dest = env.convert_byte_array(java_byte_array);
    assert!(dest.is_ok());
    assert_eq!(dest.unwrap(), src);
}

#[test]
fn local_ref_null() {
    let env = attach_current_thread();
    let null_obj = JObject::null();

    let result = env.new_local_ref::<JObject>(null_obj);
    assert!(result.is_ok());
    assert!(result.unwrap().is_null());

    // try to delete null reference
    let result = env.delete_local_ref(null_obj);
    assert!(result.is_ok());
}

#[test]
fn new_global_ref_null() {
    let env = attach_current_thread();
    let null_obj = JObject::null();
    let result = env.new_global_ref(null_obj);
    assert!(result.is_ok());
    assert!(result.unwrap().as_obj().is_null());
}

#[test]
fn auto_local_null() {
    let env = attach_current_thread();
    let null_obj = JObject::null();
    {
        let auto_ref = AutoLocal::new(&env, null_obj);
        assert!(auto_ref.as_obj().is_null());
    }
    assert!(null_obj.is_null());
}

#[test]
fn short_lifetime_with_local_frame() {
    let env = attach_current_thread();
    let object = short_lifetime_with_local_frame_sub_fn(&env);
    assert!(object.is_ok());
}

fn short_lifetime_with_local_frame_sub_fn<'a>(env: &'_ JNIEnv<'a>) -> Result<JObject<'a>, Error> {
    env.with_local_frame(16, || {
        env.new_object(INTEGER_CLASS, "(I)V", &[JValue::from(5)])
    })
}

#[test]
fn short_lifetime_list() {
    let env = attach_current_thread();
    let first_list_object = short_lifetime_list_sub_fn(&env).unwrap();
    let value = env.call_method(first_list_object, "intValue", "()I", &[]);
    assert_eq!(value.unwrap().i().unwrap(), 1);
}

fn short_lifetime_list_sub_fn<'a>(env: &'_ JNIEnv<'a>) -> Result<JObject<'a>, Error> {
    let list_object = env.new_object(ARRAYLIST_CLASS, "()V", &[])?;
    let list = JList::from_env(env, list_object)?;
    let element = env.new_object(INTEGER_CLASS, "(I)V", &[JValue::from(1)])?;
    list.add(element)?;
    short_lifetime_list_sub_fn_get_first_element(&list)
}

fn short_lifetime_list_sub_fn_get_first_element<'a>(
    list: &'_ JList<'a, '_>,
) -> Result<JObject<'a>, Error> {
    let mut iterator = list.iter()?;
    Ok(iterator.next().unwrap())
}

#[test]
fn get_object_array_element() {
    let env = attach_current_thread();
    let array = env
        .new_object_array(1, STRING_CLASS, JObject::null())
        .unwrap();
    assert!(!array.is_null());
    assert!(env.get_object_array_element(array, 0).unwrap().is_null());
    let test_str = env.new_string("test").unwrap();
    env.set_object_array_element(array, 0, test_str).unwrap();
    assert!(!env.get_object_array_element(array, 0).unwrap().is_null());
}

#[test]
pub fn throw_new() {
    let env = attach_current_thread();

    let result = env.throw_new(RUNTIME_EXCEPTION_CLASS, "Test Exception");
    assert!(result.is_ok());
    assert_pending_java_exception_detailed(
        &env,
        Some(RUNTIME_EXCEPTION_CLASS),
        Some("Test Exception"),
    );
}

#[test]
pub fn throw_new_fail() {
    let env = attach_current_thread();

    let result = env.throw_new("java/lang/NonexistentException", "Test Exception");
    assert!(result.is_err());
    // Just to clear the java.lang.NoClassDefFoundError
    assert_pending_java_exception(&env);
}

#[test]
pub fn throw_defaults() {
    let env = attach_current_thread();

    test_throwable_descriptor_with_default_type(&env, TEST_EXCEPTION_MESSAGE);
    test_throwable_descriptor_with_default_type(&env, TEST_EXCEPTION_MESSAGE.to_owned());
    test_throwable_descriptor_with_default_type(&env, JNIString::from(TEST_EXCEPTION_MESSAGE));
}

#[test]
pub fn test_conversion() {
    let env = attach_current_thread();
    let orig_obj: JObject = env.new_string("Hello, world!").unwrap().into();

    let string = JString::from(orig_obj);
    let actual = JObject::from(string);
    assert!(unwrap(&env, env.is_same_object(orig_obj, actual)));

    let global_ref = env.new_global_ref(orig_obj).unwrap();
    let actual = JObject::from(&global_ref);
    assert!(unwrap(&env, env.is_same_object(orig_obj, actual)));

    let auto_local = env.auto_local(orig_obj);
    let actual = JObject::from(&auto_local);
    assert!(unwrap(&env, env.is_same_object(orig_obj, actual)));
}

fn test_throwable_descriptor_with_default_type<'a, D>(env: &JNIEnv<'a>, descriptor: D)
where
    D: Desc<'a, JThrowable<'a>>,
{
    let result = descriptor.lookup(env);
    assert!(result.is_ok());
    let exception = result.unwrap();

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
fn assert_pending_java_exception(env: &JNIEnv) {
    assert_pending_java_exception_detailed(env, None, None)
}

// Helper method that asserts there is a pending Java exception of `expected_type` with
// `expected_message` and clears it if any.
fn assert_pending_java_exception_detailed(
    env: &JNIEnv,
    expected_type: Option<&str>,
    expected_message: Option<&str>,
) {
    assert!(env.exception_check().unwrap());
    let exception = env.exception_occurred().expect("Unable to get exception");
    env.exception_clear().unwrap();

    if let Some(expected_type) = expected_type {
        assert_exception_type(env, exception, expected_type);
    }

    if let Some(expected_message) = expected_message {
        assert_exception_message(env, exception, expected_message);
    }
}

// Asserts that exception is of `expected_type` type.
fn assert_exception_type(env: &JNIEnv, exception: JThrowable, expected_type: &str) {
    assert!(env.is_instance_of(exception, expected_type).unwrap());
}

// Asserts that exception's message is `expected_message`.
fn assert_exception_message(env: &JNIEnv, exception: JThrowable, expected_message: &str) {
    let message = env
        .call_method(exception, "getMessage", "()Ljava/lang/String;", &[])
        .unwrap()
        .l()
        .unwrap();
    let msg_rust: String = env.get_string(message.into()).unwrap().into();
    assert_eq!(msg_rust, expected_message);
}
