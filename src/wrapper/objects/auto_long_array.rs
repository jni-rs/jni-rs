use std::ptr::NonNull;

use log::error;

use crate::objects::release_mode::ReleaseMode;
use crate::sys::{jlong, jsize};
use crate::{errors::*, objects::JObject, sys, JNIEnv};

/// Auto-release wrapper for pointer-based long arrays.
///
/// This wrapper provides automatic array release when it goes out of scope.
pub struct AutoLongArray<'a: 'b, 'b> {
    obj: JObject<'a>,
    ptr: NonNull<jlong>,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'b JNIEnv<'a>,
}

impl<'a, 'b> AutoLongArray<'a, 'b> {
    pub(crate) fn new(
        env: &'b JNIEnv<'a>,
        obj: JObject<'a>,
        ptr: *mut jlong,
        mode: ReleaseMode,
        is_copy: bool,
    ) -> Result<Self> {
        Ok(AutoLongArray {
            obj,
            ptr: NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?,
            mode,
            is_copy,
            env,
        })
    }

    /// Get a reference to the wrapped pointer
    pub fn as_ptr(&self) -> *mut jlong {
        self.ptr.as_ptr()
    }

    /// Commits the result of the array, if it is a copy
    pub fn commit(&mut self) -> Result<()> {
        self.release_long_array_elements(sys::JNI_COMMIT)
    }

    fn release_long_array_elements(&mut self, mode: i32) -> Result<()> {
        jni_void_call!(
            self.env.get_native_interface(),
            ReleaseLongArrayElements,
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

impl<'a, 'b> Drop for AutoLongArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.release_long_array_elements(self.mode as i32);
        match res {
            Ok(()) => {}
            Err(e) => error!("error releasing long array: {:#?}", e),
        }
    }
}

impl<'a> From<&'a AutoLongArray<'a, '_>> for *mut jlong {
    fn from(other: &'a AutoLongArray) -> *mut jlong {
        other.as_ptr()
    }
}
