use std::ptr::NonNull;

use log::debug;

use crate::{errors::*, JNIEnv, objects::JObject};
use crate::objects::release_mode::ReleaseMode;
use crate::sys::jlong;

/// Auto-release wrapper for pointer-based long arrays.
///
/// This wrapper is used to wrap pointers returned by get_long_array_elements.
///
/// These arrays need to be released through a call to release_long_array_elements.
/// This wrapper provides automatic array release when it goes out of scope.
pub struct AutoLongArray<'a: 'b, 'b> {
    obj: JObject<'a>,
    ptr: NonNull<jlong>,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'b JNIEnv<'a>,
}

impl<'a, 'b> AutoLongArray<'a, 'b> {
    /// Creates a new auto-release wrapper for a pointer-based long array
    ///
    /// Once this wrapper goes out of scope, `release_long_array_elements` will be
    /// called on the object. While wrapped, the object can be accessed via
    /// the `From` impl.
    pub fn new(
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

    /// Commits the result of the array, if it is a copy
    pub fn commit(&mut self) {
        let res = self
            .env
            .commit_long_array_elements(*self.obj, unsafe { self.ptr.as_mut() });
        match res {
            Ok(()) => {}
            Err(e) => debug!("error committing long array: {:#?}", e),
        }
    }

    /// Indicates if the array is a copy or not
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }
}

impl<'a, 'b> Drop for AutoLongArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.env.release_long_array_elements(
            *self.obj,
            unsafe { self.ptr.as_mut() },
            self.mode,
        );
        match res {
            Ok(()) => {}
            Err(e) => debug!("error releasing long array: {:#?}", e),
        }
    }
}

impl<'a> From<&'a AutoLongArray<'a, '_>> for *mut jlong {
    fn from(other: &'a AutoLongArray) -> *mut jlong {
        other.as_ptr()
    }
}
