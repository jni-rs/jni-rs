use std::marker::PhantomData;
use sys::jmethodID;

#[repr(C)]
pub struct JMethodID<'a> {
    internal: jmethodID,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<jmethodID> for JMethodID<'a> {
    fn from(other: jmethodID) -> Self {
        JMethodID {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> JMethodID<'a> {
    pub fn into_inner(self) -> jmethodID {
        self.internal
    }
}
