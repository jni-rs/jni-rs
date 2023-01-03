use crate::sys::jsize;
use log::error;

use std::ptr::NonNull;

use crate::objects::release_mode::ReleaseMode;
use crate::sys::{jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort};
use crate::{errors::*, objects::JObject, sys, JNIEnv};

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
pub unsafe trait TypeArray {
    /// getter
    fn get(env: &mut JNIEnv, obj: &JObject, is_copy: &mut jboolean) -> Result<*mut Self>;

    /// releaser
    ///
    /// # Safety
    ///
    /// `ptr` must have been previously returned by the `get` function.
    ///
    /// If `mode` is not [`sys::JNI_COMMIT`], `ptr` must not be used again after calling this
    /// function.
    unsafe fn release(env: &mut JNIEnv, obj: &JObject, ptr: NonNull<Self>, mode: i32)
        -> Result<()>;
}

// TypeArray builder
macro_rules! type_array {
    ( $jni_type:ty, $jni_get:tt, $jni_release:tt ) => {
        /// $jni_type array access/release impl
        unsafe impl TypeArray for $jni_type {
            /// Get Java $jni_type array
            fn get(env: &mut JNIEnv, obj: &JObject, is_copy: &mut jboolean) -> Result<*mut Self> {
                let internal = env.get_native_interface();
                // Even though this method may throw OoME, use `jni_unchecked`
                // instead of `jni_non_null_call` to remove (a slight) overhead
                // of exception checking. An error will still be detected as a `null`
                // result inside AutoArray ctor. Also, modern Hotspot in case of lack
                // of memory will return null and won't throw an exception:
                // https://sourcegraph.com/github.com/openjdk/jdk/-/blob/src/hotspot/share/memory/allocation.hpp#L488-489
                let res = jni_unchecked!(internal, $jni_get, obj.as_raw(), is_copy);
                Ok(res)
            }

            /// Release Java $jni_type array
            unsafe fn release(
                env: &mut JNIEnv,
                obj: &JObject,
                ptr: NonNull<Self>,
                mode: i32,
            ) -> Result<()> {
                let internal = env.get_native_interface();
                jni_unchecked!(
                    internal,
                    $jni_release,
                    obj.as_raw(),
                    ptr.as_ptr(),
                    mode as i32
                );
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

/// Auto-release wrapper for pointer-based generic arrays.
///
/// This wrapper is used to wrap pointers returned by Get<Type>ArrayElements.
/// While wrapped, the object can be accessed via the `From` impl.
///
/// AutoArray provides automatic array release through a call to appropriate
/// Release<Type>ArrayElements when it goes out of scope.
pub struct AutoArray<'local, T: TypeArray> {
    obj: JObject<'local>,
    ptr: NonNull<T>,
    mode: ReleaseMode,
    is_copy: bool,
    env: JNIEnv<'local>,
}

impl<'local, T: TypeArray> AutoArray<'local, T> {
    pub(crate) fn new(
        env: &mut JNIEnv<'local>,
        obj: JObject<'local>,
        mode: ReleaseMode,
    ) -> Result<Self> {
        // Safety: The cloned `JNIEnv` will not be used to create any local references. It will be
        // passed to the methods of the `TypeArray` implementation, but that trait is `unsafe` and
        // implementations are required to uphold the invariants of `unsafe_clone`.
        let mut env = unsafe { env.unsafe_clone() };

        let mut is_copy: jboolean = 0xff;
        Ok(AutoArray {
            ptr: {
                let ptr = T::get(&mut env, &obj, &mut is_copy)?;
                NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?
            },
            obj,
            mode,
            is_copy: is_copy == sys::JNI_TRUE,
            env,
        })
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
    /// If `mode` is not [`sys::JNI_COMMIT`], then the array must not have already been released.
    unsafe fn release_array_elements(&mut self, mode: i32) -> Result<()> {
        T::release(&mut self.env, &self.obj, self.ptr, mode)
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

    /// Returns the array size
    pub fn size(&self) -> Result<jsize> {
        self.env.get_array_length(*self.obj)
    }
}

impl<'local, T: TypeArray> AsRef<AutoArray<'local, T>> for AutoArray<'local, T> {
    fn as_ref(&self) -> &AutoArray<'local, T> {
        self
    }
}

impl<'local, T: TypeArray> AsRef<JObject<'local>> for AutoArray<'local, T> {
    fn as_ref(&self) -> &JObject<'local> {
        &self.obj
    }
}

impl<'local, T: TypeArray> Drop for AutoArray<'local, T> {
    fn drop(&mut self) {
        // Safety: `self.mode` is valid and the array has not yet been released.
        let res = unsafe { self.release_array_elements(self.mode as i32) };

        match res {
            Ok(()) => {}
            Err(e) => error!("error releasing array: {:#?}", e),
        }
    }
}

impl<'local, T: TypeArray> From<&'local AutoArray<'local, T>> for *mut T {
    fn from(other: &'local AutoArray<T>) -> *mut T {
        other.as_ptr()
    }
}
