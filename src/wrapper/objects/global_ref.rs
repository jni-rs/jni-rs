use std::{convert::From, sync::Arc};

use log::{debug, warn};

use crate::{errors::Result, objects::JObject, sys, JNIEnv, JavaVM};

/// A global JVM reference. These are "pinned" by the garbage collector and are
/// guaranteed to not get collected until released. Thus, this is allowed to
/// outlive the `JNIEnv` that it came from and can be used in other threads.
///
/// `GlobalRef` can be cloned to use _the same_ global reference in different
/// contexts. If you want to create yet another global ref to the same java object
/// you may call `JNIEnv#new_global_ref` just like you do when create `GlobalRef`
/// from a local reference.
///
/// Underlying global reference will be dropped, when the last instance
/// of `GlobalRef` leaves its scope.
///
/// It is _recommended_ that a native thread that drops the global reference is attached
/// to the Java thread (i.e., has an instance of `JNIEnv`). If the native thread is *not* attached,
/// the `GlobalRef#drop` will print a warning and implicitly `attach` and `detach` it, which
/// significantly affects performance.

#[derive(Clone)]
pub struct GlobalRef {
    inner: Arc<GlobalRefGuard>,
}

struct GlobalRefGuard {
    obj: JObject<'static>,
    vm: JavaVM,
}

unsafe impl Send for GlobalRef {}
unsafe impl Sync for GlobalRef {}

impl<'a> From<&'a GlobalRef> for JObject<'a> {
    fn from(other: &'a GlobalRef) -> JObject<'a> {
        other.as_obj()
    }
}

impl GlobalRef {
    /// Creates a new wrapper for a global reference.
    ///
    /// # Safety
    ///
    /// Expects a valid raw global reference that should be created with `NewGlobalRef` JNI function.
    pub(crate) unsafe fn from_raw(vm: JavaVM, raw_global_ref: sys::jobject) -> Self {
        GlobalRef {
            inner: Arc::new(GlobalRefGuard::from_raw(vm, raw_global_ref)),
        }
    }

    /// Get the object from the global ref
    ///
    /// This borrows the ref and prevents it from being dropped as long as the
    /// JObject sticks around.
    pub fn as_obj(&self) -> JObject {
        self.inner.as_obj()
    }
}

impl GlobalRefGuard {
    /// Creates a new global reference guard. This assumes that `NewGlobalRef`
    /// has already been called.
    unsafe fn from_raw(vm: JavaVM, obj: sys::jobject) -> Self {
        GlobalRefGuard {
            obj: JObject::from(obj),
            vm,
        }
    }

    /// Get the object from the global ref
    ///
    /// This borrows the ref and prevents it from being dropped as long as the
    /// JObject sticks around.
    pub fn as_obj(&self) -> JObject {
        self.obj
    }
}

impl Drop for GlobalRefGuard {
    fn drop(&mut self) {
        fn drop_impl(env: &JNIEnv, global_ref: JObject) -> Result<()> {
            let internal = env.get_native_interface();
            // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
            jni_unchecked!(internal, DeleteGlobalRef, global_ref.into_inner());
            Ok(())
        }

        let res = match self.vm.get_env() {
            Ok(env) => drop_impl(&env, self.as_obj()),
            Err(_) => {
                warn!("Dropping a GlobalRef in a detached thread. Fix your code if this message appears frequently (see the GlobalRef docs).");
                self.vm
                    .attach_current_thread()
                    .and_then(|env| drop_impl(&env, self.as_obj()))
            }
        };

        if let Err(err) = res {
            debug!("error dropping global ref: {:#?}", err);
        }
    }
}
