use crate::{objects::JObject, sys::jobject};

/// Lifetime'd representation of a `jobject` that is an instance of the
/// ByteBuffer Java class. Just a `JObject` wrapped in a new class.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct JByteBuffer<'a>(JObject<'a>);

impl<'a> ::std::ops::Deref for JByteBuffer<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<JByteBuffer<'a>> for JObject<'a> {
    fn from(other: JByteBuffer) -> JObject {
        other.0
    }
}

impl<'a> From<JObject<'a>> for JByteBuffer<'a> {
    fn from(other: JObject) -> Self {
        unsafe { Self::from_raw(other.into_raw()) }
    }
}

impl<'a> std::default::Default for JByteBuffer<'a> {
    fn default() -> Self {
        Self(JObject::null())
    }
}

impl<'a> JByteBuffer<'a> {
    /// Creates a [`JByteBuffer`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    /// No runtime check is made to verify that the given [`jobject`] is an instance of
    /// a `ByteBuffer`.
    pub unsafe fn from_raw(raw: jobject) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub fn into_raw(self) -> jobject {
        self.0.into_raw() as jobject
    }
}
