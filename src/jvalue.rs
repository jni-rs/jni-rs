use errors::*;
use std::mem::transmute;
use jobject::JObject;
use jni_sys::*;

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

impl<'a> JValue<'a> {
    pub unsafe fn to_jni(self) -> jvalue {
        let val: jvalue = match self {
            JValue::Object(obj) => transmute(obj.into_inner()),
            JValue::Byte(byte) => transmute(byte as i64),
            JValue::Char(char) => transmute(char as u64),
            JValue::Short(short) => transmute(short as i64),
            JValue::Int(int) => transmute(int as i64),
            JValue::Long(long) => transmute(long),
            JValue::Bool(boolean) => transmute(boolean as u64),
            JValue::Float(float) => transmute(float as f64),
            JValue::Double(double) => transmute(double),
            JValue::Void => Default::default(),
        };
        trace!("converted {:?} to jvalue {:?}", self, val);
        val
    }

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

    pub fn l(self) -> Result<JObject<'a>> {
        match self {
            JValue::Object(obj) => Ok(obj),
            _ => {
                Err(ErrorKind::WrongJValueType("object", self.type_name())
                    .into())
            }
        }
    }

    pub fn z(self) -> Result<bool> {
        match self {
            JValue::Bool(b) => Ok(b != 0),
            _ => {
                Err(ErrorKind::WrongJValueType("bool", self.type_name()).into())
            }
        }
    }
}

impl<'a> From<JObject<'a>> for JValue<'a> {
    fn from(other: JObject<'a>) -> Self {
        JValue::Object(other)
    }
}

// jbool
impl<'a> From<bool> for JValue<'a> {
    fn from(other: bool) -> Self {
        JValue::Bool(other as jboolean)
    }
}

// jchar
impl<'a> From<jchar> for JValue<'a> {
    fn from(other: jchar) -> Self {
        JValue::Char(other)
    }
}

// jshort
impl<'a> From<jshort> for JValue<'a> {
    fn from(other: jshort) -> Self {
        JValue::Short(other)
    }
}

// jfloat
impl<'a> From<jfloat> for JValue<'a> {
    fn from(other: jfloat) -> Self {
        JValue::Float(other)
    }
}

// jdouble
impl<'a> From<jdouble> for JValue<'a> {
    fn from(other: jdouble) -> Self {
        JValue::Double(other)
    }
}

// jint
impl<'a> From<jint> for JValue<'a> {
    fn from(other: jint) -> Self {
        JValue::Int(other)
    }
}

// jlong
impl<'a> From<jlong> for JValue<'a> {
    fn from(other: jlong) -> Self {
        JValue::Long(other)
    }
}

// jbyte
impl<'a> From<jbyte> for JValue<'a> {
    fn from(other: jbyte) -> Self {
        JValue::Byte(other)
    }
}

// jvoid
impl<'a> From<()> for JValue<'a> {
    fn from(other: ()) -> Self {
        JValue::Void
    }
}
