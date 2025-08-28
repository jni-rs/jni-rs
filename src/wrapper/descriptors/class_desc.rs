use crate::{
    descriptors::Desc,
    env::JNIEnv,
    errors::*,
    objects::{AutoLocal, IntoAutoLocal as _, JClass},
    strings::JNIString,
};

unsafe impl<'local, T> Desc<'local, JClass<'local>> for T
where
    T: Into<JNIString>,
{
    type Output = AutoLocal<'local, JClass<'local>>;

    fn lookup(self, env: &mut JNIEnv<'local>) -> Result<Self::Output> {
        Ok(env.find_class(self)?.auto())
    }
}

// Note: we don't implement `Desc<JClass>` for `&JObject` as a simple cast would
// make it a lot easier to mistakenly pass an object instance in places where a
// class is required.
