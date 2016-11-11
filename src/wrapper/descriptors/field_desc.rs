use errors::*;

use descriptors::Desc;

use objects::JFieldID;
use objects::JClass;

use strings::JNIString;

use JNIEnv;

impl<'a, T, U, V> Desc<'a, JFieldID<'a>> for (T, U, V)
    where T: Desc<'a, JClass<'a>>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    fn lookup(self, env: &JNIEnv<'a>) -> Result<JFieldID<'a>> {
        env.get_field_id(self.0, self.1, self.2)
    }
}
