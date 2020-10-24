use crate::sys::{jbyte, JNI_ABORT};
use log::debug;

use crate::{errors::*, objects::JObject, sys, JNIEnv};
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

    fn commit_byte_array_elements(&mut self) -> Result<()> {
        jni_void_call!(
            self.env.get_native_interface(),
            ReleaseByteArrayElements,
            *self.obj,
            self.ptr.as_mut(),
            sys::JNI_COMMIT
        );
        Ok(())
    }

    fn release_byte_array_elements(&mut self) -> Result<()> {
        jni_void_call!(
            self.env.get_native_interface(),
            ReleaseByteArrayElements,
            *self.obj,
            self.ptr.as_mut(),
            self.mode as i32
        );
        Ok(())
    }

    /// Commits the changes to the array, if it is a copy
    pub fn commit(&mut self) {
        let res = self.commit_byte_array_elements();
        match res {
            Ok(()) => {}
            Err(e) => debug!("error committing byte array: {:#?}", e),
        }
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
}

impl<'a, 'b> Drop for AutoByteArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.release_byte_array_elements();
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
