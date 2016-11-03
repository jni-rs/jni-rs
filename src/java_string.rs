use std::marker::PhantomData;
use jni_sys::{jsize, jstring, jboolean};
use jnienv::JNIEnv;
use std::os::raw::c_char;

use std::borrow::ToOwned;
use std::borrow::Cow;

use cesu8::from_java_cesu8;

use errors::*;

// borrowed version of a java string. Holds a pointer to the array
// returned by GetStringUTFChars. Calls ReleaseStringUTFChars on Drop.
pub struct JavaStr<'a> {
    internal: *const c_char,
    length: jsize,
    obj: jstring,
    env: &'a JNIEnv<'a>,
}

impl<'a> JavaStr<'a> {
    pub fn from_env(env: &'a JNIEnv, obj: jstring) -> Result<Self> {
        let ptr: *const c_char = jni_call!(env.internal,
                      GetStringUTFChars,
                      obj,
                      ::std::ptr::null::<jboolean>() as *mut jboolean);
        let java_str = unsafe {
            let len: jsize =
                jni_unchecked!(env.internal, GetStringUTFLength, obj);
            JavaStr {
                internal: ptr,
                length: len,
                env: env,
                obj: obj,
            }
        };
        Ok(java_str)
    }
}

impl<'a> From<&'a JavaStr<'a>> for &'a [u8] {
    fn from(other: &JavaStr) -> &'a [u8] {
        unsafe {
            ::std::slice::from_raw_parts(other.internal as *const u8,
                                         other.length as usize)
        }
    }
}

impl<'a> From<&'a JavaStr<'a>> for Cow<'a, str> {
    fn from(other: &'a JavaStr) -> Cow<'a, str> {
        let slice: &[u8] = other.into();
        // TODO make sure this is ok
        from_java_cesu8(slice).unwrap()
    }
}

impl<'a> From<JavaStr<'a>> for String {
    fn from(other: JavaStr) -> String {
        let cow: Cow<str> = (&other).into();
        cow.into_owned()
    }
}

impl<'a> Drop for JavaStr<'a> {
    fn drop(&mut self) {
        match destroy_java_string(self.env, self.obj, self.internal) {
            Ok(()) => {}
            Err(e) => warn!("error dropping java str: {}", e),
        }
    }
}

fn destroy_java_string(env: &JNIEnv,
                       obj: jstring,
                       array: *const c_char)
                       -> Result<()> {
    unsafe { jni_unchecked!(env.internal, ReleaseStringUTFChars, obj, array) };
    Ok(())
}
