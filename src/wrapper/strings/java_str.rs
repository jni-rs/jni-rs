use std::os::raw::c_char;

use std::borrow::Cow;

use JNIEnv;

use errors::*;

use sys::{jstring, jboolean};

use strings::JNIStr;

/// Reference to a string in the JVM. Holds a pointer to the array
/// returned by GetStringUTFChars. Calls ReleaseStringUTFChars on Drop.
/// Can be converted to a `&JNIStr` with the same cost as the `&CStr.from_ptr`
/// conversion.
pub struct JavaStr<'a> {
    internal: *const c_char,
    obj: jstring,
    env: &'a JNIEnv<'a>,
}

impl<'a> JavaStr<'a> {
    pub fn from_env(env: &'a JNIEnv, obj: jstring) -> Result<Self> {
        let ptr: *const c_char = jni_call!(env.internal,
                      GetStringUTFChars,
                      obj,
                      ::std::ptr::null::<jboolean>() as *mut jboolean);
        let java_str = JavaStr {
            internal: ptr,
            env: env,
            obj: obj,
        };
        Ok(java_str)
    }
}

impl<'a> ::std::ops::Deref for JavaStr<'a> {
    type Target = JNIStr;
    fn deref(&self) -> &Self::Target {
        self.into()
    }
}

impl<'a> From<&'a JavaStr<'a>> for &'a JNIStr {
    fn from(other: &'a JavaStr) -> &'a JNIStr {
        unsafe { JNIStr::from_ptr(other.internal) }
    }
}

impl<'a> From<&'a JavaStr<'a>> for Cow<'a, str> {
    fn from(other: &'a JavaStr) -> Cow<'a, str> {
        let jni_str: &JNIStr = &*other;
        jni_str.into()
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
