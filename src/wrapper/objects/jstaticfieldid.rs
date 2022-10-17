use std::marker::PhantomData;

use crate::sys::jfieldID;

/// Wrapper around `sys::jstaticfieldid` that adds a lifetime. This prevents it
/// from outliving the context in which it was acquired and getting GC'd out
/// from under us. It matches C's representation of the raw pointer, so it can
/// be used in any of the extern function argument positions that would take a
/// `jstaticfieldid`.
#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct JStaticFieldID<'a> {
    internal: jfieldID,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> JStaticFieldID<'a> {
    /// Creates a [`JStaticFieldID`] that wraps the given `raw` [`jfieldID`]
    ///
    /// # Safety
    ///
    /// Expects a valid, non-`null` ID
    pub unsafe fn from_raw(raw: jfieldID) -> Self {
        debug_assert!(!raw.is_null(), "from_raw methodID argument");
        Self {
            internal: raw,
            lifetime: PhantomData,
        }
    }

    /// Unwrap to the internal jni type.
    pub fn into_raw(self) -> jfieldID {
        self.internal
    }
}
