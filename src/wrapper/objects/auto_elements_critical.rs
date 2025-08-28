use log::error;
use std::ptr::NonNull;

use crate::sys::jboolean;
use crate::wrapper::objects::ReleaseMode;
use crate::{env::JNIEnv, errors::*, sys};

use super::{JPrimitiveArray, TypeArray};

#[cfg(doc)]
use super::JByteArray;

/// Auto-release wrapper for a mutable pointer to the elements of a [`JPrimitiveArray`]
/// (such as [`JByteArray`])
///
/// This type is used to wrap pointers returned by `GetPrimitiveArrayCritical`
/// and ensure the pointer is released via `ReleasePrimitiveArrayCritical` when dropped.
pub struct AutoElementsCritical<'local, 'other_local, 'array, 'env, T: TypeArray> {
    array: &'array JPrimitiveArray<'other_local, T>,
    len: usize,
    ptr: NonNull<T>,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'env mut JNIEnv<'local>,
}

impl<'local, 'other_local, 'array, 'env, T: TypeArray>
    AutoElementsCritical<'local, 'other_local, 'array, 'env, T>
{
    /// # Safety
    ///
    /// `len` must be the correct length (number of elements) of the given `array`
    pub(crate) unsafe fn new_with_len(
        env: &'env mut JNIEnv<'local>,
        array: &'array JPrimitiveArray<'other_local, T>,
        len: usize,
        mode: ReleaseMode,
    ) -> Result<Self> {
        let mut is_copy: jboolean = true;
        // There are no documented exceptions for GetPrimitiveArrayCritical() but
        // it may return `NULL`.
        let ptr = jni_call_only_check_null_ret!(
            env,
            v1_2,
            GetPrimitiveArrayCritical,
            array.as_raw(),
            &mut is_copy
        )? as *mut T;

        Ok(AutoElementsCritical {
            array,
            len,
            ptr: NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?,
            mode,
            is_copy: is_copy == sys::JNI_TRUE,
            env,
        })
    }

    pub(crate) fn new(
        env: &'env mut JNIEnv<'local>,
        array: &'array JPrimitiveArray<'other_local, T>,
        mode: ReleaseMode,
    ) -> Result<Self> {
        let len = env.get_array_length(array)? as usize;
        unsafe { Self::new_with_len(env, array, len, mode) }
    }

    /// Get a reference to the wrapped pointer
    pub const fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Calls `ReleasePrimitiveArrayCritical`.
    ///
    /// # Safety
    ///
    /// `mode` must be a valid parameter to the JNI `ReleasePrimitiveArrayCritical` `mode`
    /// parameter.
    ///
    /// If `mode` is not [`sys::JNI_COMMIT`], then `self.ptr` must not have already been released.
    unsafe fn release_primitive_array_critical(&mut self, mode: i32) -> Result<()> {
        jni_call_unchecked!(
            self.env,
            v1_2,
            ReleasePrimitiveArrayCritical,
            self.array.as_raw(),
            self.ptr.as_ptr().cast(),
            mode
        );
        Ok(())
    }

    /// Don't copy back the changes to the array on release (if it is a copy).
    ///
    /// This has no effect if the array is not a copy.
    ///
    /// This method is useful to change the release mode of an array originally created
    /// with `ReleaseMode::CopyBack`.
    pub fn discard(&mut self) {
        self.mode = ReleaseMode::NoCopyBack;
    }

    /// Indicates if the array is a copy or not
    pub fn is_copy(&self) -> bool {
        self.is_copy
    }

    /// Returns the array length (number of elements)
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<'local, 'other_local, 'array, 'env, T: TypeArray>
    AsRef<AutoElementsCritical<'local, 'other_local, 'array, 'env, T>>
    for AutoElementsCritical<'local, 'other_local, 'array, 'env, T>
{
    fn as_ref(&self) -> &AutoElementsCritical<'local, 'other_local, 'array, 'env, T> {
        self
    }
}

impl<T: TypeArray> Drop for AutoElementsCritical<'_, '_, '_, '_, T> {
    fn drop(&mut self) {
        // Safety: `self.mode` is valid and the array has not yet been released.
        let res = unsafe { self.release_primitive_array_critical(self.mode as i32) };

        match res {
            Ok(()) => {}
            Err(e) => error!("error releasing primitive array: {:#?}", e),
        }
    }
}

impl<T: TypeArray> From<&AutoElementsCritical<'_, '_, '_, '_, T>> for *mut T {
    fn from(other: &AutoElementsCritical<T>) -> *mut T {
        other.as_ptr()
    }
}

impl<T: TypeArray> std::ops::Deref for AutoElementsCritical<'_, '_, '_, '_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T: TypeArray> std::ops::DerefMut for AutoElementsCritical<'_, '_, '_, '_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_mut(), self.len) }
    }
}
