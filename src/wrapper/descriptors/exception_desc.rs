use crate::{
    descriptors::Desc,
    env::Env,
    errors::*,
    objects::{AutoLocal, IntoAutoLocal as _, JClass, JObject, JThrowable, JValue},
    strings::{JNIStr, JNIString},
};

const DEFAULT_EXCEPTION_CLASS: &JNIStr = JNIStr::from_cstr(c"java/lang/RuntimeException");

unsafe impl<'local, 'other_local, C, M> Desc<'local, JThrowable<'local>> for (C, M)
where
    C: Desc<'local, JClass<'other_local>>,
    M: AsRef<JNIStr>,
{
    type Output = AutoLocal<'local, JThrowable<'local>>;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        let jmsg = env.new_string(self.1.as_ref())?.auto();
        let obj: JObject =
            env.new_object(self.0, c"(Ljava/lang/String;)V", &[JValue::from(&jmsg)])?;
        let throwable = env.cast_local::<JThrowable>(obj)?;
        Ok(throwable.auto())
    }
}

unsafe impl<'local> Desc<'local, JThrowable<'local>> for Exception {
    type Output = AutoLocal<'local, JThrowable<'local>>;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        let jni_class: JNIString = self.class.into();
        let jni_msg: JNIString = self.msg.into();
        Desc::<JThrowable>::lookup((jni_class, jni_msg), env)
    }
}

unsafe impl<'local> Desc<'local, JThrowable<'local>> for &str {
    type Output = AutoLocal<'local, JThrowable<'local>>;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        let jni_msg: JNIString = self.into();
        Desc::<JThrowable>::lookup((DEFAULT_EXCEPTION_CLASS, jni_msg), env)
    }
}

unsafe impl<'local> Desc<'local, JThrowable<'local>> for String {
    type Output = AutoLocal<'local, JThrowable<'local>>;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        let jni_msg: JNIString = self.into();
        Desc::<JThrowable>::lookup((DEFAULT_EXCEPTION_CLASS, jni_msg), env)
    }
}

unsafe impl<'local, T> Desc<'local, JThrowable<'local>> for T
where
    T: AsRef<JNIStr>,
{
    type Output = AutoLocal<'local, JThrowable<'local>>;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        Desc::<JThrowable>::lookup((DEFAULT_EXCEPTION_CLASS, self.as_ref()), env)
    }
}
