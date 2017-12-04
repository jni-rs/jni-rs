use std::marker::PhantomData;

use sys::jfieldID;

/// Wrapper around `sys::jstaticfieldid` that adds a lifetime. This prevents it
/// from outliving the context in which it was acquired and getting GC'd out
/// from under us. It matches C's representation of the raw pointer, so it can
/// be used in any of the extern function argument positions that would take a
/// `jstaticfieldid`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct JStaticFieldID<'a> {
    internal: jfieldID,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<jfieldID> for JStaticFieldID<'a> {
    fn from(other: jfieldID) -> Self {
        JStaticFieldID {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> JStaticFieldID<'a> {
    /// Unwrap to the internal jni type.
    pub fn into_inner(self) -> jfieldID {
        self.internal
    }
}
