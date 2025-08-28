use std::{
    borrow::{Borrow, Cow, ToOwned},
    ffi::{CStr, CString},
    os::raw::c_char,
};

use cesu8::{from_java_cesu8, to_java_cesu8};
use log::debug;

#[cfg(doc)]
use crate::wrapper::strings::JavaStr;

/// An owned null-terminated string (like [`CString`]) encoded in Java's
/// [modified UTF-8].
///
/// This type is intended for constructing Java strings from Rust code. To use
/// it, first construct an ordinary Rust [`str`] or [`String`], then use
/// [`JNIString::new`] to convert it to the encoding that Java expects.
///
/// As with `CString`, this type has a borrowed counterpart, [`JNIStr`], that
/// it coerces to.
///
/// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
pub struct JNIString {
    internal: CString,
}

/// A borrowed null-terminated string (like [`CStr`]) encoded in Java's
/// [modified UTF-8].
///
/// As with `CStr` and [`str`], instances of `JNIStr` cannot be created
/// directly, but can be borrowed from either [`JNIString`] (for constructing a
/// Java string in Rust code) or [`JavaStr`] (which refers to an existing Java
/// string).
///
/// To convert a `JNIStr` into an ordinary Rust string, use the
/// [`to_str`][Self::to_str] method. To get a view of the modified UTF-8
/// encoding of the `JNIStr`, use the [`as_cstr`][Self::as_cstr] method to get
/// a `CStr`, then call its [`to_bytes`][CStr::to_bytes] method.
///
/// Note that, as with `CStr`, this type is **not** `repr(C)`. See [the
/// `CStr` documentation][CStr] for an explanation of what that means. (This
/// type is `repr(transparent)`, but it wraps around a `CStr`, not a raw
/// pointer.)
///
/// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
#[repr(transparent)] // needed because `JNIStr` gets pointer-punned from `CStr`.
pub struct JNIStr {
    internal: CStr,
}

impl ::std::ops::Deref for JNIString {
    type Target = JNIStr;

    fn deref(&self) -> &Self::Target {
        unsafe { JNIStr::from_ptr(self.internal.as_ptr()) }
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

impl From<JNIString> for CString {
    fn from(string: JNIString) -> Self {
        string.into_cstring()
    }
}

impl<'str_ref> From<&'str_ref JNIStr> for Cow<'str_ref, str> {
    fn from(other: &'str_ref JNIStr) -> Cow<'str_ref, str> {
        let bytes = other.as_cstr().to_bytes();
        match from_java_cesu8(bytes) {
            Ok(s) => s,
            Err(e) => {
                debug!("error decoding java cesu8: {:#?}", e);
                String::from_utf8_lossy(bytes)
            }
        }
    }
}

impl<'str_ref> From<&'str_ref JNIStr> for &'str_ref CStr {
    fn from(value: &'str_ref JNIStr) -> Self {
        &value.internal
    }
}

impl<'str_ref> From<&'str_ref JNIString> for Cow<'str_ref, JNIStr> {
    /// Converts `&JNIString` into `Cow::<&JNIStr>::Borrowed`. Zero-cost.
    fn from(string: &'str_ref JNIString) -> Self {
        Cow::Borrowed(string)
    }
}

impl From<JNIString> for String {
    fn from(other: JNIString) -> String {
        other.to_str().into_owned()
    }
}

impl JNIString {
    /// Converts a Rust string (in standard UTF-8 encoding) into a
    /// Java-compatible string (in Java's [modified UTF-8] encoding).
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    pub fn new(string: impl AsRef<str>) -> Self {
        string.into()
    }

    /// Converts a `CString` into a `JNIString`.
    ///
    /// This method is zero-cost.
    ///
    ///
    /// # Safety
    ///
    /// The `string` must be in [modified UTF-8] encoding.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    pub unsafe fn from_cstring(string: CString) -> Self {
        Self { internal: string }
    }

    /// Converts a `JNIString` into a `CString`.
    ///
    /// This method is zero-cost.
    pub fn into_cstring(self) -> CString {
        self.internal
    }

    /// Borrows this `JNIString` as a `&JNIStr`.
    ///
    /// This is the `JNIString` equivalent to [`CString::as_c_str`].
    ///
    /// Note that `&JNIString` also coerces to `&JNIStr`, even without calling
    /// this method. For example:
    ///
    /// ```rust,no_run
    /// # use jni::strings::{JNIStr, JNIString};
    /// let string: JNIString;
    /// # string = unimplemented!();
    ///
    /// // This works…
    /// let borrowed: &JNIStr = string.borrowed();
    ///
    /// // …and so does this.
    /// let borrowed: &JNIStr = &string;
    /// ```
    pub fn borrowed(&self) -> &JNIStr {
        self
    }
}

impl JNIStr {
    /// Constructs a reference to a `JNIStr` from a pointer.
    ///
    /// This is the [`JNIStr`] equivalent to [`CStr::from_ptr`].
    ///
    /// # Safety
    ///
    /// `ptr` must fulfill all of the safety requirements for `CStr::from_ptr`.
    /// See that method's documentation for details.
    ///
    /// Briefly, `ptr` must be a valid, non-null pointer to a null-terminated
    /// (C-style) string, and must not be mutated or become invalid during the
    /// lifetime `'a`.
    ///
    /// In addition, the string pointed to by `ptr` must be in [modified UTF-8]
    /// encoding.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    pub unsafe fn from_ptr<'a>(ptr: *const c_char) -> &'a JNIStr {
        &*(CStr::from_ptr(ptr) as *const CStr as *const JNIStr)
    }

    /// Returns a pointer to the string.
    ///
    /// The pointer points to a null-terminated string in [modified UTF-8]
    /// encoding. It is non-null and valid for as long as `self` is.
    ///
    /// This is equivalent to calling
    /// <code>self.[as_cstr][JNIStr::as_cstr]().[as_ptr][CStr::as_ptr]()</code>.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    pub fn as_ptr(&self) -> *const c_char {
        self.as_cstr().as_ptr()
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

    /// Returns a `CStr` view of the string.
    ///
    /// To get a view of the raw bytes of the string, call this method, then
    /// [`CStr::to_bytes`].
    ///
    ///
    /// # Warning: Not UTF-8
    ///
    /// Keep in mind that the returned `CStr` does *not* use standard UTF-8
    /// encoding. Instead, it uses Java's [modified UTF-8] encoding, which
    /// differs in how the code point U+0000, and code points greater than
    /// U+FFFF, are encoded.
    ///
    /// Do not call [`to_str`][CStr::to_str] or `to_string_lossy` on the `CStr`
    /// returned by this method. Doing so will not properly convert the
    /// encoding, potentially resulting in an error or a garbled string.
    ///
    /// To convert to a Rust string, use the [`JNIStr::to_str`] method instead.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    pub fn as_cstr(&self) -> &CStr {
        &self.internal
    }

    /// Converts this [modified UTF-8] string to an ordinary Rust string (which
    /// uses standard UTF-8 encoding).
    ///
    /// Standard UTF-8 and modified UTF-8 differ in how they encode the code
    /// point U+0000 and code points greater than U+FFFF. This method checks if
    /// the string contains any such code points, and converts them into
    /// standard UTF-8 encoding.
    ///
    /// If the string contains only code points between U+0001 and U+FFFF, then
    /// it does not need to be changed, and so this method will return
    /// [`Cow::Borrowed`]. Otherwise, this method will perform the conversion
    /// into a new string, and return [`Cow::Owned`].
    ///
    /// There is also an implementation of `From<&JNIStr>` for `Cow<str>`,
    /// which has the same effect as this method.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    pub fn to_str(&'_ self) -> Cow<'_, str> {
        self.into()
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
        JNIString {
            internal: CString::from(self.as_cstr()),
        }
    }
}

impl AsRef<JNIStr> for JNIStr {
    fn as_ref(&self) -> &JNIStr {
        self
    }
}

impl AsRef<JNIStr> for JNIString {
    fn as_ref(&self) -> &JNIStr {
        self
    }
}
