use std::os::raw::c_char;

use std::borrow::Cow;

use JNIEnv;

use objects::JString;

use errors::*;

use strings::JNIStr;

/// Reference to a string in the JVM. Holds a pointer to the array
/// returned by GetStringUTFChars. Calls ReleaseStringUTFChars on Drop.
/// Can be converted to a `&JNIStr` with the same cost as the `&CStr.from_ptr`
/// conversion.
pub struct JavaStr<'a> {
    internal: *const c_char,
    obj: JString<'a>,
    env: &'a JNIEnv<'a>,
}

impl<'a> JavaStr<'a> {
    /// Build a `JavaStr` from an object and a reference to the environment. You
    /// probably want to use `JNIEnv::get_string` instead.
    pub fn from_env(env: &'a JNIEnv<'a>, obj: JString<'a>) -> Result<Self> {
        let ptr = unsafe { env.get_string_utf_chars(obj)? };
        let java_str = JavaStr {
            internal: ptr,
            env: env,
            obj: obj,
        };
        Ok(java_str)
    }

    /// Extract the raw C string pointer from the JavaStr. This will be
    /// encoded using the JVM internal `CESU-8`-style.
    pub fn get_raw(&self) -> *const c_char {
        self.internal
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
        match unsafe { self.env.release_string_utf_chars(self.obj, self.internal) } {
            Ok(()) => {}
            Err(e) => warn!("error dropping java str: {}", e),
        }
    }
}
