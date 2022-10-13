use crate::{
    descriptors::Desc,
    errors::*,
    objects::{JClass, JObject, JThrowable, JValue},
    strings::JNIString,
    JNIEnv,
};

const DEFAULT_EXCEPTION_CLASS: &str = "java/lang/RuntimeException";

impl<'a, 'c, C, M> Desc<'a, JThrowable<'a>> for (C, M)
where
    C: Desc<'a, JClass<'c>>,
    M: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JThrowable<'a>> {
        let jmsg: JObject = env.new_string(self.1)?.into();
        let obj: JThrowable = env
            .new_object(self.0, "(Ljava/lang/String;)V", &[JValue::from(jmsg)])?
            .into();
        Ok(obj)
    }
}

impl<'a> Desc<'a, JThrowable<'a>> for Exception {
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JThrowable<'a>> {
        (self.class, self.msg).lookup(env)
    }
}

impl<'a, 'b> Desc<'a, JThrowable<'a>> for &'b str {
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JThrowable<'a>> {
        (DEFAULT_EXCEPTION_CLASS, self).lookup(env)
    }
}

impl<'a> Desc<'a, JThrowable<'a>> for String {
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JThrowable<'a>> {
        (DEFAULT_EXCEPTION_CLASS, self).lookup(env)
    }
}

impl<'a> Desc<'a, JThrowable<'a>> for JNIString {
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JThrowable<'a>> {
        (DEFAULT_EXCEPTION_CLASS, self).lookup(env)
    }
}
