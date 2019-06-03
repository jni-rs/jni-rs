use JNIEnv;
use errors::*;

use sys;

use std::ptr;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::ops::Deref;
use std::thread::current;

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

    /// Attaches the current thread to a JVM. Calling this for a thread that is already attached
    /// is a no-op.
    ///
    /// Thread will be detached automatically when finished.
    ///
    /// Attached threads [block JVM exit][block].
    ///
    /// [block]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html#unloading-the-vm
    pub fn attach_current_thread_permanently(&self) -> Result<JNIEnv> {
        match self.get_env() {
            Ok(env) => Ok(env),
            Err(_) => {
                unsafe {
                    let env_ptr = InternalAttachGuard::create_and_attach(
                        self.get_java_vm_pointer(),
                        ThreadType::Normal
                    )?;
                    JNIEnv::from_raw(env_ptr)
                }
            }
        }
    }

    /// Attaches the current thread to a Java VM. The resulting `AttachGuard`
    /// can be dereferenced to a `JNIEnv` and automatically detaches the thread
    /// when dropped. Calling this for a thread that is already attached is a no-op.
    ///
    /// Attached threads [block JVM exit][block].
    ///
    /// Attaching and detaching is time-consuming operation, therefore multiple short-term attach
    /// operations on the same thread should be avoided or replaced with
    /// `attach_current_thread_permanently`.
    ///
    /// [block]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html#unloading-the-vm
    pub fn attach_current_thread(&self) -> Result<AttachGuard> {
        match self.get_env() {
            Ok(env) => {
                Ok(AttachGuard::new_nested(env))
            },
            Err(_) => {
                let env_ptr = InternalAttachGuard::create_and_attach(
                    self.get_java_vm_pointer(),
                    ThreadType::Normal
                )?;
                let env = unsafe {
                    JNIEnv::from_raw(env_ptr)?
                };
                Ok(AttachGuard::new(env))
            },
        }
    }

    /// Detaches current thread from the JVM.
    ///
    /// Detaching a non-attached thread is no-op.
    ///
    /// Calling this method is an equivalent for calling `drop()` for `AttachGuard`.
    pub fn detach_current_thread(&self) {
        InternalAttachGuard::clear_tls();
    }

    /// Attaches the current thread to a Java VM as a daemon.
    ///
    /// Thread will be automatically detached when finished.
    pub fn attach_current_thread_as_daemon(&self) -> Result<JNIEnv> {
        match self.get_env() {
            Ok(env) => Ok(env),
            Err(_) => {
                let env_ptr = InternalAttachGuard::create_and_attach(
                    self.get_java_vm_pointer(),
                    ThreadType::Daemon
                )?;
                unsafe { JNIEnv::from_raw(env_ptr) }
            }
        }
    }

    /// Returns current number of threads attached to the JVM.
    pub fn threads_attached(&self) -> usize {
        ATTACHED_THREADS.load(Ordering::SeqCst)
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

thread_local! {
    static THREAD_ATTACH_GUARD: RefCell<Option<InternalAttachGuard>> = RefCell::new(None)
}

static ATTACHED_THREADS: AtomicUsize = ATOMIC_USIZE_INIT;

/// A RAII implementation of scoped guard which detaches the current thread
/// when dropped. The attached `JNIEnv` can be accessed through this guard
/// via its `Deref` implementation.
pub struct AttachGuard<'a> {
    env: JNIEnv<'a>,
    should_detach: bool,
}

impl<'a> AttachGuard<'a> {
    /// AttachGuard created with this method will detach current thread on drop
    fn new(env: JNIEnv<'a>) -> Self {
        Self {
            env,
            should_detach: true,
        }
    }

    /// AttachGuard created with this method will not detach current thread on drop, which is
    /// the case for nested attaches.
    fn new_nested(env: JNIEnv<'a>) -> Self {
        Self {
            env,
            should_detach: false,
        }
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
        if self.should_detach {
            InternalAttachGuard::clear_tls();
        }
    }
}

#[derive(PartialEq)]
enum ThreadType {
    Normal,
    Daemon,
}

#[derive(Debug)]
struct InternalAttachGuard {
    java_vm: *mut sys::JavaVM,
}

impl InternalAttachGuard {
    fn create_and_attach(
        java_vm: *mut sys::JavaVM,
        ty: ThreadType,
    ) -> Result<*mut sys::JNIEnv> {
        let guard = InternalAttachGuard::new(java_vm);
        let env_ptr = unsafe {
            if ty == ThreadType::Daemon {
                guard.attach_current_thread_as_daemon()?
            } else {
                guard.attach_current_thread()?
            }
        };

        Self::fill_tls(guard);

        Ok(env_ptr)
    }

    fn new(java_vm: *mut sys::JavaVM) -> Self {
        Self {
            java_vm,
        }
    }

    /// Stores guard in thread local storage.
    fn fill_tls(guard: InternalAttachGuard) {
        THREAD_ATTACH_GUARD.with(move |f| {
            *f.borrow_mut() = Some(guard);
        });
    }

    /// Clears thread local storage, drops InternalAttachGuard and causes detach.
    fn clear_tls() {
        THREAD_ATTACH_GUARD.with(move |f| {
            *f.borrow_mut() = None;
        });
    }

    unsafe fn attach_current_thread(&self) -> Result<*mut sys::JNIEnv> {
        let mut env_ptr = ptr::null_mut();
        let res = java_vm_unchecked!(
            self.java_vm,
            AttachCurrentThread,
            &mut env_ptr,
            ptr::null_mut()
        );
        jni_error_code_to_result(res)?;

        ATTACHED_THREADS.fetch_add(1, Ordering::SeqCst);

        debug!("Attached thread {:?}. Name: {:?}. {} threads attached",
               current().id(),
               current().name(),
               ATTACHED_THREADS.load(Ordering::SeqCst)
        );

        Ok(env_ptr as *mut sys::JNIEnv)
    }

    unsafe fn attach_current_thread_as_daemon(&self) -> Result<*mut sys::JNIEnv> {
        let mut env_ptr = ptr::null_mut();
        let res = java_vm_unchecked!(
            self.java_vm,
            AttachCurrentThreadAsDaemon,
            &mut env_ptr,
            ptr::null_mut()
        );
        jni_error_code_to_result(res)?;

        ATTACHED_THREADS.fetch_add(1, Ordering::SeqCst);

        debug!("Attached daemon thread {:?}. Name: {:?}. {} threads attached",
               current().id(),
               current().name(),
               ATTACHED_THREADS.load(Ordering::SeqCst)
        );

        Ok(env_ptr as *mut sys::JNIEnv)
    }

    fn detach(&mut self) -> Result<()> {
        unsafe {
            java_vm_unchecked!(self.java_vm, DetachCurrentThread);
        }
        ATTACHED_THREADS.fetch_sub(1, Ordering::SeqCst);
        debug!("Detached thread {:?}. Name: {:?}. {} threads attached",
               current().id(),
               current().name(),
               ATTACHED_THREADS.load(Ordering::SeqCst)
        );

        Ok(())
    }
}

impl Drop for InternalAttachGuard {
    fn drop(&mut self) {
        if let Err(e) = self.detach() {
            error!("Error detaching current thread: {:#?}\nThread: {:?}, name: {:?}",
                   e,
                   current().id(),
                   current().name()
            );
        }
    }
}
