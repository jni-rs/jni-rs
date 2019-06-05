use errors::*;
use JNIEnv;

use sys;

use std::{ops::Deref, ptr};

#[cfg(feature = "invocation")]
use InitArgs;

/// The invocation API.
pub struct JavaVM(*mut sys::JavaVM);

unsafe impl Send for JavaVM {}
unsafe impl Sync for JavaVM {}

impl JavaVM {
    /// Launch a new JavaVM using the provided init args
    #[cfg(feature = "invocation")]
    pub fn new(args: InitArgs) -> Result<Self> {
        use std::os::raw::c_void;

        let mut ptr: *mut sys::JavaVM = ::std::ptr::null_mut();
        let mut env: *mut sys::JNIEnv = ::std::ptr::null_mut();

        unsafe {
            jni_error_code_to_result(sys::JNI_CreateJavaVM(
                &mut ptr as *mut _,
                &mut env as *mut *mut sys::JNIEnv as *mut *mut c_void,
                args.inner_ptr(),
            ))?;

            let vm = Self::from_raw(ptr)?;
            java_vm_unchecked!(vm.0, DetachCurrentThread);

            Ok(vm)
        }
    }

    /// Create a JavaVM from a raw pointer.
    pub unsafe fn from_raw(ptr: *mut sys::JavaVM) -> Result<Self> {
        non_null!(ptr, "from_raw ptr argument");
        Ok(JavaVM(ptr))
    }

    /// Returns underlying `sys::JavaVM` interface.
    pub fn get_java_vm_pointer(&self) -> *mut sys::JavaVM {
        self.0
    }

    /// Attaches the current thread to a Java VM. The resulting `AttachGuard`
    /// can be dereferenced to a `JNIEnv` and automatically detaches the thread
    /// when dropped. Calling this for a thread that is already attached is a no-op.
    pub fn attach_current_thread(&self) -> Result<AttachGuard> {
        let (env, requires_detach) = match self.get_env() {
            Ok(env) => (env, false),
            Err(_) => unsafe {
                let mut ptr = ptr::null_mut();
                let res =
                    java_vm_unchecked!(self.0, AttachCurrentThread, &mut ptr, ptr::null_mut());
                jni_error_code_to_result(res)?;

                let env = JNIEnv::from_raw(ptr as *mut sys::JNIEnv)?;
                (env, true)
            },
        };

        Ok(AttachGuard {
            java_vm: self,
            env,
            requires_detach,
        })
    }

    /// Attaches the current thread to a Java VM as a daemon.
    pub fn attach_current_thread_as_daemon(&self) -> Result<JNIEnv> {
        let mut ptr = ptr::null_mut();
        unsafe {
            let res = java_vm_unchecked!(
                self.0,
                AttachCurrentThreadAsDaemon,
                &mut ptr,
                ptr::null_mut()
            );
            jni_error_code_to_result(res)?;

            JNIEnv::from_raw(ptr as *mut sys::JNIEnv)
        }
    }

    /// Get the `JNIEnv` associated with the current thread, or
    /// `ErrorKind::Detached`
    /// if the current thread is not attached to the java VM.
    pub fn get_env(&self) -> Result<JNIEnv> {
        let mut ptr = ptr::null_mut();
        unsafe {
            let res = java_vm_unchecked!(self.0, GetEnv, &mut ptr, sys::JNI_VERSION_1_1);
            jni_error_code_to_result(res)?;

            JNIEnv::from_raw(ptr as *mut sys::JNIEnv)
        }
    }
}

/// A RAII implementation of scoped guard which detaches the current thread
/// when dropped. The attached `JNIEnv` can be accessed through this guard
/// via its `Deref` implementation.
pub struct AttachGuard<'a> {
    java_vm: &'a JavaVM,
    env: JNIEnv<'a>,
    requires_detach: bool,
}

impl<'a> AttachGuard<'a> {
    fn detach(&mut self) -> Result<()> {
        if self.requires_detach {
            unsafe {
                java_vm_unchecked!(self.java_vm.0, DetachCurrentThread);
            }
        }

        Ok(())
    }
}

impl<'a> Deref for AttachGuard<'a> {
    type Target = JNIEnv<'a>;

    fn deref(&self) -> &Self::Target {
        &self.env
    }
}

impl<'a> Drop for AttachGuard<'a> {
    fn drop(&mut self) {
        if let Err(e) = self.detach() {
            warn!("Error detaching current thread: {:#?}", e);
        }
    }
}
