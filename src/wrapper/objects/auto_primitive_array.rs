use log::debug;

use crate::sys::jsize;
use crate::wrapper::objects::ReleaseMode;
use crate::{errors::*, objects::JObject, JNIEnv};
use std::os::raw::c_void;
use std::ptr::NonNull;

/// Auto-release wrapper for pointer-based primitive arrays.
///
/// This wrapper is used to wrap pointers returned by GetPrimitiveArrayCritical.
/// While wrapped, the object can be accessed via the `From` impl.
///
/// AutoPrimitiveArray provides automatic array release through a call to
/// ReleasePrimitiveArrayCritical when it goes out of scope.
pub struct AutoPrimitiveArray<'local: 'env, 'env> {
    obj: JObject<'local>,
    ptr: NonNull<c_void>,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'env mut JNIEnv<'local>,
}

impl<'local, 'env> AutoPrimitiveArray<'local, 'env> {
    pub(crate) fn new(
        env: &'env mut JNIEnv<'local>,
        obj: JObject<'local>,
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

    fn release_primitive_array_critical(&mut self, mode: i32) -> Result<()> {
        jni_unchecked!(
            self.env.get_native_interface(),
            ReleasePrimitiveArrayCritical,
            *self.obj,
            self.ptr.as_mut(),
            mode
        );
        Ok(())
    }

    /// Don't copy the changes to the array on release (if it is a copy).
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

impl<'local, 'env> AsRef<AutoPrimitiveArray<'local, 'env>> for AutoPrimitiveArray<'local, 'env> {
    fn as_ref(&self) -> &AutoPrimitiveArray<'local, 'env> {
        self
    }
}

impl<'local, 'env> AsRef<JObject<'local>> for AutoPrimitiveArray<'local, 'env> {
    fn as_ref(&self) -> &JObject<'local> {
        &self.obj
    }
}

impl<'local, 'env> Drop for AutoPrimitiveArray<'local, 'env> {
    fn drop(&mut self) {
        let res = self.release_primitive_array_critical(self.mode as i32);
        match res {
            Ok(()) => {}
            Err(e) => debug!("error releasing primitive array: {:#?}", e),
        }
    }
}

impl<'local> From<&'local AutoPrimitiveArray<'local, '_>> for *mut c_void {
    fn from(other: &'local AutoPrimitiveArray) -> *mut c_void {
        other.as_ptr()
    }
}
