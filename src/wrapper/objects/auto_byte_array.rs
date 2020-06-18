use std::mem;

use jni_sys::{jbyte, JNI_ABORT};
use log::debug;

use crate::{objects::JObject, JNIEnv};

/// ReleaseMode
///
/// This defines the release mode of AutoByteArray (and AutoPrimitiveArray) resources, and
/// related release array functions.
#[derive(Clone, Copy)]
pub enum ReleaseMode {
    /// Copy back the content and free the elems buffer.
    Copy = 0,
    /// Free the buffer without copying back the possible changes.
    NoCopy = JNI_ABORT as isize,
}

/// Auto-release wrapper for pointer-based byte arrays.
///
/// This wrapper is used to wrap pointers returned by get_byte_array_elements.
///
/// These arrays need to be released through a call to release_byte_array_elements.
/// This wrapper provides automatic array release when it goes out of scope.
pub struct AutoByteArray<'a: 'b, 'b> {
    obj: JObject<'a>,
    ptr: *mut jbyte,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'b JNIEnv<'a>,
}

impl<'a, 'b> AutoByteArray<'a, 'b> {
    /// Creates a new auto-release wrapper for a pointer-based byte array
    ///
    /// Once this wrapper goes out of scope, `release_byte_array_elements` will be
    /// called on the object. While wrapped, the object can be accessed via
    /// the `Deref` impl.
    pub fn new(
        env: &'b JNIEnv<'a>,
        obj: JObject<'a>,
        ptr: *mut jbyte,
        mode: ReleaseMode,
        is_copy: bool,
    ) -> Self {
        AutoByteArray {
            obj,
            ptr,
            mode,
            is_copy,
            env,
        }
    }

    /// Forget the wrapper, returning the original pointer.
    ///
    /// This prevents `release_byte_array_elements` from being called when the `AutoArray`
    /// gets dropped. You must remember to release the array manually.
    pub fn forget(self) -> *mut jbyte {
        let ptr = self.ptr;
        mem::forget(self);
        ptr
    }

    /// Get a reference to the wrapped pointer
    ///
    /// Unlike `forget`, this ensures the wrapper from being dropped while the
    /// returned `JObject` is still live.
    pub fn as_ptr<'c>(&self) -> *mut jbyte
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
            .commit_byte_array_elements(*self.obj, unsafe { self.ptr.as_mut() }.unwrap());
        match res {
            Ok(()) => {}
            Err(e) => debug!("error committing byte array: {:#?}", e),
        }
    }

    /// Indicates the array is a copy or not
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }
}

impl<'a, 'b> Drop for AutoByteArray<'a, 'b> {
    fn drop(&mut self) {
        let res = self.env.release_byte_array_elements(
            *self.obj,
            unsafe { self.ptr.as_mut() }.unwrap(),
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
