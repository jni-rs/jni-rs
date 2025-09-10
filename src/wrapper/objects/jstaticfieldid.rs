use crate::sys::jfieldID;

/// Wrapper around [`jfieldID`] that implements `Send` + `Sync` since field IDs
/// are valid across threads (not tied to a `Env`).
///
/// There is no lifetime associated with these since they aren't garbage
/// collected like objects and their lifetime is not implicitly connected with
/// the scope in which they are queried.
///
/// It matches C's representation of the raw pointer, so it can be used in any
/// of the extern function argument positions that would take a [`jfieldID`].
///
/// # Safety
///
/// According to the JNI spec field IDs may be invalidated when the
/// corresponding class is unloaded.
///
/// Since this constraint can't be encoded as a Rust lifetime, and to avoid the
/// excessive cost of having every Method ID be associated with a global
/// reference to the corresponding class then it is the developers
/// responsibility to ensure they hold some class reference for the lifetime of
/// cached method IDs.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct JStaticFieldID {
    internal: jfieldID,
}

// Static Field IDs are valid across threads (not tied to a Env)
unsafe impl Send for JStaticFieldID {}
unsafe impl Sync for JStaticFieldID {}

impl JStaticFieldID {
    /// Creates a [`JStaticFieldID`] that wraps the given `raw` [`jfieldID`]
    ///
    /// # Safety
    ///
    /// Expects a valid, non-`null` ID
    pub const unsafe fn from_raw(raw: jfieldID) -> Self {
        Self { internal: raw }
    }

    /// Unwrap to the internal jni type.
    pub const fn into_raw(self) -> jfieldID {
        self.internal
    }
}

impl AsRef<JStaticFieldID> for JStaticFieldID {
    fn as_ref(&self) -> &JStaticFieldID {
        self
    }
}

impl AsMut<JStaticFieldID> for JStaticFieldID {
    fn as_mut(&mut self) -> &mut JStaticFieldID {
        self
    }
}
