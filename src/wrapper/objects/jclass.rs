use crate::{
    objects::JObject,
    sys::{jclass, jobject},
};

/// Lifetime'd representation of a `jclass`. Just a `JObject` wrapped in a new
/// class.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct JClass<'a>(JObject<'a>);

impl<'a> ::std::ops::Deref for JClass<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<JClass<'a>> for JObject<'a> {
    fn from(other: JClass) -> JObject {
        other.0
    }
}

/// This conversion assumes that the `JObject` is a pointer to a class object.
impl<'a> From<JObject<'a>> for JClass<'a> {
    fn from(other: JObject) -> Self {
        unsafe { Self::from_raw(other.into_raw()) }
    }
}

impl<'a> std::default::Default for JClass<'a> {
    fn default() -> Self {
        Self(JObject::null())
    }
}

impl<'a> JClass<'a> {
    /// Creates a [`JClass`] that wraps the given `raw` [`jclass`]
    ///
    /// # Safety
    ///
    /// Expects a valid pointer or `null`
    pub unsafe fn from_raw(raw: jclass) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub fn into_raw(self) -> jclass {
        self.0.into_raw() as jclass
    }
}
