use std::{
    cell::{Cell, RefCell},
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
    thread::{current, Thread},
};

use log::{debug, error};

use crate::{errors::*, objects::JObject, sys, JNIEnv, JNIVersion};

#[cfg(feature = "invocation")]
use {
    crate::InitArgs,
    std::os::raw::c_void,
    std::{ffi::OsStr, path::PathBuf},
};

/// The capacity of local frames, allocated for attached threads by default. Same as the default
/// value Hotspot uses when calling native Java methods.
pub const DEFAULT_LOCAL_FRAME_CAPACITY: usize = 32;

/// The Java VM, providing [Invocation API][invocation-api] support.
///
/// The JavaVM can be obtained either via [`JNIEnv#get_java_vm`][get-vm] in an already attached
/// thread, or it can be [launched](#launching-jvm-from-rust) from Rust via `JavaVM#new`.
///
/// ## Attaching Native Threads
///
/// A native thread must «attach» itself to be able to call Java methods outside of a native Java
/// method.
///
/// The attachment of the current thread is always explicitly represented via an [`AttachGuard`]
/// which blocks the thread from being detached and provides access to the [`JNIEnv`] API.
///
/// This library provides two modes of attachment, each ensuring the thread is automatically
/// detached:
/// * A permanent attachment with [`attach_current_thread`][actp]
///   The thread will automatically detach itself before it terminates.
/// * A scoped attachment with [`attach_current_thread_for_scope`][act].
///   The thread will automatically detach itself once the returned guard is dropped.
///
/// Both APIs return an [`AttachGuard`] that only guarantees that the thread is attached
/// until the guard is dropped, but [`Self::attach_current_thread()`] will request
/// a permanent attachment which will increase the chance that future attachment
/// calls will be cheap if the thread is already attached.
///
/// ### Local Reference Management
///
/// Remember that the native thread attached to the VM **must** manage the local references
/// properly, i.e., do not allocate an excessive number of references and release them promptly
/// when they are no longer needed to enable the GC to collect them. A common approach is to use
/// an appropriately-sized local frame for larger code fragments
/// (see [`with_local_frame`](struct.JNIEnv.html#method.with_local_frame) and [`AttachGuard::with_env`]
/// and [auto locals](struct.JNIEnv.html#method.auto_local) in loops.
///
/// See also the [JNI specification][spec-references] for details on referencing Java objects.
///
/// ### `AttachGuard::with_env`
///
/// The [`AttachGuard::with_env`] API is convenient way to access the [`JNIEnv`] API
/// while also creating a new JNI stack frame so that any local references created while
/// running the given closure will be automatically released after the closure returns.
///
/// Prefer it to [`AttachGuard::current_frame_env`] if you don't know when the current
/// frame will unwind and don't know when references for the current frame will be released.
///
/// ## Launching JVM from Rust
///
/// To [launch][launch-vm] a JVM from a native process, enable the `invocation`
/// feature in the Cargo.toml:
///
/// ```toml
/// jni = { version = "0.21.1", features = ["invocation"] }
/// ```
///
/// The application will be able to use [`JavaVM::new`] which will dynamically
/// load a `jvm` library (which is distributed with the JVM) at runtime:
///
/// ```rust
/// # use jni::errors;
/// # //
/// # // Ignore this test without invocation feature, so that simple `cargo test` works
/// # #[cfg(feature = "invocation")]
/// # fn main() -> errors::StartJvmResult<()> {
/// # use jni::{AttachGuard, objects::JValue, InitArgsBuilder, JNIEnv, JNIVersion, JavaVM, sys::jint};
/// # //
/// // Build the VM properties
/// let jvm_args = InitArgsBuilder::new()
///           // Pass the JNI API version (default is 8)
///           .version(JNIVersion::V1_8)
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
/// let mut guard = unsafe { jvm.attach_current_thread(JNIVersion::V1_4)? };
/// guard.with_env(1, |env| -> errors::Result<()> {
///     // Call Java Math#abs(-10)
///     let x = JValue::from(-10);
///     let val: jint = env.call_static_method("java/lang/Math", "abs", "(I)I", &[x])?
///         .i()?;
///
///     assert_eq!(val, 10);
///     Ok(())
/// });
///
/// # Ok(()) }
/// #
/// # // This is a stub that gets run instead if the invocation feature is not built
/// # #[cfg(not(feature = "invocation"))]
/// # fn main() {}
/// ```
///
/// At runtime, the JVM installation path is determined via the [java-locator] crate:
/// 1. By the `JAVA_HOME` environment variable, if it is set.
/// 2. Otherwise — from `java` output.
///
/// It is recommended to set `JAVA_HOME`
///
/// For the operating system to correctly load the `jvm` library it may also be
/// necessary to update the path that the OS uses to find dependencies of the
/// `jvm` library.
/// * On **Windows**, append the path to `$JAVA_HOME/bin` to the `PATH` environment variable.
/// * On **MacOS**, append the path to `libjvm.dylib` to `LD_LIBRARY_PATH` environment variable.
/// * On **Linux**, append the path to `libjvm.so` to `LD_LIBRARY_PATH` environment variable.
///
/// The exact relative path to `jvm` library is version-specific.
///
/// [invocation-api]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html
/// [get-vm]: struct.JNIEnv.html#method.get_java_vm
/// [launch-vm]: struct.JavaVM.html#method.new
/// [act]: struct.JavaVM.html#method.attach_current_thread
/// [actp]: struct.JavaVM.html#method.attach_current_thread_permanently
/// [spec-references]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/design.html#referencing-java-objects
/// [java-locator]: https://crates.io/crates/java-locator
#[repr(transparent)]
#[derive(Debug, Clone)]
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
    /// *This API requires the "invocation" feature to be enabled,
    /// see ["Launching JVM from Rust"](struct.JavaVM.html#launching-jvm-from-rust).*
    ///
    /// This will attempt to locate a JVM using
    /// [java-locator], if the JVM has not already been loaded. Use the
    /// [`with_libjvm`][Self::with_libjvm] method to give an explicit location for the JVM shared
    /// library (`jvm.dll`, `libjvm.so`, or `libjvm.dylib`, depending on the platform).
    #[cfg(feature = "invocation")]
    pub fn new(args: InitArgs) -> StartJvmResult<Self> {
        Self::with_libjvm(args, || {
            Ok([
                java_locator::locate_jvm_dyn_library()
                    .map_err(StartJvmError::NotFound)?
                    .as_str(),
                java_locator::get_jvm_dyn_lib_file_name(),
            ]
            .iter()
            .collect::<PathBuf>())
        })
    }

    /// Launch a new JavaVM using the provided init args, loading it from the given shared library file if it's not already loaded.
    ///
    /// Unlike original JNI API, the main thread (the thread from which this method is called) will
    /// not be attached to JVM. You must explicitly use `attach_current_thread…` methods (refer
    /// to [Attaching Native Threads section](#attaching-native-threads)).
    ///
    /// *This API requires the "invocation" feature to be enabled,
    /// see ["Launching JVM from Rust"](struct.JavaVM.html#launching-jvm-from-rust).*
    ///
    /// The `libjvm_path` parameter takes a *closure* which returns the path to the JVM shared
    /// library. The closure is only called if the JVM is not already loaded. Any work that needs
    /// to be done to locate the JVM shared library should be done inside that closure.
    #[cfg(feature = "invocation")]
    pub fn with_libjvm<P: AsRef<OsStr>>(
        args: InitArgs,
        libjvm_path: impl FnOnce() -> StartJvmResult<P>,
    ) -> StartJvmResult<Self> {
        // Determine the path to the shared library.
        let libjvm_path = libjvm_path()?;
        let libjvm_path_string = libjvm_path.as_ref().to_string_lossy().into_owned();

        // Try to load it.
        let libjvm = match unsafe { libloading::Library::new(libjvm_path.as_ref()) } {
            Ok(ok) => ok,
            Err(error) => return Err(StartJvmError::LoadError(libjvm_path_string, error)),
        };

        let result = unsafe {
            // Try to find the `JNI_CreateJavaVM` function in the loaded library.
            let create_fn = libjvm
                .get(b"JNI_CreateJavaVM\0")
                .map_err(|error| StartJvmError::LoadError(libjvm_path_string.to_owned(), error))?;

            // Create the JVM.
            Self::with_create_fn_ptr(args, *create_fn).map_err(StartJvmError::Create)
        };

        // Prevent libjvm from being unloaded.
        //
        // If libjvm is unloaded while the JVM is running, the program will crash as soon as it
        // tries to execute any JVM code, including the many threads that the JVM automatically
        // creates.
        //
        // For reasons unknown, HotSpot seems to somehow prevent itself from being unloaded, so it
        // will work even if this `forget` call isn't here, but there's no guarantee that other JVM
        // implementations will also prevent themselves from being unloaded.
        //
        // Note: `jni-rs` makes the assumption that there can only ever be a single `JavaVM`
        // per-process and it's never possible to full destroy and unload a JVM once it's been
        // created. Calling `DestroyJavaVM` is only expected to release some resources and
        // leave the JVM in a poorly-defined limbo state that doesn't allow unloading.
        // Ref: https://github.com/jni-rs/jni-rs/issues/567
        //
        // See discussion at: https://github.com/jni-rs/jni-rs/issues/550
        std::mem::forget(libjvm);

        result
    }

    #[cfg(feature = "invocation")]
    unsafe fn with_create_fn_ptr(
        args: InitArgs,
        create_fn_ptr: unsafe extern "system" fn(
            pvm: *mut *mut sys::JavaVM,
            penv: *mut *mut c_void,
            args: *mut c_void,
        ) -> sys::jint,
    ) -> Result<Self> {
        let mut ptr: *mut sys::JavaVM = ::std::ptr::null_mut();
        let mut env: *mut sys::JNIEnv = ::std::ptr::null_mut();

        jni_error_code_to_result(create_fn_ptr(
            &mut ptr as *mut _,
            &mut env as *mut *mut sys::JNIEnv as *mut *mut c_void,
            args.inner_ptr(),
        ))?;

        let vm = Self::from_raw(ptr)?;
        java_vm_call_unchecked!(vm, v1_1, DetachCurrentThread);

        Ok(vm)
    }

    /// Create a JavaVM from a raw pointer.
    ///
    /// # Safety
    ///
    /// Expects a valid, non-null JavaVM pointer that supports JNI version >= 1.4.
    ///
    /// Only does a `null` check.
    pub unsafe fn from_raw(ptr: *mut sys::JavaVM) -> Result<Self> {
        let ptr = null_check!(ptr, "from_raw ptr argument")?;
        Ok(JavaVM(ptr))
    }

    /// Returns underlying `sys::JavaVM` interface.
    pub fn get_raw(&self) -> *mut sys::JavaVM {
        self.0
    }

    /// Attaches the current thread to the Java VM and returns an
    /// [`AttachGuard`] to access the [`JNIEnv`] API for the current thread,
    /// (E.g. via [`AttachGuard::with_env()`]).
    ///
    /// If the thread was not already attached then a new attachment is made
    /// which will be automatically detached when the current thread terminates.
    ///
    /// Calling this in a thread that is already attached is a cheap no-op that
    /// will return an [`AttachGuard`] that does nothing when dropped.
    ///
    /// This API requests to permanently attach the current thread but since
    /// pre-existing attachments aren't affected by this API, it should
    /// therefore not be assumed that the thread will definitely remain attached
    /// until it exits - that is only a request.
    ///
    /// You can safely assume that the thread will remain attached for the
    /// current scope, at least until the returned [`AttachGuard`] is dropped.
    ///
    /// If you're not sure whether to use [`Self::attach_current_thread`] or
    /// [`Self::attach_current_thread_for_scope`], then you should probably use
    /// this API because it increases the chance that future attachment calls
    /// will be cheap.
    ///
    /// # Safety
    ///
    /// You must consider the 'Safety' documentation for [`AttachGuard`].
    ///
    /// In summary though:
    ///
    /// 1. This must not be used to materialize an [`AttachGuard`] if you
    ///    already have a guard accessible to your current scope, or if you have
    ///    a safe way to access a mutable [`JNIEnv`].
    ///
    /// 2. The returned guard must be kept on the stack (not boxed or given a
    ///    `'static` lifetime in any way) and should generally be considered
    ///    like an immovable type, to ensure that guards are always dropped in
    ///    LIFO order.
    pub unsafe fn attach_current_thread(&self, version: JNIVersion) -> Result<AttachGuard> {
        // Safety: the caller must ensure that no other guard / JNIEnv in scope,
        unsafe {
            match self.get_env_attachment(version) {
                Ok(guard) => Ok(guard),
                Err(Error::JniCall(JniError::ThreadDetached)) => TLSAttachGuard::attach_current_thread(self.clone()),
                Err(err) => Err(err),
            }
        }
    }

    /// Attaches the current thread to the Java VM and returns an
    /// [`AttachGuard`] to access the [`JNIEnv`] API for the current thread,
    /// (E.g. via [`AttachGuard::with_env()`]).
    ///
    /// If the thread was not already attached, the returned guard detaches the
    /// thread when dropped.
    ///
    /// Calling this in a thread that is already attached is a cheap no-op that
    /// will return an [`AttachGuard`] that does nothing when dropped.
    ///
    /// Attaching a thread is an expensive operation if it was not already
    /// attached, so it's generally recommended that you should use
    /// [`Self::attach_current_thread()`] (requesting to attach the thread
    /// permanently) instead of using a scoped attachment. Using this API may
    /// increase the chance that you incur the cost of repeatedly attaching and
    /// detaching the same thread.
    ///
    /// # Safety
    ///
    /// You must consider the 'Safety' documentation for [`AttachGuard`].
    ///
    /// In summary though:
    ///
    /// 1. This must not be used to materialize an [`AttachGuard`] if you
    ///    already have a guard accessible to your current scope, or if you have
    ///    a safe way to access a mutable [`JNIEnv`].
    ///
    /// 2. The returned guard must be kept on the stack (not boxed or given a
    ///    `'static` lifetime in any way) and should generally be considered
    ///    like an immovable type, to ensure that guards are always dropped in
    ///    LIFO order.
    pub unsafe fn attach_current_thread_for_scope<'local>(&self, version: JNIVersion) -> Result<AttachGuard> {
        // Safety: the caller must ensure that no other guard / JNIEnv in scope,
        unsafe {
            match self.get_env_attachment(version) {
                Ok(guard) => Ok(guard),
                Err(Error::JniCall(JniError::ThreadDetached)) => {
                    let jni = sys_attach_current_thread(self, &current())?;
                    Ok(AttachGuard::from_owned(jni))
                },
                Err(err) => Err(err),
            }
        }
    }

    /// Explicitly detaches the current thread from the JVM, **IFF** it was
    /// previously attached using [`JavaVM::attach_current_thread`] **AND** if
    /// there is no [`AttachGuard`] also keeping the current thread attached.
    ///
    /// This will always return an error if there are currently any active
    /// [`AttachGuard`]s (detaching the thread in this case would effectively
    /// turn guards into invalid, dangling pointers).
    ///
    /// Detaching a non-attached thread is a no-op that won't return an error
    /// (assuming there's no active [`AttachGuard`] as noted above).
    ///
    /// This API has no effect on scoped attachments that were created via
    /// [`JavaVM::attach_current_thread_for_scope`]. Or in other words it's not
    /// applicable to scoped attachments because it's an error to call while
    /// there are active [`AttachGuard`]s.
    ///
    /// _**Note**: This operation is _rarely_ necessary to use, because a
    /// thread that is attached via [`JavaVM::attach_current_thread`] will
    /// automatically detach when that thread terminates.
    ///
    /// Explicitly detaching the thread could lead to overheads later if the
    /// same thread needs to get re-attached.
    ///
    /// If there is a need to detach a thread before it terminates, then it's
    /// possible that a scoped attachment via
    /// [`JavaVM::attach_current_thread_for_scope`] could be used so that the
    /// detachment would happen automatically.
    pub fn detach_current_thread(&self) -> Result<()> {
        TLSAttachGuard::detach()
    }

    /// Returns the current number of threads attached to the JVM.
    ///
    /// This method is provided mostly for diagnostic purposes.
    #[doc(hidden)]
    pub fn threads_attached(&self) -> usize {
        ATTACHED_THREADS.load(Ordering::SeqCst)
    }

    /// Returns the current nesting level for [`AttachGuard`]s
    ///
    /// This is only really public since it's useful for unit tests
    #[doc(hidden)]
    pub fn thread_attach_guard_level(&self) -> usize {
        THREAD_GUARD_NEST_LEVEL.get()
    }

    /// Get an [`AttachGuard`] for the [`JNIEnv`] associated with the current
    /// thread or, if JNI is not attached to the Java VM, this will return
    /// [`Error::JniCall()`] with [`JniError::ThreadDetached`].
    ///
    /// You must specify what JNI `version` you require, with a minimum of
    /// [`JNIVersion::V1_4`]
    ///
    /// # Safety
    ///
    /// You must know that the [`JavaVM`] supports at least JNI >= 1.4
    ///
    /// (The implementation is not able to call `GetEnv` before 1.2 and the
    /// implementation can't validate the `JNIEnv` version by calling
    /// `GetVersion` if exceptions might be pending since `GetVersion` is not
    /// documented as safe to call with pending exceptions)
    ///
    /// This must not be used to materialize a [`AttachGuard`] if there is
    /// already another guard or mutable [`JNIEnv`] in scope (or anything that
    /// could provide "safe" access to a mutable [`JNIEnv`]).
    ///
    /// This is because a [`JNIEnv`] has a lifetime parameter that ties it to a
    /// local JNI stack frame (which holds local object references) and an
    /// existing, mutable [`JNIEnv`] could enable the creation of local
    /// references that would be tied to the wrong JNI stack frame.
    pub unsafe fn get_env_attachment(&self, version: JNIVersion) -> Result<AttachGuard> {
        let mut ptr = ptr::null_mut();
        if version < JNIVersion::V1_4 {
            return Err(Error::UnsupportedVersion);
        }

        unsafe {
            let res = java_vm_call_unchecked!(self, v1_2, GetEnv, &mut ptr, version.into());
            jni_error_code_to_result(res)?;
            let jni = ptr as *mut sys::JNIEnv;
            Ok(AttachGuard::from_unowned(jni))
        }
    }

    /// Unloads the JavaVM and frees all it's associated resources
    ///
    /// Firstly if this thread is not already attached to the `JavaVM` then it
    /// will be attached.
    ///
    /// This thread will then wait until there are no other non-daemon threads
    /// attached to the `JavaVM` before unloading it (including threads spawned
    /// by Java and those that are attached via JNI)
    ///
    /// # Safety
    ///
    /// IF YOU ARE USING DAEMON THREADS THIS MAY BE DIFFICULT TO USE SAFELY!
    ///
    /// ## Daemon thread rules
    ///
    /// Since the JNI spec makes it clear that [`DestroyJavaVM`][destroy] will
    /// not wait for attached deamon threads to exit, this also means that if
    /// you do have any attached daemon threads it is your responsibility to
    /// ensure that they don't try and use JNI after the `JavaVM` is destroyed
    /// and you won't be able to detach them after the `JavaVM` has been
    /// destroyed.
    ///
    /// This creates a very unsafe hazard if using `jni-rs` due to the various
    /// RAII types that will automatically make JNI calls within their `Drop`
    /// implementation.
    ///
    /// For this reason `jni-rs` doesn't directly support attaching or detaching
    /// 'daemon' threads and it's assumed you will manage their safety yourself
    /// if you're using them.
    ///
    /// Note: [`JavaVM::detach_current_thread()`] is a no-op for daemon threads
    /// because it will only detach threads that were attached via `jni-rs` APIs.
    ///
    /// ## Don't call from a Java native function
    ///
    /// There must be no Java methods on the call stack when `JavaVM::destroy()`
    /// is called.
    ///
    /// ## Drop all JNI state, including auto-release types before calling `JavaVM::destroy()`
    ///
    /// There is currently no `'vm` lifetime associated with a `JavaVM` that
    /// would allow the borrow checker to enforce that all `jni` resources
    /// associated with the `JavaVM` have been released.
    ///
    /// Since these JNI resources could lead to undefined behaviour through any
    /// use after the `JavaVM` has been destroyed then it is your responsibility
    /// to release these resources.
    ///
    /// In particular, there are numerous auto-release types in the `jni` API
    /// that will automatically make JNI calls within their `Drop`
    /// implementation. All such types _must_ be dropped before `destroy()` is
    /// called to avoid undefined bahaviour.
    ///
    /// Here is an non-exhaustive list of auto-release types to consider:
    /// - `AttachGuard`
    /// - `AutoElements`
    /// - `AutoElementsCritical`
    /// - `AutoLocal`
    /// - `GlobalRef`
    /// - `JavaStr`
    /// - `JMap`
    /// - `WeakRef`
    ///
    /// ## Invalid `JavaVM` on return
    ///
    /// After `destroy()` returns then the `JavaVM` will be in an undefined
    /// state and must be dropped (e.g. via `std::mem::drop()`) to avoid
    /// undefined behaviour.
    ///
    /// This method doesn't take ownership of the `JavaVM` before it is
    /// destroyed because the `JavaVM` may have been shared (E.g. via an `Arc`)
    /// between all the threads that have not yet necessarily exited before this
    /// is called.
    ///
    /// So although the `JavaVM` won't necessarily be solely owned by this
    /// thread when `destroy()` is first called it will conceptually own the
    /// `JavaVM` before `destroy()` returns.
    ///
    /// [destroy]:
    ///     https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html#unloading-the-vm
    pub unsafe fn destroy(&self) -> Result<()> {
        unsafe {
            let res = java_vm_call_unchecked!(self, v1_1, DestroyJavaVM);
            jni_error_code_to_result(res)
        }
    }
}

static ATTACHED_THREADS: AtomicUsize = AtomicUsize::new(0);

unsafe fn sys_attach_current_thread(vm: &JavaVM, thread: &Thread) -> Result<*mut sys::JNIEnv> {
    let mut env_ptr = ptr::null_mut();
    let res = java_vm_call_unchecked!(vm, v1_1, AttachCurrentThread, &mut env_ptr, ptr::null_mut());
    jni_error_code_to_result(res)?;

    ATTACHED_THREADS.fetch_add(1, Ordering::SeqCst);

    debug!(
        "Attached thread {} ({:?}). {} threads attached",
        thread.name().unwrap_or_default(),
        thread.id(),
        ATTACHED_THREADS.load(Ordering::SeqCst)
    );

    Ok(env_ptr as *mut sys::JNIEnv)
}

/// Detach a thread, asserting that we own the current attachment and have a valid `JNIEnv` pointer
///
/// Although `DetachCurrentThread` is part of the `JavaVM` "invocation" API and doesn't require a
/// `JNIEnv` pointer, we want to constrain this code to only ever detach threads if we own the
/// current attachment.
unsafe fn sys_detach_current_thread(env_ptr: *mut jni_sys::JNIEnv, thread: &Thread) -> Result<()> {
    unsafe {
        fn get_vm(env_ptr: *mut jni_sys::JNIEnv) -> Result<JavaVM> {
            let env = unsafe { JNIEnv::from_raw_unchecked(env_ptr) };
            env.get_java_vm()
        }
        let mut vm = get_vm(env_ptr)?;

        fn check_current_attachment_matches(vm: &mut JavaVM, env_ptr: *mut jni_sys::JNIEnv) -> Result<()> {
            let mut guard = unsafe { vm.get_env_attachment(JNIVersion::V1_4)? };
            let attached_env = guard.current_frame_env().get_raw();
            if attached_env != env_ptr {
                return Err(Error::JniCall(JniError::InvalidArguments))
            }
            Ok(())
        }
        check_current_attachment_matches(&mut vm, env_ptr)?;

        java_vm_call_unchecked!(vm, v1_1, DetachCurrentThread);
    }
    ATTACHED_THREADS.fetch_sub(1, Ordering::SeqCst);

    debug!(
        "Detached thread {} ({:?}). {} threads remain attached",
        thread.name().unwrap_or_default(),
        thread.id(),
        ATTACHED_THREADS.load(Ordering::SeqCst)
    );

    Ok(())
}


thread_local! {
    static THREAD_GUARD_NEST_LEVEL: Cell<usize> = const { Cell::new(0) };
}

/// Represents a JNI attachment of the current thread to a Java VM, which is
/// required before you can access the [`JNIEnv`] API.
///
/// If the [`AttachGuard`] "owns" the underlying JNI thread attachment, that
/// means the guard will automatically detach the current thread from the Java
/// VM when the guard is dropped.
///
/// See [`JavaVM::attach_current_thread()`],
/// [`JavaVM::attach_current_thread_for_scope`] or
/// [`AttachGuard::from_unowned()`] for creating thread attachment guards.
///
/// If you're implementing a JNI native method which is passed a raw
/// [`crate::sys::JNIEnv`] pointer, then you can get a corresponding guard via
/// [`AttachGuard::from_unowned`].
///
/// If you're implementing some JNI utility code that doesn't already have a raw
/// [`crate::sys::JNIEnv`] pointer you should probably use
/// [`JavaVM::attach_current_thread`] to get an attachment guard, and to also
/// request that the thread remains permanently attached (avoiding any repeated
/// overhead from attaching and detaching the current thread).
///
/// If you need an attachment guard in some case where you're concerned about
/// having any side effects you can use
/// [`JavaVM::attach_current_thread_for_scope`] to request an owned attachment
/// guard that will detach the thread when dropped. Consider though that this
/// may increase the chance that your code will be repeatedly attaching and
/// detaching the same thread, which will incur more overhead than a permanent
/// attachment would.
///
/// # Safety
///
/// Thread attachment is always considered to be an `unsafe` operation (and
/// functions like [`JavaVM::attach_current_thread()`] that can return a guard
/// are `unsafe`) because there some safety rules for managing `AttachGuard`s
/// that can't be automatically guaranteed through the Rust type system alone...
///
/// 1. You must never materialise a thread attachment guard into any scope where
///    you already have an accessible [`AttachGuard`] or where you have some
///    safe way of accessing a mutable [`JNIEnv`].
///
///    It _is_ OK to create a redundant [`AttachGuard`] in case there may
///    already be a guard for an attachment lower on the stack (owned by some
///    function that has called you) but it's not safe if the code in your
///    current scope can directly access a pre-existing guard or mutable
///    [`JNIEnv`].
///
/// 2. You must treat a guard as an immovable type that needs to live on the
///    stack and can't be given a `'static` lifetime (e.g. by boxing or moving
///    into a `static` variable) or re-ordered relative to other guards on the
///    stack.
///
///    When a guard is borrowed to access a [`JNIEnv`] reference, it would not
///    be safe if you could give yourself access to a `'static` `JNIEnv`
///    reference, because the lifetime associated with a `JNIEnv` is used to
///    associate JNI local references with a JNI stack frame.
///
/// # Panics
///
///    The `Drop` implementation will `panic` if a guard is not dropped in the
///    same order that it was created, relative to other guards (LIFO order).
pub struct AttachGuard {
    // Note: we cast away this 'static lifetime before exposing it publicly.
    // We use `'static` because we don't want a lifetime parameter for
    // `AttachGuard` which doesn't borrow anything. The lifetime we hand out
    // will be the lifetime of the `&self` reference
    //
    // TODO: I think we may be able to remove the JNIEnv lifetime if we can
    // instead assume a JNIEnv is always borrowed from an AttachGuard - since we
    // can instead name the lifetime of the reference to associate with JNI
    // local references.
    env: JNIEnv<'static>,
    should_detach: bool,
    level: usize
}

fn thread_guard_level_inc() -> usize {
    THREAD_GUARD_NEST_LEVEL.with(|cell| {
        let level = cell.get();
        cell.set(level + 1);
        level
    })
}

fn thread_guard_level_dec() -> usize {
    THREAD_GUARD_NEST_LEVEL.with(|cell| {
        let level = cell.get();
        assert_ne!(level, 0, "Spuriously dropped more AttachGuards than were known to exist");
        cell.set(level - 1);
        level - 1
    })
}

impl AttachGuard {
    /// Wrap a raw [`sys::JNIEnv`] pointer in an `AttachGuard` that will detach
    /// the current thread on drop.
    ///
    /// # Safety
    ///
    /// The pointer must be non-null and correspond to a valid [`JNIEnv`]
    /// pointer that is attached to the current thread.
    ///
    /// This must not be used to materialize a thread attachment guard while
    /// another attach guard, or any other mutable `JNIEnv` is in scope.
    ///
    /// The guard should be treated as immovable and kept on the stack for the
    /// current thread, and more-specifically it must not be moved to a new JNI
    /// stack frame.
    unsafe fn from_owned(env: *mut sys::JNIEnv) -> Self {
        Self {
            // TODO: make the JNIEnv non-transparent and read
            // `jvm.thread_attach_guard_level()` for validating that only the
            // environment at the top of the stack is ever usable for creating
            // new local references (in addition to requiring a `&mut` reference)
            env: JNIEnv::from_raw_unchecked(env),
            should_detach: true,
            level: thread_guard_level_inc()
        }
    }

    /// Wrap a raw [`sys::JNIEnv`] pointer in an `AttachGuard` that does not own
    /// the underlying thread attachment and so it will **NOT** detach the
    /// current thread on drop.
    ///
    /// This can be use when implementing native JNI methods (that are passed an
    /// attached [`sys::JNIEnv`] pointer) as a way to access the [`JNIEnv`] API.
    ///
    /// # Safety
    ///
    /// The pointer must be non-null and correspond to a valid [`JNIEnv`]
    /// pointer that is attached to the current thread.
    ///
    /// This must not be used to materialize a thread attachment guard while
    /// another attach guard, or any other mutable `JNIEnv` is in scope.
    ///
    /// The guard should be treated as immovable and kept on the stack for the
    /// current thread, and more-specifically it must not be moved to a new JNI
    /// stack frame.
    pub unsafe fn from_unowned(env: *mut sys::JNIEnv) -> Self {
        Self {
            env: JNIEnv::from_raw_unchecked(env),
            should_detach: false,
            level: thread_guard_level_inc()
        }
    }

    /// Returns true if the guard represents a scoped attachment that will also
    /// detach the thread when it is dropped.
    ///
    /// Note that not all scoped guards from
    /// [`JavaVM::attach_current_thread_for_scope`] will own the attachment,
    /// since the scope may be nested under some other guard, lower on the stack
    /// that has already attached the thread.
    pub fn owns_attachment(&self) -> bool {
        self.should_detach
    }

    /// Borrows a mutable reference to a [`JNIEnv`] with a lifetime that will
    /// associate local references with the current JNI stack frame.
    ///
    /// Beware of using this API without considering how you will ensure that
    /// local references in the current JNI stack frame will eventually get
    /// freed.
    ///
    /// If you're not sure, it may be best to use [`AttachGuard::with_env`]
    /// which will create a new JNI stack frame for the given closure and then
    /// all local references created within that closure will be freed before it
    /// returns. This may avoid situations where you slowly leak a large number
    /// of references if the current frame is not unwound for a long time.
    pub fn current_frame_env<'local>(&'local mut self) -> &'local mut JNIEnv<'local> {
        // Assuming that the application doesn't break the safety rules for
        // keeping the `AttachGuard` on the stack, and not re-ordering them,
        // we can assert that we will only ever borrow from the top-most
        // guard on the stack
        assert_eq!(THREAD_GUARD_NEST_LEVEL.get(), self.level + 1);
        // Cast away the `'static` lifetime
        unsafe { std::mem::transmute(&mut self.env) }
    }

    /// Runs a closure with a borrowed [`JNIEnv`] associated with a new JNI stack
    /// frame that will be unwound to release all local references created within
    /// the given closure.
    pub fn with_env<F, T, E>(&mut self,
        capacity: usize,
        f: F,
    ) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut JNIEnv) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        // Assuming that the application doesn't break the safety rules for
        // keeping the `AttachGuard` on the stack, and not re-ordering them,
        // we can assert that we will only ever borrow from the top-most
        // guard on the stack
        assert_eq!(THREAD_GUARD_NEST_LEVEL.get(), self.level + 1);
        // Safety: the caller must ensure that no other mutable `JNIEnv` in scope,
        self.current_frame_env().with_local_frame(capacity, |jni_env| f(jni_env))
    }

    /// Runs a closure with a borrowed [`JNIEnv`] associated with a new JNI stack
    /// frame that will be unwound to release all local references created within
    /// the given closure, except for a single return value reference.
    pub fn with_env_returning_local<'local, F, T, E>(&'local mut self,
        capacity: usize,
        f: F,
    ) -> std::result::Result<JObject<'local>, E>
    where
        F: for<'new_local> FnOnce(
            &mut JNIEnv<'new_local>,
        ) -> std::result::Result<JObject<'new_local>, E>,
        E: From<Error>,
    {
        // Assuming that the application doesn't break the safety rules for
        // keeping the `AttachGuard` on the stack, and not re-ordering them,
        // we can assert that we will only ever borrow from the top-most
        // guard on the stack
        assert_eq!(THREAD_GUARD_NEST_LEVEL.get(), self.level + 1);
        // Safety: the caller must ensure that no other mutable `JNIEnv` in scope,
        self.current_frame_env().with_local_frame_returning_local(capacity, |jni_env| f(jni_env))
    }

    /// Handles detaching the current thread if the guards owns the attachment
    ///
    /// # Safety
    ///
    /// Since this is used as part of the `Drop` implementation then you must
    /// not allow the `Drop` implementation to run if this is called explicitly
    ///
    /// Even though this only takes a reference, the implementation assumes that
    /// the guard is going to be dropped.
    unsafe fn detach_impl(&self) -> Result<()> {
        let level = thread_guard_level_dec();
        assert_eq!(level, self.level, "AttachGuard was dropped out-of-order with respect to other guards");
        if self.should_detach {
            assert_eq!(level, 0, "Spurious AttachGuard that owns its attachment but is nested under another guard");
            unsafe { sys_detach_current_thread(self.env.get_raw(), &std::thread::current()) }
        } else {
            Ok(())
        }
    }

    /// Drop a guard explicitly and detach the current thread if the guard owns
    /// the current attachment.
    ///
    /// Unlike [`AttachGuard::Drop`] this returns a `Result` that can indicate
    /// potential JNI errors from attempting to detach the thread.
    ///
    /// # Panics
    ///
    /// This will panic if a guard is dropped out-of-order, with respect to other
    /// guards. Each `AttachGuard` created may be nested with respected to other
    /// guards and must be dropped or detached in LIFO order.
    pub fn detach(self) -> Result<()> {
        // Safety: we're going to 'forget' the guard afterwards to ensure the
        // `Drop` implementation isn't run too.
        let res = unsafe { self.detach_impl() };

        // We've effectively dropped the guard manually (so we can also get a `Result`)
        // but that means we shouldn't allow the `Drop` implementation to run too.
        std::mem::forget(self);

        res
    }
}

impl Drop for AttachGuard {
    fn drop(&mut self) {
        if let Err(err) = unsafe { self.detach_impl() } {
            // This probably means that something `unsafe` happened to detach the thread already
            log::error!("Failed to detach current JNI thread: {err}");
        }
    }
}

thread_local! {
    static THREAD_ATTACH_GUARD: RefCell<Option<TLSAttachGuard>> = const { RefCell::new(None) }
}

#[derive(Debug)]
struct TLSAttachGuard {
    env: *mut jni_sys::JNIEnv,
    /// A call std::thread::current() function can panic in case the local data has been destroyed
    /// before the thead local variables. The possibility of this happening depends on the platform
    /// implementation of the crate::sys_common::thread_local_dtor::register_dtor_fallback.
    /// The InternalAttachGuard is a thread-local vairable, so capture the thread meta-data
    /// during creation
    thread: Thread,
}

impl TLSAttachGuard {
    /// Detach a thread before the thread terminates **IFF** it was previously attached via
    /// [`JavaVM::attach_current_thread`] **AND** there is no active [`AttachGuard`] in use
    /// for this thread.
    fn detach() -> Result<()> {
        if THREAD_GUARD_NEST_LEVEL.get() != 0 {
            return Err(Error::ThreadAttachmentGuarded);
        }

        THREAD_ATTACH_GUARD.with(move |f| {
            if let Some(guard) = f.borrow_mut().take() {
                // Safety: we use `std::mem::forget` to ensure we don't also
                // run the `Drop` implementation
                let res = unsafe { guard.detach_impl() };
                std::mem::forget(guard);
                res
            } else {
                Ok(())
            }
        })
    }

    unsafe fn attach_current_thread(java_vm: JavaVM) -> Result<AttachGuard> {
        let thread = current();
        let env = sys_attach_current_thread(&java_vm, &thread)?;
        THREAD_ATTACH_GUARD.with(move |f| {
            *f.borrow_mut() = Some(Self {
                env,
                thread: current(),
            });
        });
        Ok(unsafe { AttachGuard::from_unowned(env) })
    }

    /// Detach the current thread after checking there are no active [`AttachGuard`]s
    ///
    /// # Safety
    /// Since this is used in the implementation of `Drop` you must make sure
    /// to not let `Drop` run if this is called explicitly.
    unsafe fn detach_impl(&self) -> Result<()> {
        sys_detach_current_thread(self.env, &self.thread)
    }
}

impl Drop for TLSAttachGuard {
    fn drop(&mut self) {
        if let Err(e) = unsafe { self.detach_impl() } {
            error!(
                "Error detaching current thread: {:#?}\nThread {} id={:?}",
                e,
                self.thread.name().unwrap_or_default(),
                self.thread.id(),
            );
        }
    }
}
