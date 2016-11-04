use desc::Desc;
use jobject::JObject;
use ffi_str::JNIString;
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

pub struct ClassDesc<'a, S: Into<JNIString>>(pub Desc<S, JClass<'a>>);

pub trait IntoClassDesc<'a, S>
    where S: Into<JNIString>
{
    fn into_desc(self) -> ClassDesc<'a, S>;
}

impl<'a, S> IntoClassDesc<'a, S> for ClassDesc<'a, S>
    where S: Into<JNIString>
{
    fn into_desc(self) -> ClassDesc<'a, S> {
        self
    }
}

impl<'a, S> IntoClassDesc<'a, S> for S
    where S: Into<JNIString>
{
    fn into_desc(self) -> ClassDesc<'a, S> {
        ClassDesc(Desc::Descriptor(self))
    }
}

impl<'a> IntoClassDesc<'a, &'static str> for JClass<'a> {
    fn into_desc(self) -> ClassDesc<'a, &'static str> {
        ClassDesc(Desc::Value(self))
    }
}

impl<'a, S> IntoClassDesc<'a, S> for Desc<S, JClass<'a>>
    where S: Into<JNIString>
{
    fn into_desc(self) -> ClassDesc<'a, S> {
        ClassDesc(self)
    }
}
