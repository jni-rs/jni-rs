use crate::{
    objects::JObject,
    sys::{jobject, jstring},
};

/// Lifetime'd representation of a `jstring`. Just a `JObject` wrapped in a new
/// class.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct JString<'a>(JObject<'a>);

impl<'a> ::std::ops::Deref for JString<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<JString<'a>> for JObject<'a> {
    fn from(other: JString) -> JObject {
        other.0
    }
}

impl<'a> From<JObject<'a>> for JString<'a> {
    fn from(other: JObject) -> Self {
        unsafe { Self::from_raw(other.into_raw()) }
    }
}

impl<'a> std::default::Default for JString<'a> {
    fn default() -> Self {
        Self(JObject::null())
    }
}

impl<'a> JString<'a> {
    /// Creates a [`JString`] that wraps the given `raw` [`jstring`]
    ///
    /// # Safety
    ///
    /// Expects a valid pointer or `null`
    pub unsafe fn from_raw(raw: jstring) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub fn into_raw(self) -> jstring {
        self.0.into_raw() as jstring
    }
}
