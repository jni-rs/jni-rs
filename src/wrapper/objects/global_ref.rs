use std::convert::From;
use std::mem;

use JNIEnv;
use JavaVM;
use errors::*;
use objects::JObject;
use sys::{
    self,
    jobject,
};

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

    /// Detach the global ref from the JNI environment to send it across thread
    /// boundaries.
    pub fn detach(self) -> Result<DetachedGlobalRef> {
        let env = unsafe { JNIEnv::from_raw(self.env)? };
        let vm = env.get_java_vm()?;

        let res = DetachedGlobalRef {
            obj: self.obj,
            vm: vm,
        };

        mem::forget(self); // prevent dropping the reference.

        Ok(res)
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
pub struct DetachedGlobalRef {
    obj: JObject<'static>,
    vm: JavaVM,
}

unsafe impl Send for DetachedGlobalRef {}

impl DetachedGlobalRef {
    /// Creates a new detached global reference. This assumes that
    /// `CreateGlobalRef`
    /// has alrady been called.
    pub unsafe fn new(vm: JavaVM, obj: sys::jobject) -> Self {
        DetachedGlobalRef {
            obj: JObject::from(obj),
            vm,
        }
    }

    /// Attach this ref to a `JNIEnv` to produce `GlobalRef`.
    pub fn attach(self, env: &JNIEnv) -> GlobalRef {
        let res = self.attach_impl(env);
        mem::forget(self);
        res
    }

    /// A temporary workaround for access to an object
    pub fn as_obj<'a>(&'a self) -> JObject<'a> {
        self.obj
    }

    /// A temporary workaround for `newGlobalRef` not destroing instance of `DetachedGlobalRef`
    pub fn clone_global_ref(&self, env: &JNIEnv) -> Result<DetachedGlobalRef> {
        let tmp = env.new_global_ref(self.obj)?;
        let new = DetachedGlobalRef {
            obj: tmp.obj,
            vm: self.vm.clone(),
        };
        mem::forget(tmp);
        Ok(new)
    }

    /// Unwrap to the internal JNI type.
    pub fn into_inner(self) -> sys::jobject {
        self.obj.into_inner()
    }

    fn drop_impl(&self) -> Result<()> {
        match self.vm.get_env() {
            Ok(env) => {
                let _ = self.attach_impl(&env);
                Ok(())
            }
            Err(Error(ErrorKind::ThreadDetached, _)) => {
                let env = self.vm.attach_current_thread()?;
                let _ = self.attach_impl(&env);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn attach_impl(&self, env: &JNIEnv) -> GlobalRef {
        GlobalRef {
            obj: self.obj,
            env: env.get_native_interface(),
        }
    }
}

impl Drop for DetachedGlobalRef {
    fn drop(&mut self) {
        match self.drop_impl() {
            Ok(()) => {}
            Err(e) => debug!("error dropping detached global ref: {:#?}", e),
        }
    }
}
