use std::str;

use std::marker::PhantomData;

use errors::*;

use sys::{self, JavaVMAttachArgs};
use std::os::raw::c_void;
use std::ptr::null_mut;


use wrapper::JNIEnv;

/// JavaVM
#[repr(C)]
#[derive(Debug, Copy, Clone)]
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
    /// get JNIEnv from JavaVM
    pub fn get_env(&self) -> Result<JNIEnv> {
        let mut ptr: *mut c_void = null_mut();
        let pptr = &mut ptr as *mut *mut c_void;
        unsafe {
            let status = jni_unchecked!(self.internal,
                                        GetEnv,
                                        pptr,
                                        sys::JNI_VERSION_1_6);
            if status != sys::JNI_OK {
                return Err(ErrorKind::JavaException.into());
            }
        }
        Ok(JNIEnv::from(ptr as *mut sys::JNIEnv))
    }

    /// call AttachCurrentThread to get JNIEnv
    pub fn attach_current_thread(&self) -> Result<JNIEnv<'static>> {
        let mut ptr: *mut c_void = null_mut();
        let pptr = &mut ptr as *mut *mut c_void;
        use std::ffi::CString;
        let mut args = JavaVMAttachArgs {
            version: sys::JNI_VERSION_1_6,
            group: null_mut(),
            name: CString::new("default").unwrap().into_raw(),
        };
        let args_ptr = &mut args;

        unsafe {
            let status = jni_unchecked!(self.internal,
                                        AttachCurrentThread,
                                        pptr,
                                        args_ptr as *mut JavaVMAttachArgs as
                                        *mut c_void);
            if status != sys::JNI_OK {
                return Err(ErrorKind::JavaException.into());
            }
        }
        Ok(JNIEnv::from(ptr as *mut sys::JNIEnv))
    }
}
