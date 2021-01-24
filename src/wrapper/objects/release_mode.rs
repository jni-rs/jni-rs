use crate::sys::JNI_ABORT;

/// ReleaseMode
///
/// This defines the release mode of AutoArray (and AutoPrimitiveArray) resources, and
/// related release array functions.
#[derive(Clone, Copy)]
#[repr(i32)]
pub enum ReleaseMode {
    /// Copy back the content and free the elems buffer. For read-only access, prefer
    /// [`NoCopyBack`](ReleaseMode::NoCopyBack).
    CopyBack = 0,
    /// Free the buffer without copying back the possible changes.
    NoCopyBack = JNI_ABORT,
}
