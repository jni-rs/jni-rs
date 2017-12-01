use errors::*;
use JNIEnv;

use sys;

use std::ptr;
use std::ops::Deref;

/// The invocation API.
pub struct JavaVM(*mut sys::JavaVM);

unsafe impl Send for JavaVM {}
unsafe impl Sync for JavaVM {}

impl JavaVM {
    /// Create a JavaVM from a raw pointer.
    pub fn from_raw(ptr: *mut sys::JavaVM) -> Result<Self> {
        non_null!(ptr, "from_raw ptr argument");
        Ok(JavaVM(ptr))
    }

    /// Attaches the current thread to a Java VM. The resulting `AttachGuard`
    /// can be dereferenced to a `JNIEnv` and automatically detaches the thread
    /// when dropped.
    pub fn attach_current_thread(&self) -> Result<AttachGuard> {
        let mut ptr = ptr::null_mut();
        unsafe {
            // TODO: Handle errors
            let _ = jni_unchecked!(self.0, AttachCurrentThread, &mut ptr, ptr::null_mut());
            let env = JNIEnv::from_raw(ptr as *mut sys::JNIEnv)?;
            Ok(AttachGuard {
                java_vm: self,
                env: env,
            })
        }
    }

    /// Attaches the current thread to a Java VM as a daemon.
    pub fn attach_current_thread_as_daemon(&self) -> Result<JNIEnv> {
        let mut ptr = ptr::null_mut();
        unsafe {
            // TODO: Handle errors
            let _ = jni_unchecked!(
                self.0,
                AttachCurrentThreadAsDaemon,
                &mut ptr,
                ptr::null_mut()
            );
            JNIEnv::from_raw(ptr as *mut sys::JNIEnv)
        }
    }

    /// Get the `JNIEnv` associated with the current thread, or `ErrorKind::Detached`
    /// if the current thread is not attached to the java VM.
    pub fn get_env(&self) -> Result<JNIEnv> {
        let mut ptr = ptr::null_mut();
        unsafe {
            let res = jni_unchecked!(self.0, GetEnv, &mut ptr, sys::JNI_VERSION_1_1);
            match res {
                sys::JNI_OK => JNIEnv::from_raw(ptr as *mut sys::JNIEnv),
                sys::JNI_EDETACHED => Err(Error::from(ErrorKind::Detached)),
                _ => unreachable!(),
            }
        }
    }
}

/// A RAII implementation of scoped guard which detaches the current thread
/// when dropped. The attached `JNIEnv` can be accessed through this guard
/// via its `Deref` implementation.
pub struct AttachGuard<'a> {
    java_vm: &'a JavaVM,
    env: JNIEnv<'a>,
}

impl<'a> AttachGuard<'a> {
    fn detach(&mut self) -> Result<()> {
        unsafe {
            jni_unchecked!(self.java_vm.0, DetachCurrentThread);
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
        match self.detach() {
            Ok(()) => (),
            Err(e) => debug!("error detaching current thread: {:#?}", e),
        }
    }
}
