use std::marker::PhantomData;

use crate::sys::jmethodID;

/// Wrapper around `sys::jmethodid` that adds a lifetime. This prevents it from
/// outliving the context in which it was acquired and getting GC'd out from
/// under us. It matches C's representation of the raw pointer, so it can be
/// used in any of the extern function argument positions that would take a
/// `jmethodid`.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct JMethodID<'a> {
    internal: jmethodID,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<jmethodID> for JMethodID<'a> {
    fn from(other: jmethodID) -> Self {
        JMethodID {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> JMethodID<'a> {
    /// Unwrap to the internal jni type.
    pub fn into_inner(self) -> jmethodID {
        self.internal
    }
}
