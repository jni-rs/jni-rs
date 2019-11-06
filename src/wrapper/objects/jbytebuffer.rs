use crate::{objects::JObject, sys::jobject};

/// Lifetime'd representation of a `jobject` that is an instance of the
/// ByteBuffer Java class. Just a `JObject` wrapped in a new class.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct JByteBuffer<'a>(JObject<'a>);

impl<'a> From<jobject> for JByteBuffer<'a> {
    fn from(other: jobject) -> Self {
        JByteBuffer(From::from(other))
    }
}

impl<'a> ::std::ops::Deref for JByteBuffer<'a> {
    type Target = JObject<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<JByteBuffer<'a>> for JObject<'a> {
    fn from(other: JByteBuffer) -> JObject {
        other.0
    }
}

impl<'a> From<JObject<'a>> for JByteBuffer<'a> {
    fn from(other: JObject) -> JByteBuffer {
        (other.into_inner() as jobject).into()
    }
}
