use std::{fmt, str::FromStr};

use combine::{
    between, many, parser, parser::range::recognize, satisfy, skip_many, skip_many1, token,
    ParseError, Parser, RangeStream, StdParseResult, Stream,
};

use crate::{errors::*, strings::JNIStr};

/// A primitive java type. These are the things that can be represented without
/// an object.
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum Primitive {
    Boolean, // Z
    Byte,    // B
    Char,    // C
    Double,  // D
    Float,   // F
    Int,     // I
    Long,    // J
    Short,   // S
    Void,    // V
}

impl fmt::Display for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Primitive::Boolean => write!(f, "Z"),
            Primitive::Byte => write!(f, "B"),
            Primitive::Char => write!(f, "C"),
            Primitive::Double => write!(f, "D"),
            Primitive::Float => write!(f, "F"),
            Primitive::Int => write!(f, "I"),
            Primitive::Long => write!(f, "J"),
            Primitive::Short => write!(f, "S"),
            Primitive::Void => write!(f, "V"),
        }
    }
}

/// Enum representing any java type
///
/// This intentionally does not keep track of the object class names or details of array elements
/// since there would be a cost to tracking those strings and handling variable array dimensions
/// while JNI generally only needs to differentiate between primitive types and reference types.
///
/// In the past this did use to track object names and array details, but it proved to have a
/// significant hidden cost that was redundant while those details were never used (at least
/// internally).
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub enum JavaType {
    Primitive(Primitive),
    Object,
    Array,
}

impl FromStr for JavaType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parser(parse_type)
            .parse(s)
            .map_err(|e| Error::ParseFailed(format!("Failed to parse '{s}': {e}")))
            .map(|(res, tail)| {
                if tail.is_empty() {
                    Ok(res)
                } else {
                    Err(Error::ParseFailed(format!(
                        "Trailing input: '{tail}' while parsing '{s}'"
                    )))
                }
            })?
    }
}

impl fmt::Display for JavaType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            JavaType::Primitive(ref ty) => ty.fmt(f),
            JavaType::Object => write!(f, "L;"),
            JavaType::Array => write!(f, "["),
        }
    }
}

/// Enum representing any java type that may be used as a return value
pub type ReturnType = JavaType;

/// A parsed JNI method signature
///
/// This is a structured representation of a JNI method signature, such
/// as `(Ljava/lang/String;)Z`.
///
/// The decomposed types are guaranteed to match the signature string and so
/// they can be used for safe JNI calls without further validation.
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct MethodSignature<'sig, 'args> {
    sig: &'sig JNIStr,
    args: &'args [JavaType],
    ret: JavaType,
}

impl<'sig, 'args> MethodSignature<'sig, 'args> {
    /// Create a `MethodSignature` from its raw parts
    ///
    /// # Safety
    ///
    /// In order for the returned `MethodSignature` to be used safely to make
    /// JNI calls, the caller must ensure that the provided signature string,
    /// argument types, and return type are consistent
    pub const unsafe fn from_raw_parts(
        sig: &'sig JNIStr,
        args: &'args [JavaType],
        ret: ReturnType,
    ) -> Self {
        Self { sig, args, ret }
    }

    /// Get the JNI signature string
    pub fn sig(&self) -> &JNIStr {
        self.sig
    }

    /// Get the argument types
    pub fn args(&self) -> &[JavaType] {
        self.args
    }

    /// Get the return type
    pub fn ret(&self) -> JavaType {
        self.ret
    }
}

impl<'sig, 'args> From<&MethodSignature<'sig, 'args>> for MethodSignature<'sig, 'args> {
    fn from(sig: &MethodSignature<'sig, 'args>) -> Self {
        sig.clone()
    }
}

/// A parsed JNI field signature
///
/// This is a structured representation of a JNI field signature, such
/// as `I`.
///
/// The field type is guaranteed to match the signature string and so
/// it can be used for safe JNI calls without further validation.
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct FieldSignature<'sig> {
    sig: &'sig JNIStr,
    ty: JavaType,
}

impl<'sig> FieldSignature<'sig> {
    /// Create a `FieldSignature` from its raw parts
    ///
    /// # Safety
    ///
    /// In order for the returned `FieldSignature` to be used safely to get or
    /// set fields via JNI calls, the caller must ensure that the provided
    /// signature string and field type are consistent
    pub const unsafe fn from_raw_parts(sig: &'sig JNIStr, ty: JavaType) -> Self {
        Self { sig, ty }
    }

    /// Get the JNI signature string
    pub fn sig(&self) -> &JNIStr {
        self.sig
    }

    /// Get the field type
    pub fn ty(&self) -> JavaType {
        self.ty
    }
}

/// A runtime-parsed JNI method signature.
///
/// This is a structured representation of a JNI method signature, such as
/// `(Ljava/lang/String;)Z`.
///
/// The decomposed types are guaranteed to match the signature string and so
/// they can be used for safe JNI calls without further validation.
///
/// Used by the `call_(object|static)_method` functions on jnienv to ensure
/// safety.
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct TypeSignature {
    pub args: Vec<JavaType>,
    pub ret: ReturnType,
}

impl TypeSignature {
    /// Parse a signature string into a TypeSignature enum.
    // Clippy suggests implementing `FromStr` or renaming it which is not possible in our case.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<TypeSignature> {
        parser(parse_sig)
            .parse(s.as_ref())
            .map_err(|e| Error::ParseFailed(format!("Failed to parse '{}': {e}", s.as_ref())))
            .map(|(sig, tail)| {
                if tail.is_empty() {
                    Ok(sig)
                } else {
                    Err(Error::ParseFailed(format!(
                        "Trailing input: '{tail}' while parsing '{}'",
                        s.as_ref()
                    )))
                }
            })?
    }
}

impl FromStr for TypeSignature {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        TypeSignature::from_str(s)
    }
}

impl fmt::Display for TypeSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(")?;
        for a in &self.args {
            write!(f, "{a}")?;
        }
        write!(f, ")")?;
        write!(f, "{}", self.ret)?;
        Ok(())
    }
}

fn parse_primitive<S: Stream<Token = char>>(input: &mut S) -> StdParseResult<Primitive, S>
where
    S::Error: ParseError<char, S::Range, S::Position>,
{
    let boolean = token('Z').map(|_| Primitive::Boolean);
    let byte = token('B').map(|_| Primitive::Byte);
    let char_type = token('C').map(|_| Primitive::Char);
    let double = token('D').map(|_| Primitive::Double);
    let float = token('F').map(|_| Primitive::Float);
    let int = token('I').map(|_| Primitive::Int);
    let long = token('J').map(|_| Primitive::Long);
    let short = token('S').map(|_| Primitive::Short);
    let void = token('V').map(|_| Primitive::Void);

    (boolean
        .or(byte)
        .or(char_type)
        .or(double)
        .or(float)
        .or(int)
        .or(long)
        .or(short)
        .or(void))
    .parse_stream(input)
    .into()
}

fn parse_array<'a, S>(input: &mut S) -> StdParseResult<JavaType, S>
where
    S: RangeStream<Token = char, Range = &'a str>,
    S::Error: ParseError<char, S::Range, S::Position>,
{
    let marker = token('[');
    (marker, parser(parse_type))
        .map(|(_, _ty)| JavaType::Array)
        .parse_stream(input)
        .into()
}

fn parse_object<'a, S>(input: &mut S) -> StdParseResult<JavaType, S>
where
    S: RangeStream<Token = char, Range = &'a str>,
    S::Error: ParseError<char, &'a str, S::Position>,
{
    fn is_unqualified(c: char) -> bool {
        // JVMS §4.2.2: '.', ';', '[' and '/' are disallowed in an unqualified name
        !matches!(c, '.' | ';' | '[' | '/')
    }

    // One or more segments separated by '/', never starting or ending with '/'
    let class_body = recognize((
        skip_many1(satisfy(is_unqualified)),
        skip_many(token('/').with(skip_many1(satisfy(is_unqualified)))),
    ));

    (
        token('L'),
        class_body.map(|s: &'a str| s.to_owned()),
        token(';'),
    )
        .map(|(_, _name, _)| JavaType::Object)
        .parse_stream(input)
        .into()
}

fn parse_type<'a, S>(input: &mut S) -> StdParseResult<JavaType, S>
where
    S: RangeStream<Token = char, Range = &'a str>,
    S::Error: ParseError<char, &'a str, S::Position>,
{
    parser(parse_primitive)
        .map(JavaType::Primitive)
        .or(parser(parse_array))
        .or(parser(parse_object))
        .parse_stream(input)
        .into()
}

fn parse_args<'a, S>(input: &mut S) -> StdParseResult<Vec<JavaType>, S>
where
    S: RangeStream<Token = char, Range = &'a str>,
    S::Error: ParseError<char, S::Range, S::Position>,
{
    between(token('('), token(')'), many(parser(parse_type)))
        .parse_stream(input)
        .into()
}

fn parse_sig<'a, S>(input: &mut S) -> StdParseResult<TypeSignature, S>
where
    S: RangeStream<Token = char, Range = &'a str>,
    S::Error: ParseError<char, S::Range, S::Position>,
{
    (parser(parse_args), parser(parse_type))
        .map(|(a, r)| TypeSignature { args: a, ret: r })
        .parse_stream(input)
        .into()
}

#[cfg(test)]
mod test {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn test_parser_types() {
        assert_eq!(
            "Z".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Boolean)
        );
        assert_eq!(
            "B".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Byte)
        );
        assert_eq!(
            "C".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Char)
        );
        assert_eq!(
            "S".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Short)
        );
        assert_eq!(
            "I".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Int)
        );
        assert_eq!(
            "J".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Long)
        );
        assert_eq!(
            "F".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Float)
        );
        assert_eq!(
            "D".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Double)
        );
        assert_eq!(
            "Ljava/lang/String;".parse::<JavaType>().unwrap(),
            JavaType::Object
        );
        assert_eq!("[I".parse::<JavaType>().unwrap(), JavaType::Array);
        assert_eq!(
            "[Ljava/lang/String;".parse::<JavaType>().unwrap(),
            JavaType::Array
        );

        assert_matches!("".parse::<JavaType>(), Err(_));
        assert_matches!("A".parse::<JavaType>(), Err(_));
        // The parser should return an error if the entire input is not consumed (#598)
        assert_matches!("Invalid".parse::<JavaType>(), Err(_));
        assert_matches!("II".parse::<JavaType>(), Err(_));
        assert_matches!("java/lang/String".parse::<JavaType>(), Err(_));
        assert_matches!("Ljava/lang/String".parse::<JavaType>(), Err(_));
        assert_matches!("java/lang/String;".parse::<JavaType>(), Err(_));
        // Don't allow leading '/' in class names (#212)
        assert_matches!("L/java/lang/String;".parse::<JavaType>(), Err(_));
        assert_matches!("L/;".parse::<JavaType>(), Err(_));
        assert_matches!("L;".parse::<JavaType>(), Err(_));
    }

    #[test]
    fn test_parser_signatures() {
        assert_eq!(
            "()V".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![],
                ret: ReturnType::Primitive(Primitive::Void)
            }
        );
        assert_eq!(
            "(I)V".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![JavaType::Primitive(Primitive::Int)],
                ret: ReturnType::Primitive(Primitive::Void)
            }
        );
        assert_eq!(
            "(Ljava/lang/String;)I".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![JavaType::Object],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "([I)I".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![JavaType::Array],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "([Ljava/lang/String;)I".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![JavaType::Array],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "(I[Ljava/lang/String;Z)I".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![
                    JavaType::Primitive(Primitive::Int),
                    JavaType::Array,
                    JavaType::Primitive(Primitive::Boolean),
                ],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );

        assert_matches!("".parse::<TypeSignature>(), Err(_));
        assert_matches!("()".parse::<TypeSignature>(), Err(_));
        assert_matches!("V".parse::<TypeSignature>(), Err(_));
        assert_matches!("(I".parse::<TypeSignature>(), Err(_));
        assert_matches!("I)I".parse::<TypeSignature>(), Err(_));
        assert_matches!("(I)".parse::<TypeSignature>(), Err(_));
        assert_matches!("(Invalid)I".parse::<TypeSignature>(), Err(_));
        // We shouldn't recursively allow method signatures as method argument types (#597)
        assert_matches!("((()I)I)I".parse::<TypeSignature>(), Err(_));
        assert_matches!("(I)V ".parse::<TypeSignature>(), Err(_));
        assert_matches!("()java/lang/List".parse::<TypeSignature>(), Err(_));
        assert_matches!("(L/java/lang/String)V".parse::<TypeSignature>(), Err(_));
    }
}
