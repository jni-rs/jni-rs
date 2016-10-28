use errors::*;
use std::convert::From;
use jnienv;
use jni_sys::{self, jobject, JNIEnv};

pub struct GlobalRef {
    obj: jobject,
    internal: *mut JNIEnv,
}

impl GlobalRef {
    pub unsafe fn new(env: *mut JNIEnv, obj: jobject) -> Self {
        GlobalRef {
            obj: obj,
            internal: env,
        }
    }

    fn drop_ref(&mut self) -> Result<()> {
        unsafe {
            jni_unchecked!(self.internal, DeleteGlobalRef, self.obj);
            check_exception!(self.internal);
        }
        Ok(())
    }
}

impl Drop for GlobalRef {
    fn drop(&mut self) {
        let env: jnienv::JNIEnv = self.internal.into();
        let res = self.drop_ref();
        match res {
            Ok(()) => {}
            Err(e) => debug!("error dropping global ref: {:#?}", e),
        }
    }
}
