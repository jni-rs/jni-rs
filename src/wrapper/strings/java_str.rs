use std::{borrow::Cow, os::raw::c_char};

use log::warn;

use crate::{errors::*, objects::JString, strings::JNIStr, JNIEnv};

/// Reference to a string in the JVM. Holds a pointer to the array
/// returned by GetStringUTFChars. Calls ReleaseStringUTFChars on Drop.
/// Can be converted to a `&JNIStr` with the same cost as the `&CStr.from_ptr`
/// conversion.
pub struct JavaStr<'a: 'b, 'b> {
    internal: *const c_char,
    obj: JString<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JavaStr<'a, 'b> {
    /// Build a `JavaStr` from an object and a reference to the environment. You
    /// probably want to use `JNIEnv::get_string` instead.
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JString<'a>) -> Result<Self> {
        let ptr = env.get_string_utf_chars(obj)?;
        let java_str = JavaStr {
            internal: ptr,
            env,
            obj,
        };
        Ok(java_str)
    }

    /// Extract the raw C string pointer from the JavaStr. This will be
    /// encoded using the JVM internal `CESU-8`-style.
    pub fn get_raw(&self) -> *const c_char {
        self.internal
    }
}

impl<'a: 'b, 'b> ::std::ops::Deref for JavaStr<'a, 'b> {
    type Target = JNIStr;
    fn deref(&self) -> &Self::Target {
        self.into()
    }
}

impl<'a: 'b, 'b: 'c, 'c> From<&'c JavaStr<'a, 'b>> for &'c JNIStr {
    fn from(other: &'c JavaStr) -> &'c JNIStr {
        unsafe { JNIStr::from_ptr(other.internal) }
    }
}

impl<'a: 'b, 'b: 'c, 'c> From<&'c JavaStr<'a, 'b>> for Cow<'c, str> {
    fn from(other: &'c JavaStr) -> Cow<'c, str> {
        let jni_str: &JNIStr = &*other;
        jni_str.into()
    }
}

impl<'a: 'b, 'b> From<JavaStr<'a, 'b>> for String {
    fn from(other: JavaStr) -> String {
        let cow: Cow<str> = (&other).into();
        cow.into_owned()
    }
}

impl<'a: 'b, 'b> Drop for JavaStr<'a, 'b> {
    fn drop(&mut self) {
        match unsafe { self.env.release_string_utf_chars(self.obj, self.internal) } {
            Ok(()) => {}
            Err(e) => warn!("error dropping java str: {}", e),
        }
    }
}
