use jni_sys::{jboolean, JNI_TRUE};
use std::{borrow::Cow, os::raw::c_char};

use log::warn;

use crate::{errors::*, objects::JString, strings::JNIStr, JNIEnv};

/// Reference to a string in the JVM. Holds a pointer to the array
/// returned by `GetStringUTFChars`. Calls `ReleaseStringUTFChars` on Drop.
/// Can be converted to a `&JNIStr` with the same cost as the `&CStr.from_ptr`
/// conversion.
pub struct JavaStr<'a: 'b, 'b> {
    internal: *const c_char,
    obj: JString<'a>,
    env: &'b JNIEnv<'a>,
}

impl<'a: 'b, 'b> JavaStr<'a, 'b> {
    /// Get a pointer to the character array beneath a [JString]
    ///
    /// The string will be `NULL` terminated and encoded as
    /// [Modified UTF-8](https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8) /
    /// [CESU-8](https://en.wikipedia.org/wiki/CESU-8).
    ///
    /// The implementation may either create a copy of the character array for
    /// the given `String` or it may pin it to avoid it being collected by the
    /// garbage collector.
    ///
    /// Returns a tuple with the pointer and the status of whether the implementation
    /// created a copy of the underlying character array.
    ///
    /// # Warning
    ///
    /// The caller must release the array when they are done with it via
    /// [Self::release_string_utf_chars]
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the Object passed in is an instance of `java.lang.String`,
    /// passing in anything else will lead to undefined behaviour (The JNI implementation
    /// is likely to crash or abort the process).
    unsafe fn get_string_utf_chars(
        env: &JNIEnv<'_>,
        obj: JString<'_>,
    ) -> Result<(*const c_char, bool)> {
        non_null!(obj, "get_string_utf_chars obj argument");
        let mut is_copy: jboolean = 0;
        let ptr: *const c_char = jni_non_null_call!(
            env.get_raw(),
            GetStringUTFChars,
            obj.into_raw(),
            &mut is_copy as *mut _
        );

        let is_copy = is_copy == JNI_TRUE;
        Ok((ptr, is_copy))
    }

    /// Release the backing string
    ///
    /// This will either free the copy that was made by `GetStringUTFChars` or unpin it so it
    /// may be released by the garbage collector once there are no further references to the string.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that [Self::internal] was constructed from a valid pointer obtained from [Self::get_string_utf_chars]
    unsafe fn release_string_utf_chars(&mut self) -> Result<()> {
        non_null!(self.obj, "release_string_utf_chars obj argument");
        // This method is safe to call in case of pending exceptions (see the chapter 2 of the spec)
        jni_unchecked!(
            self.env.get_raw(),
            ReleaseStringUTFChars,
            self.obj.into_raw(),
            self.internal
        );

        Ok(())
    }

    /// Get a [JavaStr] from a [JNIEnv] and a [JString].
    /// You probably want [JNIEnv::get_string] instead of this method.
    pub fn from_env(env: &'b JNIEnv<'a>, obj: JString<'a>) -> Result<Self> {
        let (ptr, _) = unsafe { Self::get_string_utf_chars(env, obj)? };
        let java_str = JavaStr {
            internal: ptr,
            env,
            obj,
        };
        Ok(java_str)
    }

    /// Get the raw string pointer from the JavaStr.
    ///
    /// The string will be `NULL` terminated and encoded as
    /// [Modified UTF-8](https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8) /
    /// [CESU-8](https://en.wikipedia.org/wiki/CESU-8).
    pub fn get_raw(&self) -> *const c_char {
        self.internal
    }

    /// Consumes the `JavaStr`, returning the raw string pointer
    ///
    /// The string will be `NULL` terminated and encoded as
    /// [Modified UTF-8](https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8) /
    /// [CESU-8](https://en.wikipedia.org/wiki/CESU-8).
    ///
    /// # Warning
    /// The programmer is responsible for making sure the backing string gets
    /// released when they are done with it, for example by reconstructing a
    /// [JavaStr] with [`Self::from_raw`], which will release the backing string
    /// when it is dropped.
    pub fn into_raw(self) -> *const c_char {
        let _dont_call_drop = std::mem::ManuallyDrop::new(self);
        _dont_call_drop.internal
    }

    /// Get a [JavaStr] from it's raw components
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `ptr` is a valid, non-null pointer returned by [`Self::into_raw`],
    /// and that `obj` is the same `String` object originally used to create the [JavaStr]
    ///
    /// # Example
    /// ```ignore
    /// # use jni::strings::JavaStr;
    ///
    /// let jstring = env.new_string("foo").unwrap();
    /// let java_str = env.get_string(jstring).unwrap();
    ///
    /// let ptr = java_str.into_raw();
    /// // Do whatever you need with the pointer
    /// let java_str = unsafe { JavaStr::from_raw(env, jstring, ptr) };
    /// ```
    pub unsafe fn from_raw(env: &'b JNIEnv<'a>, obj: JString<'a>, ptr: *const c_char) -> Self {
        Self {
            internal: ptr,
            obj,
            env,
        }
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
        let jni_str: &JNIStr = other;
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
        match unsafe { self.release_string_utf_chars() } {
            Ok(()) => {}
            Err(e) => warn!("error dropping java str: {}", e),
        }
    }
}
