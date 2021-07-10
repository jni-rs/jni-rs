use crate::{
    objects::JObject,
    sys::{jobject, jstring},
};

/// Lifetime'd representation of a `jstring`. Just a `JObject` wrapped in a new
/// class.
#[repr(transparent)]
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

impl<'a> From<JObject<'a>> for JString<'a> {
    fn from(other: JObject) -> JString {
        (other.into_inner() as jstring).into()
    }
}

impl<'a> std::default::Default for JString<'a> {
    fn default() -> Self {
        JString(JObject::null())
    }
}
