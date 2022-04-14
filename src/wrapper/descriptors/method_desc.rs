use crate::{
    descriptors::Desc,
    errors::*,
    objects::{JClass, JMethodID, JStaticMethodID},
    strings::JNIString,
    JNIEnv,
};

impl<'a, 'c, T, U, V> Desc<'a, JMethodID> for (T, U, V)
where
    T: Desc<'a, JClass<'c>>,
    U: Into<JNIString>,
    V: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JMethodID> {
        env.get_method_id(self.0, self.1, self.2)
    }
}

impl<'a, 'c, T, Signature> Desc<'a, JMethodID> for (T, Signature)
where
    T: Desc<'a, JClass<'c>>,
    Signature: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JMethodID> {
        (self.0, "<init>", self.1).lookup(env)
    }
}

impl<'a, 'c, T, U, V> Desc<'a, JStaticMethodID> for (T, U, V)
where
    T: Desc<'a, JClass<'c>>,
    U: Into<JNIString>,
    V: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JStaticMethodID> {
        env.get_static_method_id(self.0, self.1, self.2)
    }
}
