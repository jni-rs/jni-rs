use log::error;
use std::marker::PhantomData;
use std::ptr::NonNull;

use crate::objects::JObjectRef;
use crate::sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort};
use crate::wrapper::objects::ReleaseMode;
use crate::{env::JNIEnv, errors::*, sys, JavaVM};

use super::JPrimitiveArray;

#[cfg(doc)]
use super::JByteArray;

mod type_array_sealed {
    use crate::sys::{jarray, jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort};
    use crate::{env::JNIEnv, errors::*};
    use std::ptr::NonNull;

    /// Trait to define type array access/release
    ///
    /// # Safety
    ///
    /// The methods of this trait must uphold the invariants described in [`JNIEnv::unsafe_clone`] when
    /// using the provided [`JNIEnv`].
    ///
    /// The `get` method must return a valid pointer to the beginning of the JNI array.
    ///
    /// The `release` method must not invalidate the `ptr` if the `mode` is [`sys::JNI_COMMIT`].
    pub unsafe trait TypeArraySealed: Copy {
        /// getter
        ///
        /// # Safety
        ///
        /// `array` must be a valid pointer to an `Array` object, or `null`
        ///
        /// The caller is responsible for passing the returned pointer to [`release`], along
        /// with the same `env` and `array` reference (which needs to still be valid)
        unsafe fn get(env: &JNIEnv, array: jarray, is_copy: &mut jboolean) -> Result<*mut Self>;

        /// releaser
        ///
        /// # Safety
        ///
        /// `ptr` must have been previously returned by the `get` function.
        ///
        /// If `mode` is not [`sys::JNI_COMMIT`], `ptr` must not be used again after calling this
        /// function.
        unsafe fn release(env: &JNIEnv, array: jarray, ptr: NonNull<Self>, mode: i32)
            -> Result<()>;
    }

    // TypeArray builder
    macro_rules! type_array {
        ( $jni_type:ty, $jni_get:tt, $jni_release:tt ) => {
            /// $jni_type array access/release impl
            unsafe impl TypeArraySealed for $jni_type {
                /// Get Java $jni_type array
                unsafe fn get(
                    env: &JNIEnv,
                    array: jarray,
                    is_copy: &mut jboolean,
                ) -> Result<*mut Self> {
                    // There are no documented exceptions for Get<Primitive>ArrayElements() but
                    // they may return `NULL`.
                    let ptr = jni_call_only_check_null_ret!(env, v1_1, $jni_get, array, is_copy)?;
                    Ok(ptr as _)
                }

                /// Release Java $jni_type array
                unsafe fn release(
                    env: &JNIEnv,
                    array: jarray,
                    ptr: NonNull<Self>,
                    mode: i32,
                ) -> Result<()> {
                    // There are no documented exceptions for Release<Primitive>ArrayElements()
                    jni_call_unchecked!(env, v1_1, $jni_release, array, ptr.as_ptr(), mode as i32);
                    Ok(())
                }
            }
        };
    }

    type_array!(jint, GetIntArrayElements, ReleaseIntArrayElements);
    type_array!(jlong, GetLongArrayElements, ReleaseLongArrayElements);
    type_array!(jbyte, GetByteArrayElements, ReleaseByteArrayElements);
    type_array!(
        jboolean,
        GetBooleanArrayElements,
        ReleaseBooleanArrayElements
    );
    type_array!(jchar, GetCharArrayElements, ReleaseCharArrayElements);
    type_array!(jshort, GetShortArrayElements, ReleaseShortArrayElements);
    type_array!(jfloat, GetFloatArrayElements, ReleaseFloatArrayElements);
    type_array!(jdouble, GetDoubleArrayElements, ReleaseDoubleArrayElements);
}

/// A sealed trait to define type array access/release for primitive JNI types
pub trait TypeArray: type_array_sealed::TypeArraySealed + Send + Sync {}

impl TypeArray for jint {}
impl TypeArray for jlong {}
impl TypeArray for jbyte {}
impl TypeArray for jboolean {}
impl TypeArray for jchar {}
impl TypeArray for jshort {}
impl TypeArray for jfloat {}
impl TypeArray for jdouble {}

/// Auto-release wrapper for a mutable pointer to the elements of a [`JPrimitiveArray`]
/// (such as [`JByteArray`])
///
/// This type is used to wrap pointers returned by `Get<Type>ArrayElements`
/// and ensure the pointer is released via `Release<Type>ArrayElements` when dropped.
///
/// The wrapper is tied to the lifetime of the array reference that becomes
/// owned by the struct (the reference needs to be retained in order to call
/// `Release<Type>ArrayElements` later).
pub struct AutoElements<'array_local, T, TArrayRef>
where
    T: TypeArray + 'array_local,
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + JObjectRef,
{
    array: TArrayRef,
    len: usize,
    ptr: NonNull<T>,
    mode: ReleaseMode,
    is_copy: bool,
    _lifetime: PhantomData<&'array_local ()>,
}

// Note: since we require a JNIEnv reference to construct AutoElements, that
// means we can assume JavaVM::singleton() is initialized later when we need to
// release the array (so we don't need to somehow save a JNIEnv reference).
impl<'array_local, T, TArrayRef> AutoElements<'array_local, T, TArrayRef>
where
    T: TypeArray + 'array_local,
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + JObjectRef,
{
    /// # Safety
    ///
    /// `len` must be the correct length (number of elements) of the given `array`
    unsafe fn new_with_len(
        env: &JNIEnv<'_>,
        array: TArrayRef,
        len: usize,
        mode: ReleaseMode,
    ) -> Result<Self> {
        let mut is_copy: jboolean = true;
        let ptr = unsafe { T::get(env, array.as_ref().as_raw(), &mut is_copy) }?;
        Ok(AutoElements {
            array,
            len,
            ptr: NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?,
            mode,
            is_copy: is_copy == sys::JNI_TRUE,
            _lifetime: PhantomData,
        })
    }

    pub(crate) fn new(env: &JNIEnv<'_>, array: TArrayRef, mode: ReleaseMode) -> Result<Self> {
        let array = null_check!(array, "get_array_elements array argument")?;
        let len = env.get_array_length(array.as_ref())? as usize;
        unsafe { Self::new_with_len(env, array, len, mode) }
    }

    /// Get a reference to the wrapped pointer
    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Commits the changes to the array, if it is a copy
    pub fn commit(&mut self) -> Result<()> {
        unsafe { self.release_array_elements(sys::JNI_COMMIT) }
    }

    /// Calls the release function.
    ///
    /// # Safety
    ///
    /// `mode` must be a valid parameter to the JNI `Release<PrimitiveType>ArrayElements`' `mode`
    /// parameter.
    ///
    /// If `mode` is not [`sys::JNI_COMMIT`], then `self.ptr` must not have already been released.
    unsafe fn release_array_elements(&self, mode: i32) -> Result<()> {
        // Panic: Since we can't construct `AutoElements` without a valid `JNIEnv` reference
        // we know we can call `JavaVM::singleton()` without a panic.
        JavaVM::singleton()?
            .with_env_current_frame(|env| T::release(env, self.array.as_raw(), self.ptr, mode))
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

impl<'array_local, T, TArrayRef> AsRef<AutoElements<'array_local, T, TArrayRef>>
    for AutoElements<'array_local, T, TArrayRef>
where
    T: TypeArray + 'array_local,
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + JObjectRef,
{
    fn as_ref(&self) -> &AutoElements<'array_local, T, TArrayRef> {
        self
    }
}

impl<'array_local, T, TArrayRef> Drop for AutoElements<'array_local, T, TArrayRef>
where
    T: TypeArray,
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + JObjectRef,
{
    fn drop(&mut self) {
        // Safety: `self.mode` is valid and the array has not yet been released.
        let res = unsafe { self.release_array_elements(self.mode as i32) };

        match res {
            Ok(()) => {}
            Err(e) => error!("error releasing array: {:#?}", e),
        }
    }
}

impl<'array_local, T, TArrayRef> From<&AutoElements<'array_local, T, TArrayRef>> for *mut T
where
    T: TypeArray,
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + JObjectRef,
{
    fn from(other: &AutoElements<'array_local, T, TArrayRef>) -> *mut T {
        other.as_ptr()
    }
}

impl<'array_local, T, TArrayRef> std::ops::Deref for AutoElements<'array_local, T, TArrayRef>
where
    T: TypeArray,
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + JObjectRef,
{
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<'array_local, T, TArrayRef> std::ops::DerefMut for AutoElements<'array_local, T, TArrayRef>
where
    T: TypeArray,
    TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + JObjectRef,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_mut(), self.len) }
    }
}
