use std::{convert::From, sync::Arc};

use log::{debug, warn};

use crate::{errors::Result, objects::JWeak, sys, JNIEnv, JavaVM};

/// A weak global JVM reference. **It doesn't protect underlaying object from been
/// garbage collected.** Still, `WeakGlobalRef` is allowed to outlive the `JNIEnv` that
/// it came from and can be used in other threads.
///
/// `WeakGloablRef` may be created via
/// [JNIEnv::new_weak_global_ref](../struct.JNIEnv.html#method.new_weak_global_ref).
///
/// `WeakGloablRef` doesn't allow access to the underlying object but may be
/// upgraded into _potentially null_ [GlobalRef](struct.GlobalRef.html) via
/// [JNIEnv::upgrade_weak_global_ref](../struct.JNIEnv.html#method.upgrade_weak_global_ref).
///
/// `WeakGlobalRef` can be cloned to use weak global reference in different contexts.
///
/// Underlying weak global reference will be dropped, when the last instance
/// of `WeakGlobalRef` leaves its scope.
///
/// It is _recommended_ that a native thread that drops the global reference is attached
/// to the Java thread (i.e., has an instance of `JNIEnv`). If the native thread is *not* attached,
/// the `WeakGlobalRef::drop` will print a warning and implicitly `attach` and `detach` the thread,
/// which significantly affects performance.
#[derive(Clone)]
pub struct WeakGlobalRef {
    inner: Arc<WeakGlobalRefGuard>,
}

struct WeakGlobalRefGuard {
    obj: JWeak<'static>,
    vm: JavaVM,
}

unsafe impl Send for WeakGlobalRefGuard {}
unsafe impl Sync for WeakGlobalRefGuard {}

impl WeakGlobalRef {
    /// Creates a new wrapper for a global reference.
    ///
    /// # Safety
    ///
    /// Expects a valid raw global reference that should be created with `NewWeakGlobalRef` JNI function.
    pub(crate) unsafe fn from_raw(vm: JavaVM, raw_weak_global_ref: sys::jweak) -> Self {
        Self {
            inner: Arc::new(WeakGlobalRefGuard::from_raw(vm, raw_weak_global_ref)),
        }
    }

    /// Get the underlying JWeak object
    pub fn as_weak(&self) -> JWeak {
        self.inner.as_weak()
    }
}

impl WeakGlobalRefGuard {
    /// Creates a new global reference guard. This assumes that `NewWeakGlobalRef`
    /// has already been called.
    unsafe fn from_raw(vm: JavaVM, obj: sys::jweak) -> Self {
        WeakGlobalRefGuard {
            obj: JWeak::from(obj),
            vm,
        }
    }

    /// Get the underlying JWeak object
    pub fn as_weak(&self) -> JWeak {
        self.obj
    }
}

impl Drop for WeakGlobalRefGuard {
    fn drop(&mut self) {
        fn drop_impl(env: &JNIEnv, weak_global_ref: JWeak) -> Result<()> {
            let internal = env.get_native_interface();
            // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
            jni_unchecked!(internal, DeleteWeakGlobalRef, weak_global_ref.into_inner());
            Ok(())
        }

        let res = match self.vm.get_env() {
            Ok(env) => drop_impl(&env, self.obj),
            Err(_) => {
                warn!("Dropping a WeakGlobalRef in a detached thread. Fix your code if this message appears frequently (see the WeakGlobalRef docs).");
                self.vm
                    .attach_current_thread()
                    .and_then(|env| drop_impl(&env, self.obj))
            }
        };

        if let Err(err) = res {
            debug!("error dropping weak global ref: {:#?}", err);
        }
    }
}
