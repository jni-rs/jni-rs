use crate::{
    descriptors::Desc,
    env::Env,
    errors::*,
    objects::{JClass, JMethodID, JStaticMethodID},
    strings::JNIStr,
};

unsafe impl<'local, 'other_local, T, U, V> Desc<'local, JMethodID> for (T, U, V)
where
    T: Desc<'local, JClass<'other_local>>,
    U: AsRef<JNIStr>,
    V: AsRef<JNIStr>,
{
    type Output = JMethodID;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        env.get_method_id(self.0, self.1, self.2)
    }
}

unsafe impl<'local, 'other_local, T, Signature> Desc<'local, JMethodID> for (T, Signature)
where
    T: Desc<'local, JClass<'other_local>>,
    Signature: AsRef<JNIStr>,
{
    type Output = JMethodID;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        Desc::<JMethodID>::lookup((self.0, c"<init>", self.1.as_ref()), env)
    }
}

unsafe impl<'local, 'other_local, T, U, V> Desc<'local, JStaticMethodID> for (T, U, V)
where
    T: Desc<'local, JClass<'other_local>>,
    U: AsRef<JNIStr>,
    V: AsRef<JNIStr>,
{
    type Output = JStaticMethodID;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        env.get_static_method_id(self.0, self.1, self.2)
    }
}
