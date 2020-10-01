use std::marker::PhantomData;

use crate::sys::jweak;

/// Wrapper around `sys::jweak` that adds a lifetime. This prevents it from
/// outliving the context in which it was acquired. It matches C's representation
/// of the raw pointer, so it can be used in any of the extern function argument
/// positions that would take a `jweak`.
///
/// See also: [WeakGlobalRef](./struct.WeakGlobalRef.html)
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct JWeak<'a> {
    internal: jweak,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<jweak> for JWeak<'a> {
    fn from(other: jweak) -> Self {
        Self {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> ::std::ops::Deref for JWeak<'a> {
    type Target = jweak;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl<'a> JWeak<'a> {
    /// Unwrap to the internal jni type.
    pub fn into_inner(self) -> jweak {
        self.internal
    }

    /// Creates a new null object
    pub fn null() -> Self {
        (::std::ptr::null_mut() as jweak).into()
    }
}
