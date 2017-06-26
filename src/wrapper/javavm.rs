use std::str;

use std::marker::PhantomData;

use errors::*;

use sys::{self};
use std::os::raw::c_void;
use std::ptr::null_mut;


use wrapper::JNIEnv;

/// TODO: Need docs
#[repr(C)]
pub struct JavaVM<'a> {
    internal: *mut sys::JavaVM,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<*mut sys::JavaVM> for JavaVM<'a> {
    fn from(other: *mut sys::JavaVM) -> Self {
        JavaVM {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> JavaVM<'a> {
    /// TODO: Need docs
    pub fn get_env(&self) -> Result<JNIEnv> {
        let mut ptr: *mut c_void = null_mut();
        let pptr = &mut ptr as *mut *mut c_void;
        unsafe {
            let status = jni_unchecked!(self.internal, GetEnv, pptr, sys::JNI_VERSION_1_6);
            if status != sys::JNI_OK {
                return Err(ErrorKind::JavaException.into());
            }
        }
        Ok(JNIEnv::from(ptr as *mut sys::JNIEnv))
    }
}
