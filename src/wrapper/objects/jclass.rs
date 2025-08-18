use crate::{
    objects::{JObject, JObjectRef},
    sys::{jclass, jobject},
};

/// Lifetime'd representation of a `jclass`. Just a `JObject` wrapped in a new
/// class.
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JClass<'local>(JObject<'local>);

impl<'local> AsRef<JClass<'local>> for JClass<'local> {
    fn as_ref(&self) -> &JClass<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JClass<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JClass<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JClass<'local>> for JObject<'local> {
    fn from(other: JClass) -> JObject {
        other.0
    }
}

/// This conversion assumes that the `JObject` is a pointer to a class object.
impl<'local> From<JObject<'local>> for JClass<'local> {
    fn from(other: JObject) -> Self {
        unsafe { Self::from_raw(other.into_raw()) }
    }
}

/// This conversion assumes that the `JObject` is a pointer to a class object.
impl<'local, 'obj_ref> From<&'obj_ref JObject<'local>> for &'obj_ref JClass<'local> {
    fn from(other: &'obj_ref JObject<'local>) -> Self {
        // Safety: `JClass` is `repr(transparent)` around `JObject`.
        unsafe { &*(other as *const JObject<'local> as *const JClass<'local>) }
    }
}

impl JClass<'_> {
    /// Creates a [`JClass`] that wraps the given `raw` [`jclass`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jclass) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Returns the raw JNI pointer.
    pub const fn as_raw(&self) -> jclass {
        self.0.as_raw() as jclass
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jclass {
        self.0.into_raw() as jclass
    }
}

impl JObjectRef for JClass<'_> {
    type Kind<'env> = JClass<'env>;
    type GlobalKind = JClass<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JClass::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JClass::from_raw(global_ref)
    }
}
