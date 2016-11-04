use std::marker::PhantomData;

use desc::Desc;

use jclass::ClassDesc;
use jclass::IntoClassDesc;

use ffi_str::JNIString;

use sys::jmethodID;

#[repr(C)]
#[derive(Copy, Clone)]
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

pub struct MethodDesc<'a, S: Into<JNIString>,
                      T: IntoClassDesc<'a, S>,
                      U: Into<JNIString>,
                      V: Into<JNIString>> (
        pub Desc<(T, U, V),
                 JMethodID<'a>>,
        pub ::std::marker::PhantomData<S>
);

pub trait IntoMethodDesc<'a, S, T, U, V>
    where S: Into<JNIString>,
          T: IntoClassDesc<'a, S>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    fn into_desc(self) -> MethodDesc<'a, S, T, U, V>;
}

impl<'a> IntoMethodDesc<'a,
                        &'static str,
                        ClassDesc<'a, &'static str>,
                        &'static str,
                        &'static str>
    for JMethodID<'a> {
        fn into_desc(self) -> MethodDesc<'a,
                                         &'static str,
                                         ClassDesc<'a, &'static str>,
                                         &'static str,
                                         &'static str> {
        MethodDesc(Desc::Value(self), Default::default())
    }
}

impl<'a, S, T, U, V> IntoMethodDesc<'a, S, T, U, V> for (T, U, V)
    where S: Into<JNIString>,
          T: IntoClassDesc<'a, S>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    fn into_desc(self) -> MethodDesc<'a, S, T, U, V> {
        MethodDesc(Desc::Descriptor(self), Default::default())
    }
}

impl<'a, S, T, U, V> IntoMethodDesc<'a, S, T, U, V> for MethodDesc<'a,
                                                                   S,
                                                                   T,
                                                                   U,
                                                                   V>
    where S: Into<JNIString>,
          T: IntoClassDesc<'a, S>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    fn into_desc(self) -> MethodDesc<'a, S, T, U, V> {
        self
    }
}

impl<'a, S, T, U, V> IntoMethodDesc<'a, S, T, U, V> for Desc<(T, U, V),
                                                             JMethodID<'a>>
    where S: Into<JNIString>,
          T: IntoClassDesc<'a, S>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    fn into_desc(self) -> MethodDesc<'a, S, T, U, V> {
        MethodDesc(self, Default::default())
    }
}
