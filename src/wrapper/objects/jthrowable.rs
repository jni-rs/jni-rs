use crate::{
    objects::JObject,
    sys::{jobject, jthrowable},
};

/// Lifetime'd representation of a `jthrowable`. Just a `JObject` wrapped in a
/// new class.
#[repr(transparent)]
pub struct JThrowable<'local>(JObject<'local>);

impl<'local> AsRef<JThrowable<'local>> for JThrowable<'local> {
    fn as_ref(&self) -> &JThrowable<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JThrowable<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JThrowable<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JThrowable<'local>> for JObject<'local> {
    fn from(other: JThrowable) -> JObject {
        other.0
    }
}

impl<'local> From<JObject<'local>> for JThrowable<'local> {
    fn from(other: JObject) -> Self {
        unsafe { Self::from_raw(other.into_raw()) }
    }
}

impl<'local, 'obj_ref> From<&'obj_ref JObject<'local>> for &'obj_ref JThrowable<'local> {
    fn from(other: &'obj_ref JObject<'local>) -> Self {
        // Safety: `JThrowable` is `repr(transparent)` around `JObject`.
        unsafe { &*(other as *const JObject<'local> as *const JThrowable<'local>) }
    }
}

impl<'local> std::default::Default for JThrowable<'local> {
    fn default() -> Self {
        Self(JObject::null())
    }
}

impl<'local> JThrowable<'local> {
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
