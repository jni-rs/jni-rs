use std::convert::TryFrom;
use std::mem::transmute;

use log::trace;

use crate::{errors::*, objects::JObject, signature::Primitive, sys::*};

/// Rusty version of the JNI C `jvalue` enum. Used in Java method call arguments
/// and returns.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum JValue<'a> {
    Object(JObject<'a>),
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

impl<'a> From<JValue<'a>> for jvalue {
    fn from(other: JValue) -> jvalue {
        other.to_jni()
    }
}

impl<'a> JValue<'a> {
    /// Convert the enum to its jni-compatible equivalent.
    pub fn to_jni(self) -> jvalue {
        let val: jvalue = match self {
            JValue::Object(obj) => jvalue {
                l: unsafe { transmute(obj) },
            },
            JValue::Byte(byte) => jvalue { b: byte },
            JValue::Char(char) => jvalue { c: char },
            JValue::Short(short) => jvalue { s: short },
            JValue::Int(int) => jvalue { i: int },
            JValue::Long(long) => jvalue { j: long },
            JValue::Bool(boolean) => jvalue { b: boolean as i8 },
            JValue::Float(float) => jvalue { f: float },
            JValue::Double(double) => jvalue { d: double },
            JValue::Void => jvalue {
                l: ::std::ptr::null_mut(),
            },
        };
        trace!("converted {:?} to jvalue {:?}", self, unsafe {
            ::std::mem::transmute::<_, u64>(val)
        });
        val
    }

    /// Get the type name for the enum variant.
    pub fn type_name(&self) -> &'static str {
        match *self {
            JValue::Void => "void",
            JValue::Object(_) => "object",
            JValue::Byte(_) => "byte",
            JValue::Char(_) => "char",
            JValue::Short(_) => "short",
            JValue::Int(_) => "int",
            JValue::Long(_) => "long",
            JValue::Bool(_) => "bool",
            JValue::Float(_) => "float",
            JValue::Double(_) => "double",
        }
    }

    /// Get the primitive type for the enum variant. If it's not a primitive
    /// (i.e. an Object), returns None.
    pub fn primitive_type(&self) -> Option<Primitive> {
        Some(match *self {
            JValue::Object(_) => return None,
            JValue::Void => Primitive::Void,
            JValue::Byte(_) => Primitive::Byte,
            JValue::Char(_) => Primitive::Char,
            JValue::Short(_) => Primitive::Short,
            JValue::Int(_) => Primitive::Int,
            JValue::Long(_) => Primitive::Long,
            JValue::Bool(_) => Primitive::Boolean,
            JValue::Float(_) => Primitive::Float,
            JValue::Double(_) => Primitive::Double,
        })
    }

    /// Try to unwrap to an Object.
    pub fn l(self) -> Result<JObject<'a>> {
        match self {
            JValue::Object(obj) => Ok(obj),
            _ => Err(Error::WrongJValueType("object", self.type_name())),
        }
    }

    /// Try to unwrap to a boolean.
    pub fn z(self) -> Result<bool> {
        match self {
            JValue::Bool(b) => Ok(b == JNI_TRUE),
            _ => Err(Error::WrongJValueType("bool", self.type_name())),
        }
    }

    /// Try to unwrap to a byte.
    pub fn b(self) -> Result<jbyte> {
        match self {
            JValue::Byte(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jbyte", self.type_name())),
        }
    }

    /// Try to unwrap to a char.
    pub fn c(self) -> Result<jchar> {
        match self {
            JValue::Char(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jchar", self.type_name())),
        }
    }

    /// Try to unwrap to a double.
    pub fn d(self) -> Result<jdouble> {
        match self {
            JValue::Double(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jdouble", self.type_name())),
        }
    }

    /// Try to unwrap to a float.
    pub fn f(self) -> Result<jfloat> {
        match self {
            JValue::Float(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jfloat", self.type_name())),
        }
    }

    /// Try to unwrap to an int.
    pub fn i(self) -> Result<jint> {
        match self {
            JValue::Int(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jint", self.type_name())),
        }
    }

    /// Try to unwrap to a long.
    pub fn j(self) -> Result<jlong> {
        match self {
            JValue::Long(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jlong", self.type_name())),
        }
    }

    /// Try to unwrap to a short.
    pub fn s(self) -> Result<jshort> {
        match self {
            JValue::Short(b) => Ok(b),
            _ => Err(Error::WrongJValueType("jshort", self.type_name())),
        }
    }

    /// Try to unwrap to a void.
    pub fn v(self) -> Result<()> {
        match self {
            JValue::Void => Ok(()),
            _ => Err(Error::WrongJValueType("void", self.type_name())),
        }
    }
}

impl<'a, T: Into<JObject<'a>>> From<T> for JValue<'a> {
    fn from(other: T) -> Self {
        JValue::Object(other.into())
    }
}

impl<'a> TryFrom<JValue<'a>> for JObject<'a> {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Object(o) => Ok(o),
            _ => Err(Error::WrongJValueType("object", value.type_name())),
        }
    }
}

impl<'a> From<bool> for JValue<'a> {
    fn from(other: bool) -> Self {
        JValue::Bool(if other { JNI_TRUE } else { JNI_FALSE })
    }
}

// jbool
impl<'a> From<jboolean> for JValue<'a> {
    fn from(other: jboolean) -> Self {
        JValue::Bool(other)
    }
}

impl<'a> TryFrom<JValue<'a>> for jboolean {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Bool(b) => Ok(b),
            _ => Err(Error::WrongJValueType("bool", value.type_name())),
        }
    }
}

// jchar
impl<'a> From<jchar> for JValue<'a> {
    fn from(other: jchar) -> Self {
        JValue::Char(other)
    }
}

impl<'a> TryFrom<JValue<'a>> for jchar {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Char(c) => Ok(c),
            _ => Err(Error::WrongJValueType("char", value.type_name())),
        }
    }
}

// jshort
impl<'a> From<jshort> for JValue<'a> {
    fn from(other: jshort) -> Self {
        JValue::Short(other)
    }
}

impl<'a> TryFrom<JValue<'a>> for jshort {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Short(s) => Ok(s),
            _ => Err(Error::WrongJValueType("short", value.type_name())),
        }
    }
}

// jfloat
impl<'a> From<jfloat> for JValue<'a> {
    fn from(other: jfloat) -> Self {
        JValue::Float(other)
    }
}

impl<'a> TryFrom<JValue<'a>> for jfloat {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Float(f) => Ok(f),
            _ => Err(Error::WrongJValueType("float", value.type_name())),
        }
    }
}

// jdouble
impl<'a> From<jdouble> for JValue<'a> {
    fn from(other: jdouble) -> Self {
        JValue::Double(other)
    }
}

impl<'a> TryFrom<JValue<'a>> for jdouble {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Double(d) => Ok(d),
            _ => Err(Error::WrongJValueType("double", value.type_name())),
        }
    }
}

// jint
impl<'a> From<jint> for JValue<'a> {
    fn from(other: jint) -> Self {
        JValue::Int(other)
    }
}

impl<'a> TryFrom<JValue<'a>> for jint {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Int(i) => Ok(i),
            _ => Err(Error::WrongJValueType("int", value.type_name())),
        }
    }
}

// jlong
impl<'a> From<jlong> for JValue<'a> {
    fn from(other: jlong) -> Self {
        JValue::Long(other)
    }
}

impl<'a> TryFrom<JValue<'a>> for jlong {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Long(l) => Ok(l),
            _ => Err(Error::WrongJValueType("long", value.type_name())),
        }
    }
}

// jbyte
impl<'a> From<jbyte> for JValue<'a> {
    fn from(other: jbyte) -> Self {
        JValue::Byte(other)
    }
}

impl<'a> TryFrom<JValue<'a>> for jbyte {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Byte(b) => Ok(b),
            _ => Err(Error::WrongJValueType("byte", value.type_name())),
        }
    }
}

// jvoid
impl<'a> From<()> for JValue<'a> {
    fn from(_: ()) -> Self {
        JValue::Void
    }
}

impl<'a> TryFrom<JValue<'a>> for () {
    type Error = Error;

    fn try_from(value: JValue<'a>) -> Result<Self> {
        match value {
            JValue::Void => Ok(()),
            _ => Err(Error::WrongJValueType("void", value.type_name())),
        }
    }
}
