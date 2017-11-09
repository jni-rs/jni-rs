use std::convert::From;
use std::mem;

use JNIEnv;
use errors::*;
use objects::JObject;
use sys::{self, jobject};

/// A global JVM reference. These are "pinned" by the garbage collector and are
/// guaranteed to not get collected until released. Thus, this is allowed to
/// outlive the `JNIEnv` that it came from. Still can't cross thread boundaries
/// since it requires a pointer to the `JNIEnv` to do anything useful with it.
pub struct GlobalRef {
    obj: JObject<'static>,
    env: *mut sys::JNIEnv,
}

impl<'a> From<&'a GlobalRef> for JObject<'a> {
    fn from(other: &'a GlobalRef) -> JObject<'a> {
        other.obj
    }
}

impl GlobalRef {
    /// Create a new global reference object. This assumes that
    /// `CreateGlobalRef` has already been called.
    pub unsafe fn new(env: *mut sys::JNIEnv, obj: jobject) -> Self {
        GlobalRef {
            obj: JObject::from(obj),
            env: env,
        }
    }

    fn drop_ref(&mut self) -> Result<()> {
        unsafe {
            jni_unchecked!(self.env, DeleteGlobalRef, self.obj.into_inner());
            check_exception!(self.env);
        }
        Ok(())
    }

    /// Get the object from the global ref
    ///
    /// This borrows the ref and prevents it from being dropped as long as the
    /// JObject sticks around.
    pub fn as_obj<'a>(&'a self) -> JObject<'a> {
        self.obj
    }

    /// Detach the global ref from the JNI environment to send it across thread boundaries.
    pub fn detach(self) -> DetachedGlobalRef {
        let res = DetachedGlobalRef { obj: self.obj };
        mem::forget(self); // prevent dropping the reference.
        res
    }
}

impl Drop for GlobalRef {
    fn drop(&mut self) {
        let res = self.drop_ref();
        match res {
            Ok(()) => {}
            Err(e) => debug!("error dropping global ref: {:#?}", e),
        }
    }
}

/// A detached global JVM reference that can be sent across threads. To do
/// anything useful with it, it must be `attach`ed first.
///
/// Warning: detached global ref will leak memory if dropped. Attach to a
/// `JNIEnv` to prevent this.
#[must_use]
pub struct DetachedGlobalRef {
    obj: JObject<'static>,
}

unsafe impl Send for DetachedGlobalRef {}

impl DetachedGlobalRef {
    /// Creates a new detached global reference. This assumes that `CreateGlobalRef`
    /// has alrady been called.
    pub unsafe fn new(obj: sys::jobject) -> Self {
        DetachedGlobalRef { obj: JObject::from(obj) }
    }

    /// Attach this ref to a `JNIEnv` to produce `GlobalRef`.
    pub fn attach(self, env: &JNIEnv) -> GlobalRef {
        GlobalRef {
            obj: self.obj,
            env: env.get_native_interface(),
        }
    }

    /// Unwrap to the internal JNI type.
    pub fn into_inner(self) -> sys::jobject {
        self.obj.into_inner()
    }
}
