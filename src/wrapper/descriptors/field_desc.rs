use crate::{
    descriptors::Desc,
    errors::*,
    objects::{JClass, JFieldID, JStaticFieldID},
    strings::JNIString,
    JNIEnv,
};

impl<'a, 'c, T, U, V> Desc<'a, JFieldID<'a>> for (T, U, V)
where
    T: Desc<'a, JClass<'c>>,
    U: Into<JNIString>,
    V: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JFieldID<'a>> {
        env.get_field_id(self.0, self.1, self.2)
    }
}

impl<'a, 'c, T, U, V> Desc<'a, JStaticFieldID<'a>> for (T, U, V)
where
    T: Desc<'a, JClass<'c>>,
    U: Into<JNIString>,
    V: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JStaticFieldID<'a>> {
        env.get_static_field_id(self.0, self.1, self.2)
    }
}
