use {objects::JObject, JavaVM, JNIEnv, errors::*};

use std::sync::Arc;

/// The capacity of local frames, allocated for attached threads by default. Same as the default
/// value Hotspot uses when calling native Java methods.
pub const DEFAULT_LOCAL_FRAME_CAPACITY: i32 = 32;

/// Thread attachment manager. Attaches threads as daemons, hence they do not block
/// JVM exit. Finished threads detach automatically.
#[derive(Clone)]
pub struct Executor {
    vm: Arc<JavaVM>,
}

impl Executor {
    /// Creates new Executor with specified JVM.
    pub fn new(vm: Arc<JavaVM>) -> Self {
        Self { vm }
    }

    /// Executes a provided closure, making sure that the current thread
    /// is attached to the JVM. Additionally ensures that local object references are freed after
    /// call.
    ///
    /// Allocates a local frame with the specified capacity.
    pub fn with_attached_capacity<F, R>(&self, capacity: i32, f: F) -> Result<R>
    where
        F: FnOnce(&JNIEnv) -> Result<R>,
    {
        assert!(capacity > 0, "capacity should be a positive integer");

        let jni_env = self.vm.attach_current_thread_as_daemon()?;
        let mut result = None;
        jni_env.with_local_frame(capacity, || {
            result = Some(f(&jni_env));
            Ok(JObject::null())
        })?;

        result.expect("The result should be Some or this line shouldn't be reached")
    }

    /// Executes a provided closure, making sure that the current thread
    /// is attached to the JVM. Additionally ensures that local object references are freed after
    /// call.
    ///
    /// Allocates a local frame with the default capacity
    /// ([`DEFAULT_LOCAL_FRAME_CAPACITY`](constant.DEFAULT_LOCAL_FRAME_CAPACITY.html)).
    pub fn with_attached<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&JNIEnv) -> Result<R>,
    {
        self.with_attached_capacity(DEFAULT_LOCAL_FRAME_CAPACITY, f)
    }
}
