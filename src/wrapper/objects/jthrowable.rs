use crate::{
    objects::JObject,
    sys::{jobject, jthrowable},
};

/// Lifetime'd representation of a `jthrowable`. Just a `JObject` wrapped in a
/// new class.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct JThrowable<'a>(JObject<'a>);

impl<'a> ::std::ops::Deref for JThrowable<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<JThrowable<'a>> for JObject<'a> {
    fn from(other: JThrowable) -> JObject {
        other.0
    }
}

impl<'a> From<JObject<'a>> for JThrowable<'a> {
    fn from(other: JObject) -> Self {
        unsafe { Self::from_raw(other.into_raw()) }
    }
}

impl<'a> std::default::Default for JThrowable<'a> {
    fn default() -> Self {
        Self(JObject::null())
    }
}

impl<'a> JThrowable<'a> {
    /// Creates a [`JThrowable`] that wraps the given `raw` [`jthrowable`]
    ///
    /// # Safety
    ///
    /// Expects a valid pointer or `null`
    pub unsafe fn from_raw(raw: jthrowable) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub fn into_raw(self) -> jthrowable {
        self.0.into_raw() as jthrowable
    }
}
