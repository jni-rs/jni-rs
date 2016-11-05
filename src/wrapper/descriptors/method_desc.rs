use descriptors::Desc;

use descriptors::ClassDesc;
use descriptors::IntoClassDesc;

use objects::JMethodID;

use strings::JNIString;

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

impl<'a, S, T, U, V> IntoMethodDesc<'a, S, T, U, V>
    for MethodDesc<'a, S, T, U, V>
    where S: Into<JNIString>,
          T: IntoClassDesc<'a, S>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    fn into_desc(self) -> MethodDesc<'a, S, T, U, V> {
        self
    }
}

impl<'a, S, T, U, V> IntoMethodDesc<'a, S, T, U, V>
    for Desc<(T, U, V), JMethodID<'a>>
    where S: Into<JNIString>,
          T: IntoClassDesc<'a, S>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    fn into_desc(self) -> MethodDesc<'a, S, T, U, V> {
        MethodDesc(self, Default::default())
    }
}
