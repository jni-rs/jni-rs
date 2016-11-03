use std::marker::PhantomData;
use sys::jobject;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct JObject<'a> {
    internal: jobject,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<jobject> for JObject<'a> {
    fn from(other: jobject) -> Self {
        JObject {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> ::std::ops::Deref for JObject<'a> {
    type Target = jobject;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a> JObject<'a> {
    pub fn into_inner(self) -> jobject {
        self.internal
    }
}
