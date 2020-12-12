use log::debug;

use crate::sys::jsize;
use crate::wrapper::objects::ReleaseMode;
use crate::{errors::*, objects::JObject, sys, JNIEnv};
use std::os::raw::c_void;
use std::ptr::NonNull;

/// Auto-release wrapper for pointer-based primitive arrays.
///
/// This wrapper is used to wrap pointers returned by GetPrimitiveArrayCritical.
///
/// These pointers normally need to be released manually, through a call to
/// ReleasePrimitiveArrayCritical.
/// This wrapper provides automatic pointer-based array release when it goes out of scope.
pub struct AutoPrimitiveArray<'a: 'b, 'b> {
    obj: JObject<'a>,
    ptr: NonNull<c_void>,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'b JNIEnv<'a>,
}

impl<'a, 'b> AutoPrimitiveArray<'a, 'b> {
    /// Creates a new auto-release wrapper for a pointer-based primitive array.
    ///
    /// Once this wrapper goes out of scope, `ReleasePrimitiveArrayCritical` will be
    /// called on the object. While wrapped, the object can be accessed via the `From` impl.
    pub(crate) fn new(
        env: &'b JNIEnv<'a>,
        obj: JObject<'a>,
        ptr: *mut c_void,
        mode: ReleaseMode,
        is_copy: bool,
    ) -> Result<Self> {
        Ok(AutoPrimitiveArray {
            obj,
            ptr: NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?,
            mode,
            is_copy,
            env,
        })
    }

    /// Get a reference to the wrapped pointer
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }

    /// Commits the changes to the array, if it is a copy
    pub fn commit(&mut self) -> Result<()> {
        self.release_primitive_array_critical(sys::JNI_COMMIT)
    }

    fn release_primitive_array_critical(&mut self, mode: i32) -> Result<()> {
        jni_void_call!(
            self.env.get_native_interface(),
            ReleasePrimitiveArrayCritical,
            *self.obj,
            self.ptr.as_mut(),
            mode
        );
        Ok(())
    }

    /// Don't commit the changes to the array on release (if it is a copy).
    /// This has no effect if the array is not a copy.
    /// This method is useful to change the release mode of an array originally created
    /// with `ReleaseMode::CopyBack`.
    pub fn discard(&mut self) {
        self.mode = ReleaseMode::NoCopyBack;
    }

    /// Indicates if the array is a copy or not
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }

    /// Returns the array size
    pub fn size(&self) -> Result<jsize> {
        self.env.get_array_length(*self.obj)
    }
}

impl<'a, 'b> Drop for AutoPrimitiveArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.release_primitive_array_critical(self.mode as i32);
        match res {
            Ok(()) => {}
            Err(e) => debug!("error releasing primitive array: {:#?}", e),
        }
    }
}

impl<'a> From<&'a AutoPrimitiveArray<'a, '_>> for *mut c_void {
    fn from(other: &'a AutoPrimitiveArray) -> *mut c_void {
        other.as_ptr()
    }
}
