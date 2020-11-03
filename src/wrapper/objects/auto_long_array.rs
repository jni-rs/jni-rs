use std::ptr::NonNull;

use log::error;

use crate::objects::release_mode::ReleaseMode;
use crate::sys::jlong;
use crate::{errors::*, objects::JObject, JNIEnv};

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
            ptr: NonNull::new(ptr).ok_or_else(|| Error::NullPtr("Non-null ptr expected"))?,
            mode,
            is_copy,
            env,
        })
    }

    /// Get a reference to the wrapped pointer
    pub fn as_ptr(&self) -> *mut jlong {
        self.ptr.as_ptr()
    }

    /// Get the release mode
    /// See [`ReleaseMode`](objects/enum.ReleaseMode.html) for details.
    pub fn get_release_mode(&self) -> ReleaseMode {
        self.mode
    }

    /// Set/Change the release mode
    /// See [`ReleaseMode`](objects/enum.ReleaseMode.html) for details.
    pub fn set_release_mode(&mut self, mode: ReleaseMode) {
        self.mode = mode;
    }

    /// Indicates if the array is a copy or not
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }

    fn release_long_array_elements(&self) -> Result<()> {
        jni_void_call!(
            self.env.get_native_interface(),
            ReleaseLongArrayElements,
            *self.obj,
            self.ptr.as_ptr(),
            self.mode as i32
        );
        Ok(())
    }
}

impl<'a, 'b> Drop for AutoLongArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.release_long_array_elements();
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
