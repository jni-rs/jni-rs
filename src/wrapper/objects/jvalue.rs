use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

use log::trace;

use crate::{errors::*, objects::JObject, signature::Primitive, sys::*};

/// A Java owned local reference or primitive value.
///
/// This type is used for values returned from Java method calls. If the Java
/// method returns an object reference, it will take the form of an owned
/// [`JObject`].
///
/// See also [`JValue`], which is used for Java method call parameters. It is
/// different from this type in that it *borrows* an object reference instead
/// of owning one.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum JValueOwned<'local> {
    Object(JObject<'local>),
    Byte(jbyte),
    Char(jchar),
    Short(jshort),
    Int(jint),
    Long(jlong),
    Bool(jboolean),
    Float(jfloat),
    Double(jdouble),
    Void,
}

/// A Java borrowed local reference or primitive value.
///
/// This type is used for parameters passed to Java method calls. If the Java
/// method is to be passed an object reference, it takes the form of a borrowed
/// <code>&[JObject]</code>.
///
/// See also [`JValueOwned`], which is used for Java method return values. It is
/// different from this type in that it *owns* an object reference instead
/// of borrowing one.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum JValue<'obj_ref> {
    Object(&'obj_ref JObject<'obj_ref>),
    Byte(jbyte),
    Char(jchar),
    Short(jshort),
    Int(jint),
    Long(jlong),
    Bool(jboolean),
    Float(jfloat),
    Double(jdouble),
    Void,
}

impl<'local> JValueOwned<'local> {
    /// Convert the enum to its jni-compatible equivalent.
    pub fn as_jni(&self) -> jvalue {
        self.borrow().as_jni()
    }

    /// Get the type name for the enum variant.
    pub fn type_name(&self) -> &'static str {
        self.borrow().type_name()
    }

    /// Get the primitive type for the enum variant. If it's not a primitive
    /// (i.e. an Object), returns None.
    pub fn primitive_type(&self) -> Option<Primitive> {
        self.borrow().primitive_type()
    }

    /// Try to unwrap to an Object.
    pub fn l(self) -> Result<JObject<'local>> {
        match self {
            Self::Object(obj) => Ok(obj),
            _ => Err(Error::WrongJValueType("object", self.type_name())),
        }
    }

    /// Try to unwrap to a boolean.
    pub fn z(self) -> Result<bool> {
        self.borrow().z()
    }

    /// Try to unwrap to a byte.
    pub fn b(self) -> Result<jbyte> {
        self.borrow().b()
    }

    /// Try to unwrap to a char.
    pub fn c(self) -> Result<jchar> {
        self.borrow().c()
    }

    /// Try to unwrap to a double.
    pub fn d(self) -> Result<jdouble> {
        self.borrow().d()
    }

    /// Try to unwrap to a float.
    pub fn f(self) -> Result<jfloat> {
        self.borrow().f()
    }

    /// Try to unwrap to an int.
    pub fn i(self) -> Result<jint> {
        self.borrow().i()
    }

    /// Try to unwrap to a long.
    pub fn j(self) -> Result<jlong> {
        self.borrow().j()
    }

    /// Try to unwrap to a short.
    pub fn s(self) -> Result<jshort> {
        self.borrow().s()
    }

    /// Try to unwrap to a void.
    pub fn v(self) -> Result<()> {
        self.borrow().v()
    }

    /// Copies or borrows the value in this `JValueOwned`.
    ///
    /// If the value is a primitive type, it is copied. If the value is an
    /// object reference, it is borrowed.
    pub fn borrow(&self) -> JValue {
        match self {
            Self::Object(o) => JValue::Object(o),
            Self::Byte(v) => JValue::Byte(*v),
            Self::Char(v) => JValue::Char(*v),
            Self::Short(v) => JValue::Short(*v),
            Self::Int(v) => JValue::Int(*v),
            Self::Long(v) => JValue::Long(*v),
            Self::Bool(v) => JValue::Bool(*v),
            Self::Float(v) => JValue::Float(*v),
            Self::Double(v) => JValue::Double(*v),
            Self::Void => JValue::Void,
        }
    }
}

impl<'obj_ref> JValue<'obj_ref> {
    /// Convert the enum to its jni-compatible equivalent.
    pub fn as_jni(&self) -> jvalue {
        let val: jvalue = match self {
            Self::Object(obj) => jvalue { l: obj.as_raw() },
            Self::Byte(byte) => jvalue { b: *byte },
            Self::Char(char) => jvalue { c: *char },
            Self::Short(short) => jvalue { s: *short },
            Self::Int(int) => jvalue { i: *int },
            Self::Long(long) => jvalue { j: *long },
            Self::Bool(boolean) => jvalue { b: *boolean as i8 },
            Self::Float(float) => jvalue { f: *float },
            Self::Double(double) => jvalue { d: *double },
            Self::Void => jvalue {
                l: ::std::ptr::null_mut(),
            },
        };
        trace!("converted {:?} to jvalue {:?}", self, unsafe {
            ::std::mem::transmute::<_, u64>(val)
        });
        val
    }

    /// Convert the enum to its jni-compatible equivalent.
    #[deprecated = "Use `as_jni` instead."]
    pub fn to_jni(self) -> jvalue {
        self.as_jni()
    }

    /// Get the type name for the enum variant.
    pub fn type_name(&self) -> &'static str {
        match *self {
            Self::Void => "void",
            Self::Object(_) => "object",
            Self::Byte(_) => "byte",
            Self::Char(_) => "char",
            Self::Short(_) => "short",
            Self::Int(_) => "int",
            Self::Long(_) => "long",
            Self::Bool(_) => "bool",
            Self::Float(_) => "float",
            Self::Double(_) => "double",
        }
    }

    /// Get the primitive type for the enum variant. If it's not a primitive
    /// (i.e. an Object), returns None.
    pub fn primitive_type(&self) -> Option<Primitive> {
        Some(match *self {
            Self::Object(_) => return None,
            Self::Void => Primitive::Void,
            Self::Byte(_) => Primitive::Byte,
            Self::Char(_) => Primitive::Char,
            Self::Short(_) => Primitive::Short,
            Self::Int(_) => Primitive::Int,
            Self::Long(_) => Primitive::Long,
            Self::Bool(_) => Primitive::Boolean,
            Self::Float(_) => Primitive::Float,
            Self::Double(_) => Primitive::Double,
        })
    }

    /// Try to unwrap to an Object.
    pub fn l(self) -> Result<&'obj_ref JObject<'obj_ref>> {
        match self {
            Self::Object(obj) => Ok(obj),
            _ => Err(Error::WrongJValueType("object", self.type_name())),
        }
    }

    /// Try to unwrap to a boolean.
    pub fn z(self) -> Result<bool> {
        match self {
            Self::Bool(b) => Ok(b == JNI_TRUE),
            _ => Err(Error::WrongJValueType("bool", self.type_name())),
        }
    }

    /// Try to unwrap to a byte.
    pub fn b(self) -> Result<jbyte> {
        match self {
            Self::Byte(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jbyte", self.type_name())),
        }
    }

    /// Try to unwrap to a char.
    pub fn c(self) -> Result<jchar> {
        match self {
            Self::Char(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jchar", self.type_name())),
        }
    }

    /// Try to unwrap to a double.
    pub fn d(self) -> Result<jdouble> {
        match self {
            Self::Double(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jdouble", self.type_name())),
        }
    }

    /// Try to unwrap to a float.
    pub fn f(self) -> Result<jfloat> {
        match self {
            Self::Float(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jfloat", self.type_name())),
        }
    }

    /// Try to unwrap to an int.
    pub fn i(self) -> Result<jint> {
        match self {
            Self::Int(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jint", self.type_name())),
        }
    }

    /// Try to unwrap to a long.
    pub fn j(self) -> Result<jlong> {
        match self {
            Self::Long(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jlong", self.type_name())),
        }
    }

    /// Try to unwrap to a short.
    pub fn s(self) -> Result<jshort> {
        match self {
            Self::Short(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jshort", self.type_name())),
        }
    }

    /// Try to unwrap to a void.
    pub fn v(self) -> Result<()> {
        match self {
            Self::Void => Ok(()),
            _ => Err(Error::WrongJValueType("void", self.type_name())),
        }
    }
}

impl<'obj_ref> From<&'obj_ref JValueOwned<'obj_ref>> for JValue<'obj_ref> {
    fn from(other: &'obj_ref JValueOwned) -> Self {
        other.borrow()
    }
}

impl<'local, T: Into<JObject<'local>>> From<T> for JValueOwned<'local> {
    fn from(other: T) -> Self {
        Self::Object(other.into())
    }
}

impl<'obj_ref, T: AsRef<JObject<'obj_ref>>> From<&'obj_ref T> for JValue<'obj_ref> {
    fn from(other: &'obj_ref T) -> Self {
        Self::Object(other.as_ref())
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for JObject<'local> {
    type Error = Error;

    fn try_from(value: JValueOwned<'local>) -> Result<Self> {
        value.l()
    }
}

impl<'local> From<bool> for JValueOwned<'local> {
    fn from(other: bool) -> Self {
        Self::Bool(if other { JNI_TRUE } else { JNI_FALSE })
    }
}

impl<'obj_ref> From<bool> for JValue<'obj_ref> {
    fn from(other: bool) -> Self {
        Self::Bool(if other { JNI_TRUE } else { JNI_FALSE })
    }
}

// jbool
impl<'local> From<jboolean> for JValueOwned<'local> {
    fn from(other: jboolean) -> Self {
        Self::Bool(other)
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for jboolean {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.borrow().try_into()
    }
}

impl<'obj_ref> From<jboolean> for JValue<'obj_ref> {
    fn from(other: jboolean) -> Self {
        Self::Bool(other)
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for jboolean {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        match value {
            JValue::Bool(b) => Ok(b),
            _ => Err(Error::WrongJValueType("bool", value.type_name())),
        }
    }
}

// jchar
impl<'local> From<jchar> for JValueOwned<'local> {
    fn from(other: jchar) -> Self {
        Self::Char(other)
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for jchar {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.c()
    }
}

impl<'obj_ref> From<jchar> for JValue<'obj_ref> {
    fn from(other: jchar) -> Self {
        Self::Char(other)
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for jchar {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        value.c()
    }
}

// jshort
impl<'local> From<jshort> for JValueOwned<'local> {
    fn from(other: jshort) -> Self {
        Self::Short(other)
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for jshort {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.s()
    }
}

impl<'obj_ref> From<jshort> for JValue<'obj_ref> {
    fn from(other: jshort) -> Self {
        Self::Short(other)
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for jshort {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        value.s()
    }
}

// jfloat
impl<'local> From<jfloat> for JValueOwned<'local> {
    fn from(other: jfloat) -> Self {
        Self::Float(other)
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for jfloat {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.f()
    }
}

impl<'obj_ref> From<jfloat> for JValue<'obj_ref> {
    fn from(other: jfloat) -> Self {
        Self::Float(other)
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for jfloat {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        value.f()
    }
}

// jdouble
impl<'local> From<jdouble> for JValueOwned<'local> {
    fn from(other: jdouble) -> Self {
        Self::Double(other)
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for jdouble {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.d()
    }
}

impl<'obj_ref> From<jdouble> for JValue<'obj_ref> {
    fn from(other: jdouble) -> Self {
        Self::Double(other)
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for jdouble {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        value.d()
    }
}

// jint
impl<'local> From<jint> for JValueOwned<'local> {
    fn from(other: jint) -> Self {
        Self::Int(other)
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for jint {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.i()
    }
}

impl<'obj_ref> From<jint> for JValue<'obj_ref> {
    fn from(other: jint) -> Self {
        Self::Int(other)
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for jint {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        value.i()
    }
}

// jlong
impl<'local> From<jlong> for JValueOwned<'local> {
    fn from(other: jlong) -> Self {
        Self::Long(other)
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for jlong {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.j()
    }
}

impl<'obj_ref> From<jlong> for JValue<'obj_ref> {
    fn from(other: jlong) -> Self {
        Self::Long(other)
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for jlong {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        value.j()
    }
}

// jbyte
impl<'local> From<jbyte> for JValueOwned<'local> {
    fn from(other: jbyte) -> Self {
        Self::Byte(other)
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for jbyte {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.b()
    }
}

impl<'obj_ref> From<jbyte> for JValue<'obj_ref> {
    fn from(other: jbyte) -> Self {
        Self::Byte(other)
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for jbyte {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        value.b()
    }
}

// jvoid
impl<'local> From<()> for JValueOwned<'local> {
    fn from(_: ()) -> Self {
        Self::Void
    }
}

impl<'local> TryFrom<JValueOwned<'local>> for () {
    type Error = Error;

    fn try_from(value: JValueOwned) -> Result<Self> {
        value.v()
    }
}

impl<'obj_ref> From<()> for JValue<'obj_ref> {
    fn from(_: ()) -> Self {
        Self::Void
    }
}

impl<'obj_ref> TryFrom<JValue<'obj_ref>> for () {
    type Error = Error;

    fn try_from(value: JValue) -> Result<Self> {
        value.v()
    }
}
