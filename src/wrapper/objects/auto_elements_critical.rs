use log::error;
use std::marker::PhantomData;
use std::ptr::NonNull;

use crate::sys::jboolean;
use crate::wrapper::objects::ReleaseMode;
use crate::JavaVM;
use crate::{env::Env, errors::*, sys};

use super::{JPrimitiveArray, TypeArray};

#[cfg(doc)]
use super::JByteArray;

/// Auto-release wrapper for a mutable pointer to the elements of a [`JPrimitiveArray`]
/// (such as [`JByteArray`])
///
/// This type is used to wrap pointers returned by `GetPrimitiveArrayCritical`
/// and ensure the pointer is released via `ReleasePrimitiveArrayCritical` when dropped.
pub struct AutoElementsCritical<'array_local, T: TypeArray, TArrayRef>
where
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>>,
{
    array: TArrayRef,
    len: usize,
    ptr: NonNull<T>,
    mode: ReleaseMode,
    is_copy: bool,
    _lifetime: PhantomData<&'array_local ()>,
}

impl<'array_local, T: TypeArray, TArrayRef> AutoElementsCritical<'array_local, T, TArrayRef>
where
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>>,
{
    /// # Safety
    ///
    /// `len` must be the correct length (number of elements) of the given `array`
    pub(crate) unsafe fn new_with_len(
        env: &Env<'_>,
        array: TArrayRef,
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
            array.as_ref().as_raw(),
            &mut is_copy
        )? as *mut T;

        Ok(AutoElementsCritical {
            array,
            len,
            ptr: NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?,
            mode,
            is_copy: is_copy == sys::JNI_TRUE,
            _lifetime: PhantomData,
        })
    }

    pub(crate) fn new(env: &Env<'_>, array: TArrayRef, mode: ReleaseMode) -> Result<Self> {
        let len = array.as_ref().len(env)?;
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
        // Panic: Since we can't construct `AutoElementsCritical` without a
        // valid `Env` reference we know we can call `JavaVM::singleton()`
        // without a panic.
        JavaVM::singleton()?.with_env_current_frame(|env| {
            jni_call_unchecked!(
                env,
                v1_2,
                ReleasePrimitiveArrayCritical,
                self.array.as_ref().as_raw(),
                self.ptr.as_ptr().cast(),
                mode
            );
            Ok(())
        })
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

impl<'array_local, T: TypeArray, TArrayRef> AsRef<AutoElementsCritical<'array_local, T, TArrayRef>>
    for AutoElementsCritical<'array_local, T, TArrayRef>
where
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>>,
{
    fn as_ref(&self) -> &AutoElementsCritical<'array_local, T, TArrayRef> {
        self
    }
}

impl<'array_local, T: TypeArray, TArrayRef> Drop
    for AutoElementsCritical<'array_local, T, TArrayRef>
where
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>>,
{
    fn drop(&mut self) {
        // Safety: `self.mode` is valid and the array has not yet been released.
        let res = unsafe { self.release_primitive_array_critical(self.mode as i32) };

        match res {
            Ok(()) => {}
            Err(e) => error!("error releasing primitive array: {:#?}", e),
        }
    }
}

impl<'array_local, T: TypeArray, TArrayRef> From<&AutoElementsCritical<'array_local, T, TArrayRef>>
    for *mut T
where
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>>,
{
    fn from(other: &AutoElementsCritical<'array_local, T, TArrayRef>) -> *mut T {
        other.as_ptr()
    }
}

impl<'array_local, T: TypeArray, TArrayRef> std::ops::Deref
    for AutoElementsCritical<'array_local, T, TArrayRef>
where
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>>,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<'array_local, T: TypeArray, TArrayRef> std::ops::DerefMut
    for AutoElementsCritical<'array_local, T, TArrayRef>
where
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_mut(), self.len) }
    }
}
