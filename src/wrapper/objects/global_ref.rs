use std::convert::From;

use errors::*;

use objects::JObject;

use sys::{jobject, JNIEnv};

/// A global JVM reference. These are "pinned" by the garbage collector and are
/// guaranteed to not get collected until released. Thus, this is allowed to
/// outlive the `JNIEnv` that it came from. Still can't cross thread boundaries
/// since it requires a pointer to the `JNIEnv` to do anything useful with it.
pub struct GlobalRef {
    obj: JObject<'static>,
    env: *mut JNIEnv,
}

impl<'a> From<&'a GlobalRef> for JObject<'a> {
    fn from(other: &'a GlobalRef) -> JObject<'a> {
        other.obj
    }
}

impl GlobalRef {
    /// Create a new global reference object. This assumes that
    /// `CreateGlobalRef` has already been called.
    pub unsafe fn new(env: *mut JNIEnv, obj: jobject) -> Self {
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
