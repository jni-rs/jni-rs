use crate::{
    objects::JObject,
    sys::{jobject, jthrowable},
};

/// Lifetime'd representation of a `jthrowable`. Just a `JObject` wrapped in a
/// new class.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct JThrowable<'a>(JObject<'a>);

impl<'a> From<jthrowable> for JThrowable<'a> {
    fn from(other: jthrowable) -> Self {
        JThrowable(From::from(other as jobject))
    }
}

impl<'a> ::std::ops::Deref for JThrowable<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<JThrowable<'a>> for JObject<'a> {
    fn from(other: JThrowable) -> JObject {
        other.0
    }
}

impl<'a> From<JObject<'a>> for JThrowable<'a> {
    fn from(other: JObject) -> JThrowable {
        (other.into_inner() as jthrowable).into()
    }
}
