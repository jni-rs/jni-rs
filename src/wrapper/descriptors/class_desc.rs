use strings::JNIString;

use objects::{JObject, JClass, GlobalRef};

use descriptors::Desc;

use JNIEnv;

use errors::*;

impl<'a, T> Desc<'a, JClass<'a>> for T
where
    T: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JClass<'a>> {
        env.find_class(self)
    }
}

impl<'a, 'b> Desc<'a, JClass<'a>> for JObject<'b> {
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JClass<'a>> {
        env.get_object_class(self)
    }
}

/// This conversion assumes that the `GlobalRef` is a pointer to a class object.
impl<'a> Desc<'a, JClass<'a>> for &'a GlobalRef {
    fn lookup(self, _: &JNIEnv<'a>) -> Result<JClass<'a>> {
        Ok(self.as_obj().into())
    }
}
