use errors::*;

use descriptors::Desc;

use objects::JMethodID;
use objects::JStaticMethodID;
use objects::JClass;

use strings::JNIString;

use JNIEnv;

impl<'a, T, U, V> Desc<'a, JMethodID<'a>> for (T, U, V)
where
    T: Desc<'a, JClass<'a>>,
    U: Into<JNIString>,
    V: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JMethodID<'a>> {
        env.get_method_id(self.0, self.1, self.2)
    }
}

impl<'a, T, Signature> Desc<'a, JMethodID<'a>> for (T, Signature)
    where
        T: Desc<'a, JClass<'a>>,
        Signature: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JMethodID<'a>> {
        (self.0, "<init>", self.1).lookup(env)
    }
}

impl<'a, T, U, V> Desc<'a, JStaticMethodID<'a>> for (T, U, V)
where
    T: Desc<
        'a,
        JClass<'a>,
    >,
    U: Into<JNIString>,
    V: Into<JNIString>,
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JStaticMethodID<'a>> {
        env.get_static_method_id(self.0, self.1, self.2)
    }
}
