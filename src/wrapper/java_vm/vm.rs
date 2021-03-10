use std::{
    cell::RefCell,
    ops::Deref,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
    thread::current,
};

use log::{debug, error};

use crate::{errors::*, sys, JNIEnv};

#[cfg(feature = "invocation")]
use crate::InitArgs;

/// The Java VM, providing [Invocation API][invocation-api] support.
///
/// The JavaVM can be obtained either via [`JNIEnv#get_java_vm`][get-vm] in an already attached
/// thread, or it can be [launched](#launching-jvm-from-rust) from Rust via `JavaVM#new`.
///
/// ## Attaching Native Threads
///
/// A native thread must «attach» itself to be able to call Java methods outside of a native Java
/// method. This library provides two modes of attachment, each ensuring the thread is promptly
/// detached:
/// * A scoped attachment with [`attach_current_thread`][act].
///   The thread will automatically detach itself once the returned guard is dropped.
/// * A permanent attachment with [`attach_current_thread_permanently`][actp]
///   or [`attach_current_thread_as_daemon`][actd].
///   The thread will automatically detach itself before it terminates.
///
/// As attachment and detachment of a thread is an expensive operation, the scoped attachment
/// shall be used if happens infrequently. If you have an undefined scope where you need
/// to use `JNIEnv` and cannot keep the `AttachGuard`, consider attaching the thread
/// permanently.
///
/// ### Local Reference Management
///
/// Remember that the native thread attached to the VM **must** manage the local references
/// properly, i.e., do not allocate an excessive number of references and release them promptly
/// when they are no longer needed to enable the GC to collect them. A common approach is to use
/// an appropriately-sized local frame for larger code fragments
/// (see [`with_local_frame`](struct.JNIEnv.html#method.with_local_frame) and [Executor](#executor))
/// and [auto locals](struct.JNIEnv.html#method.auto_local) in loops.
///
/// See also the [JNI specification][spec-references] for details on referencing Java objects.
///
/// ### Executor
///
/// Jni-rs provides an [`Executor`](struct.Executor.html) — a helper struct that allows to
/// execute a closure with `JNIEnv`. It combines the performance benefits of permanent attaches
/// *and* automatic local reference management. Prefer it to manual permanent attaches if
/// they happen in various parts of the code to reduce the burden of local reference management.
///
/// ## Launching JVM from Rust
///
/// To [launch][launch-vm] a JVM from a native process, enable the `invocation` feature
/// in the Cargo.toml:
///
/// ```toml
/// jni = { version = "0.19.0", features = ["invocation"] }
/// ```
///
/// The application will require linking to the dynamic `jvm` library, which is distributed
/// with the JVM, and allow to use `JavaVM#new`:
///
/// ```rust
/// # use jni::errors;
/// # //
/// # fn main() -> errors::Result<()> {
/// # // Ignore this test without invocation feature, so that simple `cargo test` works
/// # #[cfg(feature = "invocation")] {
/// # //
/// # use jni::{AttachGuard, objects::JValue, InitArgsBuilder, JNIEnv, JNIVersion, JavaVM, sys::jint};
/// # //
/// // Build the VM properties
/// let jvm_args = InitArgsBuilder::new()
///           // Pass the JNI API version (default is 8)
///           .version(JNIVersion::V8)
///           // You can additionally pass any JVM options (standard, like a system property,
///           // or VM-specific).
///           // Here we enable some extra JNI checks useful during development
///           .option("-Xcheck:jni")
///           .build()
///           .unwrap();
///
/// // Create a new VM
/// let jvm = JavaVM::new(jvm_args)?;
///
/// // Attach the current thread to call into Java — see extra options in
/// // "Attaching Native Threads" section.
/// //
/// // This method returns the guard that will detach the current thread when dropped,
/// // also freeing any local references created in it
/// let env = jvm.attach_current_thread()?;
///
/// // Call Java Math#abs(-10)
/// let x = JValue::from(-10);
/// let val: jint = env.call_static_method("java/lang/Math", "abs", "(I)I", &[x])?
///   .i()?;
///
/// assert_eq!(val, 10);
///
/// # }
/// # Ok(()) }
/// ```
///
/// During build time, the JVM installation path is determined:
/// 1. By `JAVA_HOME` environment variable, if it is set.
/// 2. Otherwise — from `java` output.
///
/// It is recommended to set `JAVA_HOME` to have reproducible builds,
/// especially, in case of multiple VMs installed.
///
/// At application run time, you must specify the path
/// to the `jvm` library so that the loader can locate it.
/// * On **Windows**, append the path to `jvm.dll` to `PATH` environment variable.
/// * On **MacOS**, append the path to `libjvm.dylib` to `LD_LIBRARY_PATH` environment variable.
/// * On **Linux**, append the path to `libjvm.so` to `LD_LIBRARY_PATH` environment variable.
///
/// The exact relative path to `jvm` library is version-specific.
///
/// For more information on linking — see documentation
/// in [build.rs](https://github.com/jni-rs/jni-rs/tree/master/build.rs).
///
/// [invocation-api]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html
/// [get-vm]: struct.JNIEnv.html#method.get_java_vm
/// [launch-vm]: struct.JavaVM.html#method.new
/// [act]: struct.JavaVM.html#method.attach_current_thread
/// [actp]: struct.JavaVM.html#method.attach_current_thread_permanently
/// [actd]: struct.JavaVM.html#method.attach_current_thread_as_daemon
/// [spec-references]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/design.html#referencing-java-objects
#[repr(transparent)]
pub struct JavaVM(*mut sys::JavaVM);

unsafe impl Send for JavaVM {}
unsafe impl Sync for JavaVM {}

impl JavaVM {
    /// Launch a new JavaVM using the provided init args.
    ///
    /// Unlike original JNI API, the main thread (the thread from which this method is called) will
    /// not be attached to JVM. You must explicitly use `attach_current_thread…` methods (refer
    /// to [Attaching Native Threads section](#attaching-native-threads)).
    ///
    /// *This API requires "invocation" feature to be enabled,
    /// see ["Launching JVM from Rust"](struct.JavaVM.html#launching-jvm-from-rust).*
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
    ///
    /// # Safety
    ///
    /// Expects a valid pointer retrieved from the `JNI_CreateJavaVM` JNI function. Only does null check.
    pub unsafe fn from_raw(ptr: *mut sys::JavaVM) -> Result<Self> {
        non_null!(ptr, "from_raw ptr argument");
        Ok(JavaVM(ptr))
    }

    /// Returns underlying `sys::JavaVM` interface.
    pub fn get_java_vm_pointer(&self) -> *mut sys::JavaVM {
        self.0
    }

    /// Attaches the current thread to the JVM. Calling this for a thread that is already attached
    /// is a no-op.
    ///
    /// The thread will detach itself automatically when it exits.
    ///
    /// Attached threads [block JVM exit][block]. If it is not desirable — consider using
    /// [`attach_current_thread_as_daemon`][attach-as-daemon].
    ///
    /// [block]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html#unloading-the-vm
    /// [attach-as-daemon]: struct.JavaVM.html#method.attach_current_thread_as_daemon
    pub fn attach_current_thread_permanently(&self) -> Result<JNIEnv> {
        match self.get_env() {
            Ok(env) => Ok(env),
            Err(_) => self.attach_current_thread_impl(ThreadType::Normal),
        }
    }

    /// Attaches the current thread to the Java VM. The returned `AttachGuard`
    /// can be dereferenced to a `JNIEnv` and automatically detaches the thread
    /// when dropped. Calling this in a thread that is already attached is a no-op, and
    /// will neither change its daemon status nor prematurely detach it.
    ///
    /// Attached threads [block JVM exit][block].
    ///
    /// Attaching and detaching a thread is an expensive operation. If you use it frequently
    /// in the same threads, consider either [attaching them permanently][attach-as-daemon],
    /// or, if the scope where you need the `JNIEnv` is well-defined, keeping the returned guard.
    ///
    /// [block]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html#unloading-the-vm
    /// [attach-as-daemon]: struct.JavaVM.html#method.attach_current_thread_as_daemon
    pub fn attach_current_thread(&self) -> Result<AttachGuard> {
        match self.get_env() {
            Ok(env) => Ok(AttachGuard::new_nested(env)),
            Err(_) => {
                let env = self.attach_current_thread_impl(ThreadType::Normal)?;
                Ok(AttachGuard::new(env))
            }
        }
    }

    /// Detaches current thread from the JVM. This operation is _rarely_ appropriate to use,
    /// because the attachment methods [ensure](#attaching-native-threads) that the thread is
    /// promptly detached.
    ///
    /// Detaching a non-attached thread is a no-op.
    ///
    /// __Any existing `JNIEnv`s and `AttachGuard`s created in the calling thread
    /// will be invalidated after this method completes. It is the__ caller’s __responsibility
    /// to ensure that no JNI calls are subsequently performed on these objects.__
    /// Failure to do so will result in unspecified errors, possibly, the process crash.
    ///
    /// Given some care is exercised, this method can be used to detach permanently attached
    /// threads _before_ they exit (when automatic detachment occurs). However, it is
    /// never appropriate to use it with the scoped attachment (`attach_current_thread`).
    // This method is hidden because it is almost never needed and its use requires some
    // extra care. Its status might be reconsidered if we learn of any use cases that require it.
    #[doc(hidden)]
    pub fn detach_current_thread(&self) {
        InternalAttachGuard::clear_tls();
    }

    /// Attaches the current thread to the Java VM as a _daemon_. Calling this in a thread
    /// that is already attached is a no-op, and will not change its status to a daemon thread.
    ///
    /// The thread will detach itself automatically when it exits.
    pub fn attach_current_thread_as_daemon(&self) -> Result<JNIEnv> {
        match self.get_env() {
            Ok(env) => Ok(env),
            Err(_) => self.attach_current_thread_impl(ThreadType::Daemon),
        }
    }

    /// Returns the current number of threads attached to the JVM.
    ///
    /// This method is provided mostly for diagnostic purposes.
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

    /// Creates `InternalAttachGuard` and attaches current thread.
    fn attach_current_thread_impl(&self, thread_type: ThreadType) -> Result<JNIEnv> {
        let guard = InternalAttachGuard::new(self.get_java_vm_pointer());
        let env_ptr = unsafe {
            if thread_type == ThreadType::Daemon {
                guard.attach_current_thread_as_daemon()?
            } else {
                guard.attach_current_thread()?
            }
        };

        InternalAttachGuard::fill_tls(guard);

        unsafe { JNIEnv::from_raw(env_ptr as *mut sys::JNIEnv) }
    }
}

thread_local! {
    static THREAD_ATTACH_GUARD: RefCell<Option<InternalAttachGuard>> = RefCell::new(None)
}

static ATTACHED_THREADS: AtomicUsize = AtomicUsize::new(0);

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
    fn new(java_vm: *mut sys::JavaVM) -> Self {
        Self { java_vm }
    }

    /// Stores guard in thread local storage.
    fn fill_tls(guard: InternalAttachGuard) {
        THREAD_ATTACH_GUARD.with(move |f| {
            *f.borrow_mut() = Some(guard);
        });
    }

    /// Clears thread local storage, dropping the InternalAttachGuard and causing detach of
    /// the current thread.
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

        debug!(
            "Attached thread {} ({:?}). {} threads attached",
            current().name().unwrap_or_default(),
            current().id(),
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

        debug!(
            "Attached daemon thread {} ({:?}). {} threads attached",
            current().name().unwrap_or_default(),
            current().id(),
            ATTACHED_THREADS.load(Ordering::SeqCst)
        );

        Ok(env_ptr as *mut sys::JNIEnv)
    }

    fn detach(&mut self) -> Result<()> {
        unsafe {
            java_vm_unchecked!(self.java_vm, DetachCurrentThread);
        }
        ATTACHED_THREADS.fetch_sub(1, Ordering::SeqCst);
        debug!(
            "Detached thread {} ({:?}). {} threads remain attached",
            current().name().unwrap_or_default(),
            current().id(),
            ATTACHED_THREADS.load(Ordering::SeqCst)
        );

        Ok(())
    }
}

impl Drop for InternalAttachGuard {
    fn drop(&mut self) {
        if let Err(e) = self.detach() {
            error!(
                "Error detaching current thread: {:#?}\nThread {} id={:?}",
                e,
                current().name().unwrap_or_default(),
                current().id(),
            );
        }
    }
}
