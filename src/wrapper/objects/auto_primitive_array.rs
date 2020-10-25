use log::debug;

use crate::wrapper::objects::ReleaseMode;
use crate::{errors::*, objects::JObject, JNIEnv};
use std::os::raw::c_void;
use std::ptr::NonNull;

/// Auto-release wrapper for pointer-based primitive arrays.
///
/// This wrapper is used to wrap pointers returned by get_primitive_array_critical.
///
/// These pointers normally need to be released manually, through a call to
/// release_primitive_array_critical.
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
    /// Once this wrapper goes out of scope, `release_primitive_array_critical` will be
    /// called on the object. While wrapped, the object can be accessed via the `From` impl.
    pub fn new(
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

    /// Commits the result of the array, if it is a copy
    pub fn commit(&mut self) {
        let res = self
            .env
            .commit_primitive_array_critical(*self.obj, unsafe { self.ptr.as_mut() });
        match res {
            Ok(()) => {}
            Err(e) => debug!("error committing primitive array: {:#?}", e),
        }
    }

    /// Indicates if the array is a copy or not
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }
}

impl<'a, 'b> Drop for AutoPrimitiveArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.env.release_primitive_array_critical(
            *self.obj,
            unsafe { self.ptr.as_mut() },
            self.mode,
        );
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
