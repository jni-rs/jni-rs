use std::os::raw::c_char;

use std::ffi;

use std::borrow::{Cow, ToOwned, Borrow};

use cesu8::from_java_cesu8;
use cesu8::to_java_cesu8;

/// Wrapper for `std::ffi::CString` that also takes care of encoding between
/// UTF-8 and Java's Modified UTF-8. As with `CString`, this implements `Deref`
/// to `&JNIStr`.
#[derive(Clone)]
pub struct JNIString {
    internal: ffi::CString,
}

/// Wrapper for `std::ffi::CStr` that also takes care of encoding between
/// UTF-8 and Java's Modified UTF-8.
pub struct JNIStr {
    internal: ffi::CStr,
}

impl ::std::ops::Deref for JNIString {
    type Target = JNIStr;

    fn deref(&self) -> &Self::Target {
        unsafe { ::std::mem::transmute(self.internal.as_bytes_with_nul()) }
    }
}

impl ::std::ops::Deref for JNIStr {
    type Target = ffi::CStr;

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
        JNIString { internal: unsafe { ffi::CString::from_vec_unchecked(enc) } }
    }
}

impl<'a> From<&'a JNIStr> for Cow<'a, str> {
    fn from(other: &'a JNIStr) -> Cow<'a, str> {
        let bytes = other.to_bytes();
        match from_java_cesu8(bytes) {
            Ok(s) => s,
            Err(e) => {
                debug!("error decoding java cesu8: {:#?}", e);
                String::from_utf8_lossy(bytes).into()
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
    /// Construct a reference to a `JNIStr` from a pointer. Equivalent to
    /// `CStr::from_ptr`.
    pub unsafe fn from_ptr<'a>(ptr: *const c_char) -> &'a JNIStr {
        ::std::mem::transmute(ffi::CStr::from_ptr(ptr))
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
                internal: ffi::CString::from_vec_unchecked(
                    self.to_bytes().to_vec(),
                ),
            }
        }
    }
}
