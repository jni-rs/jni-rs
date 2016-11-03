use jobject::JObject;
use sys::{jobject, jclass};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct JClass<'a>(JObject<'a>);

impl<'a> From<jclass> for JClass<'a> {
    fn from(other: jclass) -> Self {
        JClass(From::from(other as jobject))
    }
}

impl<'a> ::std::ops::Deref for JClass<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<JClass<'a>> for JObject<'a> {
    fn from(other: JClass) -> JObject {
        other.0
    }
}

impl<'a> From<JObject<'a>> for JClass<'a> {
    fn from(other: JObject) -> JClass {
        (other.into_inner() as jclass).into()
    }
}
