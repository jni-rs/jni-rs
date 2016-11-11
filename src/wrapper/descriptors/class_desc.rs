use strings::JNIString;

use objects::JClass;

use descriptors::Desc;

use JNIEnv;

use errors::*;

impl<'a, T> Desc<'a, JClass<'a>> for T
    where T: Into<JNIString>
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JClass<'a>> {
        env.find_class(self)
    }
}
