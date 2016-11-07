use descriptors::Desc;

use descriptors::ClassDesc;
use descriptors::IntoClassDesc;

use objects::JMethodID;

use strings::JNIString;

/// Specialization of `Desc` for describing `JMethodID`s. Requires a class,
/// method name, and signature.
pub struct MethodDesc<'a,
                      T: IntoClassDesc<'a>,
                      U: Into<JNIString>,
                      V: Into<JNIString>> (
    pub Desc<(T, U, V), JMethodID<'a>>,
);

/// Trait representing anything that can be turned into a method descriptor.
/// Implementations are provided for `MethodDesc`, `Desc<(T, U, V), JMethodID>`,
/// `JMethodID`, and any tuple containing an `IntoClassDesc` implementor, and
/// two things that implement `Into<JNIString>`.
///
/// Implementors are better off implementing `Into<JNIString>` and
/// `IntoClassDesc` and letting the provided implementations for it do the rest.
pub trait IntoMethodDesc<'a> {
    type ClassDesc: IntoClassDesc<'a>;
    type Name: Into<JNIString>;
    type Sig: Into<JNIString>;

    fn into_desc(self) -> MethodDesc<'a, Self::ClassDesc, Self::Name, Self::Sig>;
}

impl<'a> IntoMethodDesc<'a> for JMethodID<'a> {
    type ClassDesc = ClassDesc<'a, &'static str>;
    type Name = &'static str;
    type Sig = &'static str;

    fn into_desc(self) -> MethodDesc<'a,
                                     ClassDesc<'a, &'static str>,
                                     &'static str,
                                     &'static str> {
        MethodDesc(Desc::Value(self))
    }
}

impl<'a, T, U, V> IntoMethodDesc<'a> for (T, U, V)
    where T: IntoClassDesc<'a>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    type ClassDesc = T;
    type Name = U;
    type Sig = V;

    fn into_desc(self) -> MethodDesc<'a, T, U, V> {
        MethodDesc(Desc::Descriptor(self))
    }
}

impl<'a, T, U, V> IntoMethodDesc<'a> for MethodDesc<'a, T, U, V>
    where T: IntoClassDesc<'a>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    type ClassDesc = T;
    type Name = U;
    type Sig = V;

    fn into_desc(self) -> MethodDesc<'a, T, U, V> {
        self
    }
}

impl<'a, T, U, V> IntoMethodDesc<'a> for Desc<(T, U, V), JMethodID<'a>>
    where T: IntoClassDesc<'a>,
          U: Into<JNIString>,
          V: Into<JNIString>
{
    type ClassDesc = T;
    type Name = U;
    type Sig = V;

    fn into_desc(self) -> MethodDesc<'a, T, U, V> {
        MethodDesc(self)
    }
}
