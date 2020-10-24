use crate::sys::JNI_ABORT;

/// ReleaseMode
///
/// This defines the release mode of Auto*Array (and AutoPrimitiveArray) resources, and
/// related release array functions.
#[derive(Clone, Copy)]
#[repr(i32)]
pub enum ReleaseMode {
    /// Copy back the content and free the elems buffer.
    CopyBack = 0,
    /// Free the buffer without copying back the possible changes.
    NoCopyBack = JNI_ABORT,
}