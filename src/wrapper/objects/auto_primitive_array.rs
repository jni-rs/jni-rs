use std::mem;

use log::debug;

use crate::wrapper::objects::ReleaseMode;
use crate::{objects::JObject, JNIEnv};
use std::os::raw::c_void;

/// Auto-release wrapper for pointer-based primitive arrays.
///
/// This wrapper is used to wrap pointers returned by get_primitive_array_critical.
///
/// These pointers normally need to be released manually, through a call to
/// release_primitive_array_critical.
/// This wrapper provides automatic pointer-based array release when it goes out of scope.
pub struct AutoPrimitiveArray<'a: 'b, 'b> {
    obj: JObject<'a>,
    ptr: *mut c_void,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'b JNIEnv<'a>,
}

impl<'a, 'b> AutoPrimitiveArray<'a, 'b> {
    /// Creates a new auto-release wrapper for a pointer-based primitive array.
    ///
    /// Once this wrapper goes out of scope, `release_primitive_array_critical` will be
    /// called on the object. While wrapped, the object can be accessed via the `Deref` impl.
    pub fn new(
        env: &'b JNIEnv<'a>,
        obj: JObject<'a>,
        ptr: *mut c_void,
        mode: ReleaseMode,
        is_copy: bool,
    ) -> Self {
        AutoPrimitiveArray {
            obj,
            ptr,
            mode,
            is_copy,
            env,
        }
    }

    /// Forget the wrapper, returning the original pointer.
    ///
    /// This prevents `release_primitive_array_critical` from being called when the
    /// `AutoArrayCritical` gets dropped. You must remember to release the primitive array
    /// manually.
    pub fn forget(self) -> *mut c_void {
        let ptr = self.ptr;
        mem::forget(self);
        ptr
    }

    /// Get a reference to the wrapped pointer
    ///
    /// Unlike `forget`, this ensures the wrapper from being dropped while the
    /// returned `JObject` is still live.
    pub fn as_ptr<'c>(&self) -> *mut c_void
    where
        'a: 'c,
    {
        self.ptr
    }

    /// Commits the result of the array, if it is a copy
    pub fn commit(&self) {
        if !self.is_copy {
            return;
        }
        let res = self
            .env
            .commit_primitive_array_critical(*self.obj, unsafe { self.ptr.as_mut() }.unwrap());
        match res {
            Ok(()) => {}
            Err(e) => debug!("error committing primitive array: {:#?}", e),
        }
    }

    /// Indicates the array is a copy or not
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }
}

impl<'a, 'b> Drop for AutoPrimitiveArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.env.release_primitive_array_critical(
            *self.obj,
            unsafe { self.ptr.as_mut() }.unwrap(),
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
