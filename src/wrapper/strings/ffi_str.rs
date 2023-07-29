use std::{
    borrow::{Borrow, Cow, ToOwned},
    ffi::{CStr, CString},
    os::raw::c_char,
};

use cesu8::{from_java_cesu8, to_java_cesu8};
use log::debug;

use crate::wrapper::strings::ffi_str;

#[cfg(doc)]
use std::ops::Deref;

/// An owned, null-terminated string, encoded in Java's [Modified UTF-8].
///
/// Most JNI functions that accept or return strings, such as [`NewStringUTF`],
/// expect or produce strings encoded this way.
///
/// This type plays a similar role as [`CString`]. Its borrowed counterpart is
/// [`JNIStr`], which this type [dereferences][Deref] to.
///
/// Ordinary Rust strings ([`String`], <code>&amp;[str]</code>, or any other
/// type implementing <code>[AsRef]&lt;str&gt;</code>) can be converted to this
/// type using `.into()`. Specifically, this type implements
/// <code>[From]&lt;T&gt; where T: AsRef&lt;str&gt;</code>.
///
/// [Modified UTF-8]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/types.html#modified-utf-8-strings
/// [`NewStringUTF`]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/functions.html#newstringutf
pub struct JNIString {
    internal: CString,
}

/// A borrowed, null-terminated string, encoded in Java's [Modified UTF-8].
///
/// Most JNI functions that accept or return strings, such as [`NewStringUTF`],
/// expect or produce strings encoded this way.
///
/// This type plays a similar role as (and [dereferences][Deref] to) [`CStr`].
/// Its owned counterpart is [`JNIString`].
///
///
/// # Instantiating
///
/// There are two main ways to create a `JNIStr` from a string.
///
/// The simplest way is to convert an ordinary Rust string to `JNIString`,
/// which implements <code>[From]&lt;&amp;[str]&gt;</code> and dereferences to
/// this type. The downside is that this conversion has a run-time cost.
///
/// If you have a `CStr` that you are certain is already encoded in Modified
/// UTF-8, you can instead use [`JNIStr::from_cstr_unchecked`] to convert it
/// to a `JNIStr` at no run-time cost. The downside is that this is `unsafe`;
/// see the “safety” section of that method's documentation for details.
///
/// [Modified UTF-8]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/types.html#modified-utf-8-strings
/// [`NewStringUTF`]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/functions.html#newstringutf
pub struct JNIStr {
    internal: CStr,
}

impl ::std::ops::Deref for JNIString {
    type Target = JNIStr;

    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.internal.as_bytes_with_nul() as *const [u8] as *const ffi_str::JNIStr) }
    }
}

impl ::std::ops::Deref for JNIStr {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<T> From<T> for JNIString
where
    T: AsRef<str>,
{
    fn from(other: T) -> Self {
        let enc = to_java_cesu8(other.as_ref()).into_owned();
        JNIString {
            internal: unsafe { CString::from_vec_unchecked(enc) },
        }
    }
}

impl<'str_ref> From<&'str_ref JNIStr> for Cow<'str_ref, str> {
    fn from(other: &'str_ref JNIStr) -> Cow<'str_ref, str> {
        let bytes = other.to_bytes();
        match from_java_cesu8(bytes) {
            Ok(s) => s,
            Err(e) => {
                debug!("error decoding java cesu8: {:#?}", e);
                String::from_utf8_lossy(bytes)
            }
        }
    }
}

impl From<JNIString> for String {
    fn from(other: JNIString) -> String {
        Cow::from(other.borrowed()).into_owned()
    }
}

impl JNIString {
    /// Get the borrowed version of the JNIString. Equivalent to
    /// `CString::borrowed`.
    pub fn borrowed(&self) -> &JNIStr {
        self
    }
}

impl JNIStr {
    /// Construct a reference to a `JNIStr` from a pointer. Equivalent to `CStr::from_ptr`.
    ///
    /// # Safety
    ///
    /// Expects a valid pointer to a null-terminated C string and does not perform any lifetime
    /// checks for the resulting value.
    pub unsafe fn from_ptr<'jni_str>(ptr: *const c_char) -> &'jni_str JNIStr {
        &*(CStr::from_ptr(ptr) as *const CStr as *const JNIStr)
    }

    /// Converts a `&CStr` to a `&JNIStr` without checking for validity.
    ///
    /// # Safety
    ///
    /// The provided string must be encoded in Java's [Modified UTF-8].
    /// Undefined behavior will result if it is not.
    ///
    /// Note that standard UTF-8 has the same encoding as Modified UTF-8 for the code points U+0001 through U+FFFF (inclusive).
    ///
    /// [Modified UTF-8]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/types.html#modified-utf-8-strings
    pub const unsafe fn from_cstr_unchecked(cstr: &CStr) -> &JNIStr {
        // The reason we don't just use `from_ptr` here is that `CStr::from_ptr` is not yet a `const fn`.
        &*(cstr as *const CStr as *const JNIStr)
    }
}

// impls for CoW
impl Borrow<JNIStr> for JNIString {
    fn borrow(&self) -> &JNIStr {
        self
    }
}

impl ToOwned for JNIStr {
    type Owned = JNIString;

    fn to_owned(&self) -> JNIString {
        unsafe {
            JNIString {
                internal: CString::from_vec_unchecked(self.to_bytes().to_vec()),
            }
        }
    }
}
