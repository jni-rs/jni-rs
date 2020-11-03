use crate::sys::{JNI_ABORT, JNI_COMMIT};

/// ReleaseMode
///
/// This defines the release mode of Auto*Array (and AutoPrimitiveArray) resources.
#[derive(Clone, Copy)]
#[repr(i32)]
pub enum ReleaseMode {
    /// Copy back the content and free the native buffer.
    CopyBack = 0,
    /// Copy back the content and don't free the native buffer.
    CopyBackNoFree = JNI_COMMIT,
    /// Free the native buffer without copying back the possible changes.
    NoCopyBack = JNI_ABORT,
}
