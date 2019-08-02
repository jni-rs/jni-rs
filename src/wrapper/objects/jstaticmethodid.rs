use std::marker::PhantomData;

use crate::sys::jmethodID;

/// Wrapper around `sys::jmethodid` that adds a lifetime. This prevents it from
/// outliving the context in which it was acquired and getting GC'd out from
/// under us. It matches C's representation of the raw pointer, so it can be
/// used in any of the extern function argument positions that would take a
/// `jmethodid`. This represents static methods only since they require a
/// different set of JNI signatures.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct JStaticMethodID<'a> {
    internal: jmethodID,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<jmethodID> for JStaticMethodID<'a> {
    fn from(other: jmethodID) -> Self {
        JStaticMethodID {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> JStaticMethodID<'a> {
    /// Unwrap to the internal jni type.
    pub fn into_inner(self) -> jmethodID {
        self.internal
    }
}
