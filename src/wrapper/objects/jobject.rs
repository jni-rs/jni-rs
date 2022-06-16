use std::marker::PhantomData;

use crate::sys::jobject;

/// Wrapper around `sys::jobject` that adds a lifetime. This prevents it from
/// outliving the context in which it was acquired and getting GC'd out from
/// under us. It matches C's representation of the raw pointer, so it can be
/// used in any of the extern function argument positions that would take a
/// `jobject`.
///
/// Most other types in the `objects` module deref to this, as they do in the C
/// representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct JObject<'a> {
    internal: jobject,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<jobject> for JObject<'a> {
    fn from(other: jobject) -> Self {
        JObject {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> ::std::ops::Deref for JObject<'a> {
    type Target = jobject;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a> JObject<'a> {
    /// Unwrap to the internal jni type.
    pub fn into_inner(self) -> jobject {
        self.internal
    }

    /// Creates a new null object
    pub fn null() -> JObject<'a> {
        (::std::ptr::null_mut() as jobject).into()
    }
}

impl<'a> std::default::Default for JObject<'a> {
    fn default() -> Self {
        Self::null()
    }
}
