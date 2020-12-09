use crate::sys::jsize;
use log::debug;

use crate::objects::release_mode::ReleaseMode;
use crate::sys::{jbyte, jlong};
use crate::{errors::*, objects::JObject, sys, JNIEnv};
use jni_sys::jboolean;
use std::ptr::NonNull;

/// Trait to define type array access/release
pub trait TypeArray {
    /// getter
    fn get(env: *mut sys::JNIEnv, obj: JObject, is_copy: &mut jboolean) -> Result<*mut Self>;

    /// releaser
    fn release(env: *mut sys::JNIEnv, obj: JObject, ptr: *mut Self, mode: i32) -> Result<()>;
}

/// jbyte array access/release impl
impl TypeArray for jbyte {
    /// get Java byte array
    fn get(env: *mut sys::JNIEnv, obj: JObject, is_copy: &mut jboolean) -> Result<*mut Self> {
        let res = jni_non_void_call!(env, GetByteArrayElements, *obj, is_copy);
        Ok(res)
    }

    /// release Java byte array
    fn release(env: *mut sys::JNIEnv, obj: JObject, ptr: *mut Self, mode: i32) -> Result<()> {
        jni_void_call!(env, ReleaseByteArrayElements, *obj, ptr, mode);
        Ok(())
    }
}

/// jlong array access/release impl
impl TypeArray for jlong {
    /// get Java long array
    fn get(env: *mut sys::JNIEnv, obj: JObject, is_copy: &mut jboolean) -> Result<*mut Self> {
        let res = jni_non_void_call!(env, GetLongArrayElements, *obj, is_copy);
        Ok(res)
    }

    /// release Java long array
    fn release(env: *mut sys::JNIEnv, obj: JObject, ptr: *mut Self, mode: i32) -> Result<()> {
        jni_void_call!(env, ReleaseLongArrayElements, *obj, ptr, mode as i32);
        Ok(())
    }
}

/// Auto-release wrapper for pointer-based generic arrays.
///
/// This wrapper is used to wrap pointers returned by Get<Type>ArrayElements.
///
/// These arrays need to be released through a call to Release<Type>ArrayElements.
/// This wrapper provides automatic array release when it goes out of scope.
pub struct AutoArray<'a: 'b, 'b, T: TypeArray> {
    obj: JObject<'a>,
    ptr: NonNull<T>,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'b JNIEnv<'a>,
}

impl<'a, 'b, T: TypeArray> AutoArray<'a, 'b, T> {
    /// Creates a new auto-release wrapper for a pointer-based generic array
    ///
    /// Once this wrapper goes out of scope, `Release<Type>ArrayElements` will be
    /// called on the object. While wrapped, the object can be accessed via
    /// the `From` impl.
    pub fn new(env: &'b JNIEnv<'a>, obj: JObject<'a>, mode: ReleaseMode) -> Result<Self> {
        let mut is_copy: jboolean = 0xff;
        Ok(AutoArray {
            obj,
            ptr: {
                let internal = env.get_native_interface();
                let ptr = T::get(internal, obj, &mut is_copy)?;
                NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?
            },
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
        self.release_array_elements(sys::JNI_COMMIT)
    }

    fn release_array_elements(&mut self, mode: i32) -> Result<()> {
        let internal = self.env.get_native_interface();
        let ptr = self.ptr.as_ptr();
        T::release(internal, self.obj, ptr, mode)
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

impl<'a, 'b, T: TypeArray> Drop for AutoArray<'a, 'b, T> {
    fn drop(&mut self) {
        let res = self.release_array_elements(self.mode as i32);
        match res {
            Ok(()) => {}
            Err(e) => debug!("error releasing array: {:#?}", e),
        }
    }
}

impl<'a, T: TypeArray> From<&'a AutoArray<'a, '_, T>> for *mut T {
    fn from(other: &'a AutoArray<T>) -> *mut T {
        other.as_ptr()
    }
}
