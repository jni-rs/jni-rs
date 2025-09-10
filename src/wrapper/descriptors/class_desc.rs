use crate::{
    descriptors::Desc,
    env::Env,
    errors::*,
    objects::{AutoLocal, IntoAutoLocal as _, JClass},
    strings::JNIStr,
};

unsafe impl<'local, T> Desc<'local, JClass<'local>> for T
where
    T: AsRef<JNIStr>,
{
    type Output = AutoLocal<'local, JClass<'local>>;

    fn lookup(self, env: &mut Env<'local>) -> Result<Self::Output> {
        Ok(env.find_class(self.as_ref())?.auto())
    }
}

// Note: we don't implement `Desc<JClass>` for `&JObject` as a simple cast would
// make it a lot easier to mistakenly pass an object instance in places where a
// class is required.
