use crate::{
    objects::JObject,
    sys::{jobject, jthrowable},
};

use super::JObjectRef;

/// Lifetime'd representation of a `jthrowable`. Just a `JObject` wrapped in a
/// new class.
#[repr(transparent)]
#[derive(Default)]
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

impl JThrowable<'_> {
    /// Creates a [`JThrowable`] that wraps the given `raw` [`jthrowable`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jthrowable) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jthrowable {
        self.0.into_raw() as jthrowable
    }
}

impl JObjectRef for JThrowable<'_> {
    type Kind<'env> = JThrowable<'env>;
    type GlobalKind = JThrowable<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JThrowable::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JThrowable::from_raw(global_ref)
    }
}
