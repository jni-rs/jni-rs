use strings::JNIString;

use objects::JClass;

use descriptors::Desc;

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
