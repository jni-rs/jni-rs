extern crate libc;

mod bindgen;
use bindgen::*;

use std::ffi;
use std::str;


#[repr(C)]
pub struct JEnv(*mut JNIEnv);

impl JEnv {
    fn find_class<S: Into<String>>(&self, name: S) -> jclass {
        let jni_env = self.0;
        let mut name_null_term = name.into();
        name_null_term.push_str("\0");
        unsafe { (**jni_env).FindClass.unwrap()(jni_env, name_null_term.as_ptr() as *const i8)}
    }

    fn get_string(&self, str_obj: jstring) -> &str {
        let jni_env = self.0;
        let mut copy = false as jboolean;

        unsafe { str::from_utf8(ffi::CStr::from_ptr((**jni_env).GetStringUTFChars.unwrap()(jni_env, str_obj, &mut copy)).to_bytes()).unwrap() }
    }
}

#[no_mangle]
pub extern "C" fn Java_HelloWorld_nativeProtect(arg1: JEnv, arg2: jobject, input: jstring) -> jobject {
    let string = arg1.get_string(input);
    println!("String from java: {}", string);

    arg1.find_class("com/prevoty/commons/content/ProtectResult")
}


