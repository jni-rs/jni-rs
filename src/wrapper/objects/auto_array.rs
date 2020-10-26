use log::debug;

use crate::objects::ReleaseMode;
use crate::{errors::*, objects::JObject, sys, JNIEnv};
use combine::lib::any::TypeId;
use std::any::type_name;
use std::ptr::NonNull;

/// Auto-release wrapper for pointer-based generic arrays.
///
/// This wrapper is used to wrap pointers returned by Get<Type>ArrayElements.
///
/// These arrays need to be released through a call to Release<Type>ArrayElements.
/// This wrapper provides automatic array release when it goes out of scope.
pub struct AutoArray<'a: 'b, 'b, T: 'static> {
    obj: JObject<'a>,
    ptr: NonNull<T>,
    mode: ReleaseMode,
    is_copy: bool,
    env: &'b JNIEnv<'a>,
    type_id: TypeId,
}

impl<'a, 'b, T: 'static> AutoArray<'a, 'b, T> {
    /// Creates a new auto-release wrapper for a pointer-based generic array
    ///
    /// Once this wrapper goes out of scope, `Release<Type>ArrayElements` will be
    /// called on the object. While wrapped, the object can be accessed via
    /// the `From` impl.
    pub fn new(
        env: &'b JNIEnv<'a>,
        obj: JObject<'a>,
        ptr: *mut T,
        mode: ReleaseMode,
        is_copy: bool,
    ) -> Result<Self> {
        Ok(AutoArray {
            obj,
            ptr: NonNull::new(ptr).ok_or(Error::NullPtr("Non-null ptr expected"))?,
            mode,
            is_copy,
            env,
            type_id: TypeId::of::<T>(),
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
        let env = self.env.get_native_interface();
        let ptr = self.ptr.as_ptr();
        if self.type_id == TypeId::of::<i32>() {
            jni_void_call!(
                env,
                ReleaseIntArrayElements,
                *self.obj,
                ptr as *mut i32,
                mode
            );
        } else if self.type_id == TypeId::of::<i64>() {
            jni_void_call!(
                env,
                ReleaseLongArrayElements,
                *self.obj,
                ptr as *mut i64,
                mode
            );
        } else if self.type_id == TypeId::of::<i8>() {
            jni_void_call!(
                env,
                ReleaseByteArrayElements,
                *self.obj,
                ptr as *mut i8,
                mode
            );
        } else if self.type_id == TypeId::of::<u8>() {
            jni_void_call!(
                env,
                ReleaseBooleanArrayElements,
                *self.obj,
                ptr as *mut u8,
                mode
            );
        } else if self.type_id == TypeId::of::<u16>() {
            jni_void_call!(
                env,
                ReleaseCharArrayElements,
                *self.obj,
                ptr as *mut u16,
                mode
            );
        } else if self.type_id == TypeId::of::<i16>() {
            jni_void_call!(
                env,
                ReleaseShortArrayElements,
                *self.obj,
                ptr as *mut i16,
                mode
            );
        } else if self.type_id == TypeId::of::<f32>() {
            jni_void_call!(
                env,
                ReleaseFloatArrayElements,
                *self.obj,
                ptr as *mut f32,
                mode
            );
        } else if self.type_id == TypeId::of::<f64>() {
            jni_void_call!(
                env,
                ReleaseDoubleArrayElements,
                *self.obj,
                ptr as *mut f64,
                mode
            );
        } else {
            return Err(Error::WrongJValueType(type_name::<T>(), "?"));
        }
        Ok(())
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

impl<'a, 'b, T: 'static> Drop for AutoArray<'a, 'b, T> {
    fn drop(&mut self) {
        let res = self.release_array_elements(self.mode as i32);
        match res {
            Ok(()) => {}
            Err(e) => debug!("error releasing array: {:#?}", e),
        }
    }
}

impl<'a, T> From<&'a AutoArray<'a, '_, T>> for *mut T {
    fn from(other: &'a AutoArray<T>) -> *mut T {
        other.as_ptr()
    }
}
