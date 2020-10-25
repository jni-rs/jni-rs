use crate::sys::{jbyte, JNI_ABORT};
use log::debug;

use crate::{errors::*, objects::JObject, JNIEnv};
use std::ptr::NonNull;

/// ReleaseMode
///
/// This defines the release mode of AutoByteArray (and AutoPrimitiveArray) resources, and
/// related release array functions.
#[derive(Clone, Copy)]
#[repr(i32)]
pub enum ReleaseMode {
    /// Copy back the content and free the elems buffer.
    CopyBack = 0,
    /// Free the buffer without copying back the possible changes.
    NoCopyBack = JNI_ABORT,
}

/// Auto-release wrapper for pointer-based byte arrays.
///
/// This wrapper is used to wrap pointers returned by get_byte_array_elements.
///
/// These arrays need to be released through a call to release_byte_array_elements.
/// This wrapper provides automatic array release when it goes out of scope.
pub struct AutoByteArray<'a: 'b, 'b> {
    obj: JObject<'a>,
    ptr: NonNull<jbyte>,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'b JNIEnv<'a>,
}

impl<'a, 'b> AutoByteArray<'a, 'b> {
    /// Creates a new auto-release wrapper for a pointer-based byte array
    ///
    /// Once this wrapper goes out of scope, `release_byte_array_elements` will be
    /// called on the object. While wrapped, the object can be accessed via
    /// the `From` impl.
    pub fn new(
        env: &'b JNIEnv<'a>,
        obj: JObject<'a>,
        ptr: *mut jbyte,
        mode: ReleaseMode,
        is_copy: bool,
    ) -> Result<Self> {
        Ok(AutoByteArray {
            obj,
            ptr: NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?,
            mode,
            is_copy,
            env,
        })
    }

    /// Get a reference to the wrapped pointer
    pub fn as_ptr(&self) -> *mut jbyte {
        self.ptr.as_ptr()
    }

    /// Commits the result of the array, if it is a copy
    pub fn commit(&mut self) {
        let res = self
            .env
            .commit_byte_array_elements(*self.obj, unsafe { self.ptr.as_mut() });
        match res {
            Ok(()) => {}
            Err(e) => debug!("error committing byte array: {:#?}", e),
        }
    }

    /// Indicates if the array is a copy or not
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }
}

impl<'a, 'b> Drop for AutoByteArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.env.release_byte_array_elements(
            *self.obj,
            unsafe { self.ptr.as_mut() },
            self.mode,
        );
        match res {
            Ok(()) => {}
            Err(e) => debug!("error releasing byte array: {:#?}", e),
        }
    }
}

impl<'a> From<&'a AutoByteArray<'a, '_>> for *mut jbyte {
    fn from(other: &'a AutoByteArray) -> *mut jbyte {
        other.as_ptr()
    }
}
