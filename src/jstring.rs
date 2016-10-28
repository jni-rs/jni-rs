use jobject::JObject;
use jni_sys::{jobject, jstring};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JString<'a>(JObject<'a>);

impl<'a> From<jstring> for JString<'a> {
    fn from(other: jstring) -> Self {
        JString(From::from(other as jobject))
    }
}

impl<'a> ::std::ops::Deref for JString<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<JString<'a>> for JObject<'a> {
    fn from(other: JString) -> JObject {
        other.0
    }
}
