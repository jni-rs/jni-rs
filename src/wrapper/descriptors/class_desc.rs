use crate::{
    descriptors::Desc,
    errors::*,
    objects::{AutoLocal, GlobalRef, JClass, JObject},
    strings::JNIString,
    JNIEnv,
};

unsafe impl<'local, T> Desc<'local, JClass<'local>> for T
where
    T: Into<JNIString>,
{
    type Output = AutoLocal<'local, JClass<'local>>;

    fn lookup(self, env: &mut JNIEnv<'local>) -> Result<Self::Output> {
        Ok(AutoLocal::new(env.find_class(self)?, env))
    }
}

unsafe impl<'local, 'other_local> Desc<'local, JClass<'local>> for JObject<'other_local> {
    type Output = AutoLocal<'local, JClass<'local>>;

    fn lookup(self, env: &mut JNIEnv<'local>) -> Result<Self::Output> {
        Desc::<JClass>::lookup(&self, env)
    }
}

unsafe impl<'local, 'other_local, 'obj_ref> Desc<'local, JClass<'local>>
    for &'obj_ref JObject<'other_local>
{
    type Output = AutoLocal<'local, JClass<'local>>;

    fn lookup(self, env: &mut JNIEnv<'local>) -> Result<Self::Output> {
        Ok(env.auto_local(env.get_object_class(self)?))
    }
}

/// This conversion assumes that the `GlobalRef` is a pointer to a class object.

// TODO: Generify `GlobalRef` and get rid of this `impl`. The transmute is
// sound-ish at the moment (`JClass` is currently `repr(transparent)`
// around `JObject`), but that may change in the future. Moreover, this
// doesn't check if the global reference actually refers to a
// `java.lang.Class` object.
unsafe impl<'local, 'obj_ref> Desc<'local, JClass<'static>> for &'obj_ref GlobalRef {
    type Output = &'obj_ref JClass<'static>;

    fn lookup(self, _: &mut JNIEnv<'local>) -> Result<Self::Output> {
        let obj: &JObject<'static> = self.as_ref();
        Ok(unsafe { std::mem::transmute(obj) })
    }
}
