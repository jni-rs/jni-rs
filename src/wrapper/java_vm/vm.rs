use std::{
    cell::{Cell, RefCell},
    os::raw::c_char,
    ptr,
    sync::atomic::{AtomicUsize, Ordering},
    thread::{current, Thread},
};

use log::{debug, error};

use crate::{
    env::Env,
    errors::*,
    objects::{Global, JObject, JObjectRef},
    strings::JNIString,
    sys, JNIVersion,
};

#[cfg(feature = "invocation")]
use {
    crate::InitArgs,
    std::os::raw::c_void,
    std::{ffi::OsStr, path::PathBuf},
};

#[cfg(doc)]
use {crate::env, crate::objects};

/// The capacity of local frames, allocated for attached threads by default. Same as the default
/// value Hotspot uses when calling native Java methods.
pub const DEFAULT_LOCAL_FRAME_CAPACITY: usize = 32;

/// The `jni-rs` crate makes the assumption that it's not possible to create more than one Java VM
/// per-process, or even re-initialize a JavaVM that is "destroyed".
///
/// This allows us to save a global pointer for the JavaVM.
///
/// We also guarantee that if you currently have an [`AttachGuard`] thread attachment (or a `Env`
/// reference), that implies that [`JavaVM::singleton()`] has been initialized and will return a
/// valid [`JavaVM`].
///
/// For example, this guarantee is relied on internally to avoid redundantly saving JavaVM pointers
/// if know we can assume that `JavaVM::singleton()` will return a `JavaVM` when needed.
static JAVA_VM_SINGLETON: once_cell::sync::OnceCell<JavaVM> = once_cell::sync::OnceCell::new();

/// The Java VM API, including (optional) [Invocation API][invocation-api] support.
///
/// An existing JavaVM can be obtained either via [`JavaVM::singleton`], or [`Env::get_java_vm`]
/// in an already attached thread, or a new VM can be [launched](#launching-jvm-from-rust) via
/// [`JavaVM::new`].
///
/// ## Minimum supported JNI version is 1.4
///
/// The implementation of this crate assumes your Java VM supports at least JNI >= 1.4
///
/// The implementation wouldn't be able to call `ExceptionCheck` without JNI 1.2 and requiring >=
/// 1.4 means we don't need any runtime version checks for the direct byte buffer APIs.
///
/// Since `GetVersion` requires a [`Env`] and is not one of the JNI APIs that is safe to use with
/// pending exceptions then the implementation is not always able to explicitly assert the supported
/// version.
///
/// ## Attaching Native Threads
///
/// Your application always needs to explicitly attach `jni-rs` to the current thread before it can
/// access the [`Env`] API (most of the interesting APIs are under [`Env`]).
///
/// If you're implementing a native/foreign method then JNI will pass a thread attachment (in the
/// form of a raw [`sys::JNIEnv`] pointer that should be captured using the
/// [`env::EnvUnowned`] type) and converted into a [`Env`] reference via
/// [`env::EnvUnowned::with_env`])
///
/// Note: [`Env`] is not a `#[transparent]` wrapper over a [`sys::JNIEnv`] pointer.
///
/// The attachment of the current thread is always represented via an [`AttachGuard`] which blocks
/// the thread from being detached and acts as a marker for a JNI stack frame.
///
/// [`Env`] is only ever exposed in the public API by-reference, and will always borrow from an
/// [`AttachGuard`] such that it's lifetime is tied to a fixed JNI stack frame.
///
/// Unless you are using `unsafe` APIs though, the [`AttachGuard`] itself will usually be hidden,
/// and you will get a [`Env`] reference (that borrows from a hidden [`AttachGuard`]).
///
/// This library supports these modes of attachment:
/// * A permanent attachment with [`JavaVM::attach_current_thread`]. The thread will automatically
///   detach itself before it terminates (recommended for attaching in native threads).
/// * A scoped attachment with [`JavaVM::attach_current_thread_for_scope`]. The thread will
///   automatically detach itself once your given closure returns (and the hidden [`AttachGuard`]
///   is dropped).
/// * Implicit ("unowned") attachments, for use in native methods. The thread will never get
///   explicitly detached by `jni-rs` if we have implicit attachment.
///
/// ### Local Reference Management
///
/// Remember that the native thread attached to the VM must manage local references
/// carefully, i.e., do not allocate an excessive number of references and release them promptly when
/// they are no longer needed to enable the GC to collect them.
///
/// A common approach is to push appropriately-sized local frames for larger
/// code fragments (see [`Env::with_local_frame`] or [`JavaVM::with_env`])
/// and [`objects::AutoLocal`] for temporary references in loops.
///
/// See also the [JNI specification][spec-references] for details on referencing Java objects.
///
/// ## Launching JVM from Rust
///
/// To [launch][launch-vm] a JVM from a native process, enable the `invocation` feature in the
/// Cargo.toml:
///
/// ```toml
/// jni = { version = "0.21.1", features = ["invocation"] }
/// ```
///
/// The application will be able to use [`JavaVM::new`] which will dynamically load a `jvm` library
/// (which is distributed with the JVM) at runtime:
///
/// ```rust
/// # use jni::errors;
/// # //
/// # // Ignore this test without invocation feature, so that simple `cargo test` works
/// # #[cfg(feature = "invocation")]
/// # fn main() -> errors::StartJvmResult<()> {
/// # use jni::{AttachGuard, objects::JValue, InitArgsBuilder, Env, JNIVersion, JavaVM, sys::jint};
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
/// // Attach the current thread to call to the JavaVM
/// jvm.attach_current_thread(|env| -> errors::Result<()> {
///     // Call Java Math#abs(-10)
///     let x = JValue::from(-10);
///     let val: jint = env.call_static_method(c"java/lang/Math", c"abs", c"(I)I", &[x])?
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
/// For the operating system to correctly load the `jvm` library it may also be necessary to update
/// the path that the OS uses to find dependencies of the `jvm` library.
/// * On **Windows**, append the path to `$JAVA_HOME/bin` to the `PATH` environment variable.
/// * On **MacOS**, append the path to `libjvm.dylib` to `LD_LIBRARY_PATH` environment variable.
/// * On **Linux**, append the path to `libjvm.so` to `LD_LIBRARY_PATH` environment variable.
///
/// The exact relative path to `jvm` library is version-specific.
///
/// [invocation-api]: https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html
/// [get-vm]: struct.Env.html#method.get_java_vm
/// [launch-vm]: struct.JavaVM.html#method.new
/// [act]: struct.JavaVM.html#method.attach_current_thread
/// [actp]: struct.JavaVM.html#method.attach_current_thread_permanently
/// [spec-references]:
///     https://docs.oracle.com/en/java/javase/12/docs/specs/jni/design.html#referencing-java-objects
/// [java-locator]: https://crates.io/crates/java-locator
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

    /// Get a [`JavaVM`] for the global Java VM
    ///
    /// If no [`JavaVM`] has been initialized, this will return [`Error::UninitializedJavaVM`].
    ///
    /// If a [`JavaVM`] has previously been created, either via [`JavaVM::new()`] or
    /// [`JavaVM::from_raw`] then that [`JavaVM`] will be accessible as a global singleton.
    ///
    /// This is possible because JNI does not support fully destroying a Java VM and then
    /// initializing a new one and so as soon as we have seen a Java VM pointer once, we know it's
    /// the only VM that will ever exist and it will always be valid in safe code.
    ///
    /// If your code observes a [`Env`] reference or an [`AttachGuard`] (from this crate version)
    /// then you can assume that [`JavaVM::singleton()`] has been initialized.
    ///
    /// Beware that the observation of reference types (such as [`crate::objects::JObject`]) only
    /// imply that [`JavaVM::singleton()`] has been initialized if the references are non-null.
    ///
    /// One other caveat is that native methods may capture reference type arguments, such as
    /// [`JObject`], where their lifetime is _not_ tied to a real `Env`. (And so at the start of
    /// a native method, [`JavaVM::singleton()`] may not be initialized even though we can observe
    /// reference types).
    ///
    /// In practice though, you can usually assume [`JavaVM::singleton()`] has been initialized
    /// if you observe non-null reference types, based on the assumption that:
    ///
    /// - Before any other `jni-rs` API is used, a native method is expected to use
    ///   [`env::EnvUnowned::with_env`] to get a `Env` reference, which will initialize
    ///   [`JavaVM::singleton()`].
    /// - For any native method implementation to be safe, it must use `catch_unwind` (e.g. via
    ///   [`env::EnvUnowned::with_env`]) to ensure that panics can't unwind over an FFI boundary
    ///   (at least rendering an early miss-use of `JavaVM::singleton()` "safe").
    ///
    /// Note: that other versions of `jni-rs` within the same application aren't able to share this
    /// singleton state. So you should not make assumptions about this being initialized as a side
    /// effect of other dependencies using `jni-rs` (unless you are using a re-exported version of
    /// `jni-rs` from that dependency). For example the `android-activity` crate will initialize a
    /// [JavaVM] before `android_main()` is called, but unless you are using the same version of
    /// `jni-rs` as `android-activity` you can't immediately assume there is a [JavaVM] singleton.
    pub fn singleton() -> Result<Self> {
        JAVA_VM_SINGLETON
            .get()
            .cloned()
            .ok_or(Error::UninitializedJavaVM)
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
        // Don't use .get_or_try_init() around all this code because `Self::with_create_fn_ptr`
        // will call `JavaVM::from_raw` which will also try and set JAVA_VM_SINGLETON and create
        // a deadlock
        if let Some(jvm) = JAVA_VM_SINGLETON.get() {
            Ok(jvm.clone())
        } else {
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
                let create_fn = libjvm.get(b"JNI_CreateJavaVM\0").map_err(|error| {
                    StartJvmError::LoadError(libjvm_path_string.to_owned(), error)
                })?;

                // Create the JVM.
                Self::with_create_fn_ptr(args, *create_fn).map_err(StartJvmError::Create)
            };

            if result.is_ok() {
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
            }

            result
        }
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

        let vm = Self::from_raw(ptr);

        // JNI_CreateJavaVM will implicitly attach the calling thread to the JVM.
        //
        // Since the JVM may attribute this thread with special significance as a "main" thread, we
        // avoid detaching it.
        //
        // Instead we take ownership of that attachment by creating a `TLSAttachGuard` for it.
        //
        // Note: This will make a redundant `AttachCurrentThread` call via
        // `sys_attach_current_thread` and the `Default` `config` is benign here because it will be
        // ignored while the thread is already attached.
        //
        // This will track the new attachment in TLS, under `THREAD_ATTACH_GUARD`
        TLSAttachGuard::attach_current_thread(&vm, &Default::default())?;

        Ok(vm)
    }

    /// Create a JavaVM from a raw pointer.
    ///
    /// # Safety
    ///
    /// Expects a valid, non-null JavaVM pointer that supports JNI version >= 1.4.
    ///
    /// Only does a `null` check.
    pub unsafe fn from_raw(ptr: *mut sys::JavaVM) -> Self {
        assert!(!ptr.is_null());
        JAVA_VM_SINGLETON.get_or_init(|| JavaVM(ptr)).clone()
    }

    /// Returns underlying [`sys::JavaVM`] interface.
    pub fn get_raw(&self) -> *mut sys::JavaVM {
        self.0
    }

    pub(crate) fn from_env(env: &Env) -> Self {
        // Don't use `.get_or_init()` here because it would deadlock if calling `JavaVM::from_raw`
        // which also uses `.get_or_init()`
        if let Some(jvm) = JAVA_VM_SINGLETON.get() {
            jvm.clone()
        } else {
            let mut raw = ptr::null_mut();
            let res = unsafe { jni_call_unchecked!(env, v1_1, GetJavaVM, &mut raw) };
            let res = jni_error_code_to_result(res);

            // If we have a Env reference then we can assume we have a valid, non-null Env
            // pointer and there should be no reason for GetJavaVM to fail.
            //
            // If it would fail, we assume that would be breaking fundamental invariants we
            // rely on within jni-rs so we wouldn't consider it safe in any case.
            res.expect("Spurious failure to get JavaVM from Env");

            // Safety: The pointer from GetJavaVM should be valid
            unsafe { JavaVM::from_raw(raw) }
        }
    }

    /// Attaches the current thread to the Java VM and calls the provided callback with a mutable
    /// [`Env`] reference.
    ///
    /// If the thread was not already attached then a new attachment is made which will be
    /// automatically detached when the current thread terminates.
    ///
    /// Calling this in a thread that is already attached is cheap since it will only need to check
    /// thread local storage without making a JNI call.
    ///
    /// This API requests to permanently attach the current thread but since pre-existing
    /// attachments aren't affected by this API, it should not be assumed that the thread will
    /// definitely remain attached until it exits - that is only a request. For example if something
    /// higher on the stack has created a scoped attachment then that will take precedence and the
    /// thread will not be permanently attached.
    ///
    /// You can safely assume that the thread will remain attached for the duration of the callback.
    ///
    /// If you're not sure whether to use [`Self::attach_current_thread`] or
    /// [`Self::attach_current_thread_for_scope`], then you should probably use this API because it
    /// increases the chance that future attachment calls will be cheap.
    ///
    /// # One mutable Env per scope rule
    ///
    /// Remember:
    /// - Each [Env] has a lifetime that ties it to a JNI stack frame that holds local
    ///   references.
    /// - A mutable [Env] allows you to create new local references that would have a lifetime
    ///   for the associated stack frame.
    /// - Since new local references can only be created in the top JNI stack frame; you should only
    ///   ever have one mutable [Env] in scope at a time, which is associated with the top JNI
    ///   stack frame.
    ///
    /// Although the `jni-rs` API tries to constrain access to mutable [Env] references through
    /// the borrow checker, at compile time, a Java VM is a global resource that can be accessed in
    /// a myriad of ways that Rust can't limit.
    ///
    /// `jni-rs` also cannot differentiate (at runtime or compile time) whether some scope already
    /// has access to a mutable [Env] because `jni-rs` may be used across multiple orthogonal
    /// crates that may work at different levels of the stack.
    ///
    /// So, in addition to the compile-time borrow checker, `jni-rs` also implements runtime checks
    /// that will `panic` if you try to create a new local reference with a mutable [Env] that is
    /// not associated with the top JNI stack frame.
    ///
    /// To avoid being exposed to runtime borrow checking for the JNI stack frames, this API should
    /// never be used to materialize access to a mutable [Env] if you already have a mutable
    /// [Env] in scope.
    ///
    /// As long as you only ever have one mutable [Env] then you can rely on the compile-time
    /// borrow checker making sure you can only create new local references that are associated with
    /// the top of the JNI stack.
    ///
    /// This is an important safety feature for `jni-rs` because it would be unsound to allow local
    /// references to be created at the top of the JNI stack but have a Rust lifetime associated
    /// with the middle of the stack.
    pub fn attach_current_thread<F, T, E>(&self, callback: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        self.attach_current_thread_with_config(
            AttachConfig::default,
            Some(DEFAULT_LOCAL_FRAME_CAPACITY),
            callback,
        )
    }

    /// Attaches the current thread to the Java VM and calls the provided callback with a mutable
    /// [`Env`] reference.
    ///
    /// If the thread was not already attached, the thread will be detached when the callback
    /// returns.
    ///
    /// Calling this in a thread that is already attached is cheap since it will only need to check
    /// thread local storage without making a JNI call.
    ///
    /// Attaching a thread is an expensive operation if it was not already attached, so it's
    /// generally recommended that you should use [`Self::attach_current_thread()`] (requesting to
    /// attach the thread permanently) instead of using a scoped attachment. Using this API may
    /// increase the chance that you incur the cost of repeatedly attaching and detaching the same
    /// thread.
    ///
    /// # One mutable Env per scope rule
    ///
    /// See [`JavaVM::attach_current_thread`](#one-mutable-jnienv-per-scope-rule) for details.
    pub fn attach_current_thread_for_scope<F, T, E>(&self, callback: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        self.attach_current_thread_with_config(
            || AttachConfig::new().scoped(true),
            Some(DEFAULT_LOCAL_FRAME_CAPACITY),
            callback,
        )
    }

    /// Attaches the current thread to the Java VM and calls the provided callback with a mutable
    /// [`Env`] reference.
    ///
    /// This function allows you to customize the attachment process and choose whether to create
    /// a new local frame (with a given capacity) or use the current one.
    ///
    /// Most of the time you should prefer to use [`Self::attach_current_thread()`] or
    /// [`Self::attach_current_thread_for_scope()`] instead of this function.
    ///
    /// The semantics of [`Self::attach_current_thread`] are equivalent to:
    /// ```rust
    /// # use jni::{JavaVM, AttachConfig, DEFAULT_LOCAL_FRAME_CAPACITY};
    /// # fn jni_example(vm: &JavaVM) -> jni::errors::Result<()> {
    ///      vm.attach_current_thread_with_config(AttachConfig::default, Some(DEFAULT_LOCAL_FRAME_CAPACITY), |env| {
    ///          // Use the Env reference
    ///          Ok(())
    ///      })
    /// # }
    /// ```
    ///
    /// The semantics of [`Self::attach_current_thread_for_scope`] are equivalent to:
    /// ```rust
    /// # use jni::{JavaVM, AttachConfig, DEFAULT_LOCAL_FRAME_CAPACITY};
    /// # fn jni_example(vm: &JavaVM) -> jni::errors::Result<()> {
    ///      vm.attach_current_thread_with_config(|| AttachConfig::default().scoped(true), Some(DEFAULT_LOCAL_FRAME_CAPACITY), |env| {
    ///          // Use the Env reference
    ///          Ok(())
    ///      })
    /// # }
    /// ```
    ///
    /// See [`Self::attach_current_thread`], [`Self::attach_current_thread_for_scope`] and [`AttachConfig`] for more details.
    ///
    /// # One mutable Env per scope rule
    ///
    /// See [`JavaVM::attach_current_thread`](#one-mutable-jnienv-per-scope-rule) for details.
    pub fn attach_current_thread_with_config<'config, F, C, T, E>(
        &self,
        config: C,
        capacity: Option<usize>,
        callback: F,
    ) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
        C: FnOnce() -> AttachConfig<'config>,
    {
        // Safety: The guard will remain fixed on the stack by keeping the guard
        // private to this function.
        let mut guard = unsafe { self.attach_current_thread_guard(config)? };
        if let Some(capacity) = capacity {
            guard.with_env(capacity, callback)
        } else {
            guard.with_env_current_frame(callback)
        }
    }

    /// Attaches the current thread to the Java VM and returns an [`AttachGuard`] for the
    /// attachment.
    ///
    /// This is a low-level (unsafe) building block for [`Self::attach_current_thread`],
    /// [`Self::attach_current_thread_for_scope`] and [`Self::attach_current_thread_with_config`]
    /// that allows for more fine-grained control over the attachment process and how you borrow an
    /// `Env` reference from the guard.
    ///
    /// The given `config` callback is only lazily called if the thread was not already attached and
    /// returns a [`AttachConfig`] that you can use to customize the attachment.
    ///
    /// For example, this can be used to implement your own equivalent to
    /// [`Self::attach_current_thread`] like:
    ///
    /// ```rust
    /// # use jni::{JavaVM, AttachConfig, DEFAULT_LOCAL_FRAME_CAPACITY};
    /// struct Executor {
    ///     vm: JavaVM,
    /// }
    /// impl Executor {
    ///     fn new(vm: JavaVM) -> Self {
    ///         Self { vm }
    ///     }
    ///
    ///     pub fn my_attach_current_thread<F, T, E>(&self, callback: F) -> std::result::Result<T, E>
    ///     where
    ///         F: FnOnce(&mut jni::Env) -> std::result::Result<T, E>,
    ///         E: From<jni::errors::Error>,
    ///     {
    ///         // Safety: The guard will remain fixed on the stack by keeping the guard
    ///         // private to this function.
    ///         let mut guard = unsafe { self.vm.attach_current_thread_guard(AttachConfig::default)? };
    ///         guard.with_env(DEFAULT_LOCAL_FRAME_CAPACITY, callback)
    ///     }
    /// }
    /// ```
    ///
    /// See [`Self::attach_current_thread`], [`Self::attach_current_thread_for_scope`],
    /// [`AttachConfig`] and [`AttachGuard`] for more details.
    ///
    /// Consider using [`Self::attach_current_thread_with_config`] before resorting to this (unsafe) API.
    ///
    /// # Safety
    ///
    /// See the 'Safety' rules for [`AttachGuard`]
    ///
    /// The main concern is that you must keep the [`AttachGuard`] on the stack and it must not
    /// move. This can be achieved by hiding it from safe code, only exposing a temporary [`Env`]
    /// reference that borrows from the guard (as in the example above).
    ///
    pub unsafe fn attach_current_thread_guard<'config, F>(&self, config: F) -> Result<AttachGuard>
    where
        F: FnOnce() -> AttachConfig<'config>,
    {
        // Safety:
        // - The minimum supported JNI version is >= 1.4
        // - The caller is responsible for managing the returned guard safely
        let guard = unsafe {
            if let Some(guard) = Self::tls_get_env_attachment() {
                guard
            } else {
                match self.sys_get_env_attachment() {
                    Ok(guard) => guard,
                    Err(Error::JniCall(JniError::ThreadDetached)) => {
                        let config = config();
                        if config.scoped {
                            let jni = sys_attach_current_thread(self, &config, &current())?;
                            AttachGuard::from_owned(jni)
                        } else {
                            TLSAttachGuard::attach_current_thread(self, &config)?
                        }
                    }
                    Err(err) => Err(err)?,
                }
            }
        };

        Ok(guard)
    }

    /// Explicitly detaches the current thread from the JVM, **IFF** it was previously attached
    /// using [`JavaVM::attach_current_thread`] **AND** if there is no [`AttachGuard`] also keeping
    /// the current thread attached (I.e. you have no [`Env`] reference in scope).
    ///
    /// This will always return an error if there are currently any active [`AttachGuard`]s
    /// (detaching the thread in this case would effectively turn guards into invalid, dangling
    /// pointers).
    ///
    /// Detaching a non-attached thread is a no-op that won't return an error (assuming there's no
    /// active [`AttachGuard`] as noted above).
    ///
    /// This API has no effect on scoped attachments that were created via
    /// [`JavaVM::attach_current_thread_for_scope`]. Or in other words it's not applicable to scoped
    /// attachments because it's an error to call while there are active [`AttachGuard`]s.
    ///
    /// _**Note**: It's _rarely_ necessary to use this API because a thread that is attached
    /// via [`JavaVM::attach_current_thread`] will automatically detach when that thread terminates.
    ///
    /// Explicitly detaching the thread could lead to overheads later if the same thread needs to
    /// get re-attached.
    ///
    /// If there is a need to detach a thread before it terminates, then it's possible that a scoped
    /// attachment via [`JavaVM::attach_current_thread_for_scope`] could be used so that the
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
    pub fn thread_attach_guard_level() -> usize {
        THREAD_GUARD_NEST_LEVEL.get()
    }

    /// Get an [`AttachGuard`] for the [`Env`] associated with the current thread or, if JNI is
    /// not attached to the Java VM, this will return [`Error::JniCall()`] with
    /// [`JniError::ThreadDetached`].
    ///
    /// Note: jni-rs is implemented based on an assumption that all real-world implementations of
    /// JNI `GetEnv` will return the same pointer for any given version (so long as the version is
    /// supported).
    ///
    /// Hypothetically the JNI spec allows for the possibility for an implementation to return a
    /// different Env pointer that nulls out functions that aren't valid for that version (or
    /// dispatches calls differently).
    ///
    /// If we ever find a JVM implementation that in fact returns a different pointer then we could
    /// just repeat the GetEnv call with the maximum supported version after querying the version.
    ///
    /// # Safety
    ///
    /// You must know that the [`JavaVM`] supports at least JNI >= 1.2 (we require >= 1.4 but
    /// we couldn't even call GetEnv without 1.2)
    ///
    /// See the 'Safety' rules for [`AttachGuard`]
    pub(crate) unsafe fn sys_get_env_attachment(&self) -> Result<AttachGuard> {
        unsafe {
            let mut ptr = ptr::null_mut();
            let res =
                java_vm_call_unchecked!(self, v1_2, GetEnv, &mut ptr, JNIVersion::V1_4.into());
            jni_error_code_to_result(res)?;
            let jni = ptr as *mut sys::JNIEnv;
            Ok(AttachGuard::from_unowned(jni))
        }
    }

    /// Returns `true` if the current thread is attached to a Java VM.
    ///
    /// Since this calls [`sys::JNIInvokeInterface__1_2::GetEnv`], it will also recognize thread
    /// attachments that made without using this crate (such as other JNI language bindings).
    pub fn is_thread_attached(&self) -> Result<bool> {
        // Safety: we aren't materializing an attachment guard while we already have access to one
        unsafe {
            self.sys_get_env_attachment()
                .map(|_| true)
                .or_else(|jni_err| match jni_err {
                    Error::JniCall(JniError::ThreadDetached) => Ok(false),
                    _ => Err(jni_err),
                })
        }
    }

    /// Returns an [`AttachGuard`] for the [`Env`] associated with the current thread or, `None`
    /// if JNI is not attached to the Java VM.
    ///
    /// This serves a similar purpose to [`sys::JNIInvokeInterface__1_2::GetEnv`] in that it
    /// provides access to the current thread's JNI environment if JNI is attached.
    ///
    /// This API will only recognize attachments made by this crate (including uses of
    /// [`AttachGuard::from_unowned`]). I.e. the implementation only checks crate-specific thread
    /// local storage and will not actually call [`sys::JNIInvokeInterface__1_2::GetEnv`].
    ///
    /// Consider using [`Self::with_env`] or [`Self::with_env_current_frame`] as safe alternatives
    /// for running code against the currently attached JNI environment.
    ///
    /// # Safety
    ///
    /// See the 'Safety' rules for [`AttachGuard`]
    pub(crate) unsafe fn tls_get_env_attachment() -> Option<AttachGuard> {
        let env_ptr = THREAD_ATTACHMENT.get();
        if env_ptr.is_null() {
            None
        } else {
            // Safety: we can assume any THREAD_ATTACHMENT pointer is valid
            unsafe { Some(AttachGuard::from_unowned(env_ptr)) }
        }
    }

    /// Returns an [`AttachGuard`] for the [`Env`] associated with the current thread.
    ///
    /// If JNI is not attached to the Java VM, this will return [`Error::JniCall()`] with
    /// [`JniError::ThreadDetached`].
    ///
    /// This serves a similar purpose to [`sys::JNIInvokeInterface__1_2::GetEnv`] in that it
    /// provides access to the current thread's JNI environment if JNI is attached.
    ///
    /// This API can recognize attachments made by other JNI language bindings but will first check
    /// crate-specific thread local storage for an attachment before calling
    /// [`sys::JNIInvokeInterface__1_2::GetEnv`].
    ///
    /// Consider using [`Self::with_env`] or [`Self::with_env_current_frame`] as safe alternatives
    /// for running code against the currently attached JNI environment.
    ///
    /// # Safety
    ///
    /// See the 'Safety' rules for [`AttachGuard`]
    pub unsafe fn get_env_attachment(&self) -> Result<AttachGuard> {
        match Self::tls_get_env_attachment() {
            Some(guard) => Ok(guard),
            None => self.sys_get_env_attachment(),
        }
    }

    /// Runs a closure with a borrowed [`Env`] associated with a new JNI stack frame that will be
    /// unwound to release all local references created within the given closure.
    ///
    /// If JNI is not attached to the Java VM, this will return [`Error::JniCall()`] with
    /// [`JniError::ThreadDetached`].
    ///
    /// This API can recognize attachments made by other JNI language bindings but will first check
    /// crate-specific thread local storage for an attachment before calling
    /// [`sys::JNIInvokeInterface__1_2::GetEnv`].
    ///
    /// Internally this calls [`sys::JNINativeInterface__1_2::PushLocalFrame`] with the given
    /// `capacity`, to create a new JNI stack frame, and calls
    /// [`sys::JNINativeInterface__1_2::PopLocalFrame`] after the closure is executed.
    ///
    /// # One mutable Env per scope rule
    ///
    /// See [`JavaVM::attach_current_thread`](#one-mutable-jnienv-per-scope-rule) for details.
    pub fn with_env_with_capacity<F, T, E>(
        &self,
        capacity: usize,
        f: F,
    ) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        unsafe {
            let mut guard = self.get_env_attachment()?;
            guard.with_env(capacity, f)
        }
    }

    /// Runs a closure with a borrowed [`Env`] associated with a new JNI stack frame that will be
    /// unwound to release all local references created within the given closure.
    ///
    /// If JNI is not attached to the Java VM, this will return [`Error::JniCall()`] with
    /// [`JniError::ThreadDetached`].
    ///
    /// This API can recognize attachments made by other JNI language bindings but will first check
    /// crate-specific thread local storage for an attachment before calling
    /// [`sys::JNIInvokeInterface__1_2::GetEnv`].
    ///
    /// Internally this calls [`sys::JNINativeInterface__1_2::PushLocalFrame`] with a capacity
    /// of [`DEFAULT_LOCAL_FRAME_CAPACITY`] to create a new JNI stack frame and calls
    /// [`sys::JNINativeInterface__1_2::PopLocalFrame`] after the closure is executed.
    ///
    /// # One mutable Env per scope rule
    ///
    /// See [`JavaVM::attach_current_thread`](#one-mutable-jnienv-per-scope-rule) for details.
    pub fn with_env<F, T, E>(&self, f: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        self.with_env_with_capacity(DEFAULT_LOCAL_FRAME_CAPACITY, f)
    }

    /// Runs a closure with a borrowed [`Env`] associated with the current (top) JNI stack frame.
    ///
    /// Unlike [`Self::with_env()`], this API does not push a new JNI stack frame.
    ///
    /// Most of the time this API should probably be avoided (see [`Self::with_env`] instead) unless
    /// you're sure your code won't leak local references into the current stack frame (or you're
    /// sure that the leaked references are acceptable because you know when the frame will unwind
    /// and release those references).
    ///
    /// This may have a slightly lower overhead than [`Self::with_env()`] (since it doesn't need to
    /// push/pop a JNI stack frame), but the trade off is that you may leak local references into
    /// the current stack frame if you don't delete them. Deleting local references individually is
    /// likely to have a higher cost than pushing/popping a JNI stack frame.
    pub fn with_env_current_frame<F, T, E>(&self, f: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        unsafe {
            let mut guard = self.get_env_attachment()?;
            guard.with_env_current_frame(f)
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
    /// not wait for attached daemon threads to exit, this also means that if
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
    /// called to avoid undefined behaviour.
    ///
    /// Here is an non-exhaustive list of auto-release types to consider:
    /// - `AttachGuard`
    /// - `AutoElements`
    /// - `AutoElementsCritical`
    /// - `AutoLocal`
    /// - `Global`
    /// - `MUTF8Chars`
    /// - `JMap`
    /// - `Weak`
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

/// Configuration options for attaching the current thread to a Java VM.
#[derive(Default)]
pub struct AttachConfig<'a> {
    scoped: bool,
    name: Option<JNIString>,
    group: Option<&'a Global<JObject<'static>>>,
}

impl<'a> AttachConfig<'a> {
    /// Creates a new `AttachConfig` with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether the attachment should be owned by the current scope, such
    /// that the thread will be automatically detached when the attachment guard
    /// is dropped.
    ///
    /// The default is `false`, so the thread will be attached permanently.
    ///
    /// It is normally best to attach permanently because it can reduce the cost
    /// of repeatedly attaching and detaching threads.
    pub fn scoped(mut self, scoped: bool) -> Self {
        self.scoped = scoped;
        self
    }

    /// Sets the name of the thread as seen by the JVM and operating system.
    pub fn name<S: AsRef<str>>(mut self, name: S) -> Self {
        self.name = Some(JNIString::from(name.as_ref()));
        self
    }

    /// Specifies a global reference to a `ThreadGroup` that the thread should
    /// be associated with.
    pub fn group(mut self, group: &'a Global<JObject<'static>>) -> Self {
        self.group = Some(group);
        self
    }
}

static ATTACHED_THREADS: AtomicUsize = AtomicUsize::new(0);

unsafe fn sys_attach_current_thread(
    vm: &JavaVM,
    config: &AttachConfig,
    thread: &Thread,
) -> Result<*mut sys::JNIEnv> {
    let mut env_ptr = ptr::null_mut();
    let mut args = sys::JavaVMAttachArgs {
        version: JNIVersion::V1_4.into(),
        name: config
            .name
            .as_ref()
            .map(|s| s.as_ptr() as *mut c_char)
            .unwrap_or(ptr::null_mut()),
        group: config
            .group
            .as_ref()
            .map(|g| g.as_raw())
            .unwrap_or(ptr::null_mut()),
    };
    let res = java_vm_call_unchecked!(
        vm,
        v1_1,
        AttachCurrentThread,
        &mut env_ptr,
        &mut args as *mut sys::JavaVMAttachArgs as *mut core::ffi::c_void
    );
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

/// Detach a thread, asserting that we own the current attachment and have a valid `Env` pointer
///
/// Although `DetachCurrentThread` is part of the `JavaVM` "invocation" API and doesn't require a
/// `Env` pointer, we want to constrain this code to only ever detach threads if we own the
/// current attachment.
unsafe fn sys_detach_current_thread(env_ptr: *mut jni_sys::JNIEnv, thread: &Thread) -> Result<()> {
    assert_eq!(JavaVM::thread_attach_guard_level(), 0);

    unsafe {
        let mut guard = AttachGuard::from_unowned(env_ptr);
        let env = Env::new(&mut guard);
        let vm = env.get_java_vm();

        let vm_get_env_guard = vm.sys_get_env_attachment()?;

        // Maybe just assert_eq!() ?
        if vm_get_env_guard.env != env_ptr {
            return Err(Error::JniCall(JniError::InvalidArguments));
        }

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
    static THREAD_ATTACHMENT: Cell<*mut jni_sys::JNIEnv> = const { Cell::new(std::ptr::null_mut()) };
    static THREAD_GUARD_NEST_LEVEL: Cell<usize> = const { Cell::new(0) };
}

/// Represents a JNI attachment of the current thread to a Java VM, which is
/// required before you can access the [`Env`] API.
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
/// # JavaVM::singleton() guarantee
///
/// If you know that at least one [`AttachGuard`] has ever existed (which is
/// implied if you have a `Env` reference or any non-null reference type
/// (like `JObject` or `Global`) you can assume that [`JavaVM::singleton()`]
/// will return `Some(JavaVM)`.
///
/// You can use this to avoid redundantly copying JavaVM pointers that can
/// instead be read via [`JavaVM::singleton()`].
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
///    safe way of accessing a mutable [`Env`].
///
///    It _is_ OK to create a redundant [`AttachGuard`] in case there may
///    already be a guard for an attachment lower on the stack (owned by some
///    function that has called you) but it's not safe if the code in your
///    current scope can directly access a pre-existing guard or mutable
///    [`Env`].
///
/// 2. You must treat a guard as an immovable type that needs to live on the
///    stack and can't be given a `'static` lifetime (e.g. by boxing or moving
///    into a `static` variable) or re-ordered relative to other guards on the
///    stack.
///
///    When a guard is borrowed to access a [`Env`] reference, it would not
///    be safe if you could give yourself access to a `'static` `Env`
///    reference, because the lifetime associated with a `Env` is used to
///    associate JNI local references with a JNI stack frame.
///
/// # Panics
///
///    The `Drop` implementation will `panic` if a guard is not dropped in the
///    same order that it was created, relative to other guards (LIFO order).
#[derive(Debug)]
pub struct AttachGuard {
    pub(crate) env: *mut crate::sys::JNIEnv,
    should_detach: bool,
    pub(crate) level: usize,
}

fn thread_guard_level_push(env: *mut jni_sys::JNIEnv) -> usize {
    THREAD_GUARD_NEST_LEVEL.with(|cell| {
        let level = cell.get();
        if level == 0 {
            THREAD_ATTACHMENT.set(env);
        }
        cell.set(level + 1);
        level
    })
}

fn thread_guard_level_pop() -> usize {
    let level = THREAD_GUARD_NEST_LEVEL.with(|cell| {
        let level = cell.get();
        assert_ne!(
            level, 0,
            "Spuriously dropped more AttachGuards than were known to exist"
        );
        cell.set(level - 1);
        level - 1
    });

    if level == 0 {
        THREAD_ATTACHMENT.set(std::ptr::null_mut());
    }

    level
}

impl AttachGuard {
    /// Wrap a raw [`sys::JNIEnv`] pointer in an `AttachGuard` that will detach
    /// the current thread on drop.
    ///
    /// An owned `AttachGuard` can be used to implement "for_scope" thread
    /// attachments.
    ///
    /// # Safety
    ///
    /// The pointer must be non-null and correspond to a valid [`Env`]
    /// pointer that is attached to the current thread.
    ///
    /// The returned guard must be managed according to the general
    /// [`AttachGuard`] "Safety" rules. The guard should be treated as an
    /// immovable value on the stack that represents a handle for the
    /// currently-top JNI stack frame.
    unsafe fn from_owned(env: *mut sys::JNIEnv) -> Self {
        let level = thread_guard_level_push(env);

        let mut guard = Self {
            env,
            should_detach: true,
            level,
        };

        unsafe {
            let env = Env::new(&mut guard);
            // Guarantee that if you have an `AttachGuard` then
            // `JavaVM::singleton()` will always return `Some(JavaVM)`
            let _vm = env.get_java_vm();
        }

        guard
    }

    /// Wrap a raw [`sys::JNIEnv`] pointer in an [`AttachGuard`] that does not
    /// own the underlying thread attachment and so it will **NOT** detach the
    /// current thread on drop.
    ///
    /// This can be use when implementing native JNI methods (that are passed an
    /// attached [`sys::JNIEnv`] pointer) as a way to access the [`Env`] API.
    ///
    /// # Safety
    ///
    /// The pointer must be non-null and correspond to a valid [`Env`]
    /// pointer that is attached to the current thread.
    ///
    /// The returned guard must be managed according to the general
    /// [`AttachGuard`] "Safety" rules. The guard should be treated as an
    /// immovable value on the stack that represents a handle for the
    /// currently-top JNI stack frame.
    pub unsafe fn from_unowned(env: *mut sys::JNIEnv) -> Self {
        let level = thread_guard_level_push(env);
        let mut guard = Self {
            env,
            should_detach: false,
            level,
        };

        unsafe {
            let env = Env::new(&mut guard);
            // Guarantee that if you have an `AttachGuard` then `JavaVM::singleton()` will
            // always return `Some(JavaVM)`
            let _vm = env.get_java_vm();
        }

        guard
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

    /// Runs a closure with a borrowed [`Env`] associated with a new JNI stack
    /// frame that will be unwound to release all local references created within
    /// the given closure.
    ///
    /// # Panic
    ///
    /// This will panic if the `AttachGuard` does not currently represent the
    /// top JNI stack frame.
    pub fn with_env<F, T, E>(&mut self, capacity: usize, f: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        // Assuming that the application doesn't break the safety rules for
        // keeping the `AttachGuard` on the stack, and not re-ordering them,
        // we can assert that we will only ever borrow from the top-most
        // guard on the stack
        assert_eq!(THREAD_GUARD_NEST_LEVEL.get(), self.level + 1);
        let mut env = unsafe { Env::new(self) };
        env.with_local_frame(capacity, |jni_env| f(jni_env))
    }

    /// Runs a closure with a borrowed [`Env`] associated with a new JNI stack
    /// frame that will be unwound to release all local references created within
    /// the given closure, except for a single return value reference.
    ///
    /// # Panic
    ///
    /// This will panic if the `AttachGuard` does not currently represent the
    /// top JNI stack frame.
    pub fn with_env_returning_local<'local, F, T, E>(
        &'local mut self,
        capacity: usize,
        f: F,
    ) -> std::result::Result<T::Kind<'local>, E>
    where
        F: for<'new_local> FnOnce(
            &mut Env<'new_local>,
        ) -> std::result::Result<T::Kind<'new_local>, E>,
        T: JObjectRef,
        E: From<Error>,
    {
        // Assuming that the application doesn't break the safety rules for
        // keeping the `AttachGuard` on the stack, and not re-ordering them,
        // we can assert that we will only ever borrow from the top-most
        // guard on the stack
        assert_eq!(THREAD_GUARD_NEST_LEVEL.get(), self.level + 1);
        let mut env = unsafe { Env::new(self) };
        env.with_local_frame_returning_local::<_, T, E>(capacity, |jni_env| f(jni_env))
    }

    /// Runs a closure with a borrowed [`Env`] associated with the guard's
    /// JNI stack frame (which must currently be the top stack frame)
    ///
    /// Most of the time this API should probably be avoided (see
    /// [`Self::with_env`] instead) unless you're sure your code won't leak
    /// local references into the current stack frame.
    ///
    /// This may have a slightly lower overhead than [`Self::with_env()`] (since
    /// it doesn't need to push/pop a JNI stack frame), but the trade off is
    /// that you may leak local references into the current stack frame if you
    /// don't delete them. Deleting local references individually is likely
    /// to have a higher cost that pushing/popping a JNI stack frame.
    ///
    /// # Panic
    ///
    /// This will panic if the `AttachGuard` does not currently represent the
    /// top JNI stack frame.
    pub fn with_env_current_frame<F, T, E>(&mut self, f: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        // Assuming that the application doesn't break the safety rules for
        // keeping the `AttachGuard` on the stack, and not re-ordering them,
        // we can assert that we will only ever borrow from the top-most
        // guard on the stack
        assert_eq!(THREAD_GUARD_NEST_LEVEL.get(), self.level + 1);

        // Assuming that the application doesn't break the safety rules for
        // keeping the `AttachGuard` on the stack, we know we won't create
        // a 'static Env here.
        let mut env = unsafe { Env::new(self) };
        f(&mut env)
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
        let level = thread_guard_level_pop();
        assert_eq!(
            level, self.level,
            "AttachGuard was dropped out-of-order with respect to other guards"
        );
        if self.should_detach {
            assert_eq!(
                level, 0,
                "Spurious AttachGuard that owns its attachment but is nested under another guard"
            );
            unsafe { sys_detach_current_thread(self.env, &std::thread::current()) }
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
    /// implementation of the sys_common::thread_local_dtor::register_dtor_fallback.
    ///
    /// Since this struct will be saved as a thread-local variable, we capture the thread meta-data
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

    unsafe fn attach_current_thread(
        java_vm: &JavaVM,
        config: &AttachConfig,
    ) -> Result<AttachGuard> {
        let thread = current();
        let env = sys_attach_current_thread(java_vm, config, &thread)?;
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
