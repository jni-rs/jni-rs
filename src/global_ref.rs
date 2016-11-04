use errors::*;
use std::convert::From;
use sys::{jobject, JNIEnv};

pub struct GlobalRef {
    obj: jobject,
    env: *mut JNIEnv,
}

impl GlobalRef {
    pub unsafe fn new(env: *mut JNIEnv, obj: jobject) -> Self {
        GlobalRef {
            obj: obj,
            env: env,
        }
    }

    fn drop_ref(&mut self) -> Result<()> {
        unsafe {
            jni_unchecked!(self.env, DeleteGlobalRef, self.obj);
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
