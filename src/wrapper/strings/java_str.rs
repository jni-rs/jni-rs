use jni_sys::{jboolean, JNI_TRUE};
use std::{borrow::Cow, marker::PhantomData, os::raw::c_char};

use log::warn;

use crate::{
    env::JNIEnv,
    errors::*,
    objects::JString,
    strings::{JNIStr, JNIString},
    JavaVM,
};

/// Represents the bytes of a string in the JVM, in Java's [modified UTF-8]
/// encoding.
///
/// This type is returned by [`JNIEnv::get_string`]. It can be used to convert
/// a Java string into a Rust string (in standard UTF-8 encoding) with the
/// [`to_str`][JNIStr::to_str] method, and to get the bytes of the string in
/// modified UTF-8 encoding using the [`as_cstr`][JNIStr::as_cstr] method.
///
/// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
///
///
/// # Relationships with Other Types
///
/// The borrowed form of this type is [`JNIStr`], whose relationship with this
/// type is similar to the relationship between [`str`] and [`String`].
///
/// This is related to, but different from, the [`JNIString`] type. A
/// `JNIString` is created and owned by Rust code, whereas a `JavaStr`
/// represents a string owned by the JVM and merely borrowed by Rust code.
///
/// This is not to be confused with [`JString`]. That refers to a
/// `java.lang.String` object, whereas this refers to the bytes of the string.
pub struct JavaStr<'local, 'other_local: 'obj_ref, 'obj_ref> {
    internal: *const c_char,
    obj: &'obj_ref JString<'other_local>,
    is_copy: bool,
    _lifetime: PhantomData<&'local ()>,
}

impl<'local, 'other_local: 'obj_ref, 'obj_ref> JavaStr<'local, 'other_local, 'obj_ref> {
    /// Constructs a `JavaStr` from a `JNIEnv` and a `JString`.
    ///
    /// The string will be `NULL` terminated and encoded as [Modified
    /// UTF-8](https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8) /
    /// [CESU-8](https://en.wikipedia.org/wiki/CESU-8).
    ///
    /// The implementation may either create a copy of the character array for
    /// the given `String` or it may pin it to avoid it being collected by the
    /// garbage collector.
    ///
    /// Returns a `JavaStr` that will automatically release the underlying
    /// character array when dropped (see [Self::release_string_utf_chars]).
    pub(crate) fn from_get_string_utf_chars(
        env: &JNIEnv<'local>,
        obj: &'obj_ref JString<'other_local>,
    ) -> Result<Self> {
        let obj = null_check!(obj, "get_string_utf_chars obj argument")?;

        // SAFETY:
        // - We have checked that the object is not null.
        // - Having a `JString` guarantees that the reference is for a `java.lang.String`
        //   (it would require unsafe code for that to be violated)
        // - The pointer is immediately wrapped to ensure that the pointer will
        //   be released when dropped.
        unsafe {
            let mut is_copy: jboolean = false;
            let ptr: *const c_char = jni_call_only_check_null_ret!(
                env,
                v1_1,
                GetStringUTFChars,
                obj.as_raw(),
                &mut is_copy as *mut _
            )?;

            let is_copy = is_copy == JNI_TRUE;
            Ok(Self::from_raw(env, obj, ptr, is_copy))
        }
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
        let obj = null_check!(self.obj, "release_string_utf_chars obj argument")?;
        // Panic: Since we can't construct a `JavaStr` without a valid `JNIEnv` reference we know
        // `JavaVM::singleton()` must be initialized and won't panic.
        JavaVM::singleton()?.with_env_current_frame(|env| {
            // This method is safe to call in case of pending exceptions (see the chapter 2 of the spec)
            jni_call_unchecked!(
                env,
                v1_1,
                ReleaseStringUTFChars,
                obj.as_raw(),
                self.internal
            );

            Ok(())
        })
    }

    /// Destroys the `JavaStr` without freeing the underlying string, and
    /// returns a raw pointer to it.
    ///
    /// The returned pointer is the same as the one returned by the
    /// [`as_ptr`][JNIStr::as_ptr] method. It points to a null-terminated
    /// string in [modified UTF-8] encoding (which is similar, but not
    /// identical, to [CESU-8]). It is valid when returned by this method, and
    /// will remain valid until freed (see below).
    ///
    /// [CESU-8]: https://en.wikipedia.org/wiki/CESU-8
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    ///
    /// This also returns the status for whether the original string was copied or not.
    ///
    /// # Warning
    ///
    /// After calling this method, the underlying string must be manually
    /// freed. This can be done either by reconstructing the [`JavaStr`] using
    /// [`JavaStr::from_raw`] and then dropping it, or by passing the pointer
    /// to the JNI function `ReleaseStringUTFChars`.
    pub fn into_raw(self) -> (*const c_char, bool) {
        let _dont_call_drop = std::mem::ManuallyDrop::new(self);
        (_dont_call_drop.internal, _dont_call_drop.is_copy)
    }

    /// Constructs a [`JavaStr`] from raw components.
    ///
    /// The required components are the current `JNIEnv`, a reference to a
    /// `java.lang.String` object, and a pointer to the characters of that
    /// `java.lang.String`.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid, non-null pointer, previously returned by
    /// [`JavaStr::into_raw`] or the JNI function `GetStringUTFChars`. `ptr`
    /// must not belong to another `JavaStr` at the same time.
    ///
    /// `obj` must be a non-null reference to the same `java.lang.String` object
    /// that was originally passed to [`JNIEnv::get_string`], or the JNI
    /// function `GetStringUTFChars`, in order to obtain `ptr`.
    ///
    /// `is_copy` must be a boolean indicating whether the string was copied
    /// or not (as returned by `GetStringUTFChars`).
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, env::JNIEnv, strings::JavaStr};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
    /// let jstring = env.new_string(c"foo")?;
    /// let java_str = env.get_string(&jstring)?;
    ///
    /// let (ptr, is_copy) = java_str.into_raw();
    /// // Do whatever you need with the pointer
    /// let java_str = unsafe { JavaStr::from_raw(env, &jstring, ptr, is_copy) };
    /// # Ok(())
    /// # }
    /// ```
    pub unsafe fn from_raw(
        _env: &JNIEnv<'local>,
        obj: &'obj_ref JString<'other_local>,
        ptr: *const c_char,
        is_copy: bool,
    ) -> Self {
        Self {
            internal: ptr,
            obj,
            is_copy,
            _lifetime: PhantomData,
        }
    }

    /// Returns whether the string was copied or not.
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }
}

impl<'other_local: 'obj_ref, 'obj_ref> ::std::ops::Deref for JavaStr<'_, 'other_local, 'obj_ref> {
    type Target = JNIStr;
    fn deref(&self) -> &Self::Target {
        self.into()
    }
}

impl<'other_local: 'obj_ref, 'obj_ref: 'java_str, 'java_str>
    From<&'java_str JavaStr<'_, 'other_local, 'obj_ref>> for &'java_str JNIStr
{
    fn from(other: &'java_str JavaStr) -> &'java_str JNIStr {
        unsafe { JNIStr::from_ptr(other.internal) }
    }
}

impl<'other_local: 'obj_ref, 'obj_ref> From<JavaStr<'_, 'other_local, 'obj_ref>> for JNIString {
    fn from(other: JavaStr) -> JNIString {
        let jni_str: &JNIStr = &other;
        jni_str.to_owned()
    }
}

impl<'other_local: 'obj_ref, 'obj_ref: 'java_str, 'java_str>
    From<&'java_str JavaStr<'_, 'other_local, 'obj_ref>> for Cow<'java_str, str>
{
    fn from(other: &'java_str JavaStr) -> Cow<'java_str, str> {
        let jni_str: &JNIStr = other;
        jni_str.into()
    }
}

impl<'other_local: 'obj_ref, 'obj_ref> From<JavaStr<'_, 'other_local, 'obj_ref>> for String {
    fn from(other: JavaStr) -> String {
        let cow: Cow<str> = (&other).into();
        cow.into_owned()
    }
}

impl<'other_local: 'obj_ref, 'obj_ref> Drop for JavaStr<'_, 'other_local, 'obj_ref> {
    fn drop(&mut self) {
        match unsafe { self.release_string_utf_chars() } {
            Ok(()) => {}
            Err(e) => warn!("error dropping java str: {}", e),
        }
    }
}

impl<'other_local: 'obj_ref, 'obj_ref> AsRef<JNIStr> for JavaStr<'_, 'other_local, 'obj_ref> {
    fn as_ref(&self) -> &JNIStr {
        self
    }
}
