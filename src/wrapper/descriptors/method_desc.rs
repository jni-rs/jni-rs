use descriptors::Desc;

use descriptors::ClassDesc;
use descriptors::IntoClassDesc;

use objects::JMethodID;

use strings::JNIString;

/// Specialization of `Desc` for describing `JMethodID`s. Requires a class,
/// method name, and signature.
pub struct MethodDesc<'a, S: Into<JNIString>,
                      T: IntoClassDesc<'a, S>,
                      U: Into<JNIString>,
                      V: Into<JNIString>> (
        pub Desc<(T, U, V),
                 JMethodID<'a>>,
        pub ::std::marker::PhantomData<S>
);

/// Trait representing anything that can be turned into a method descriptor.
/// Implementations are provided for `MethodDesc`, `Desc<(T, U, V), JMethodID>`,
/// `JMethodID`, and any tuple containing an `IntoClassDesc` implementor, and
/// two things that implement `Into<JNIString>`.
///
/// Implementors are better off implementing `Into<JNIString>` and
/// `IntoClassDesc` and letting the provided implementations for it do the rest.
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
