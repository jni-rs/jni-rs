use std::{fmt, str::FromStr};

use combine::{
    between, many, parser, parser::range::recognize, satisfy, skip_many, skip_many1, token,
    ParseError, Parser, RangeStream, StdParseResult, Stream,
};

use crate::errors::*;

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

/// Enum representing any java type in addition to method signatures.
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum JavaType {
    Primitive(Primitive),
    Object(String),
    Array(Box<JavaType>),
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
            JavaType::Object(ref name) => write!(f, "L{name};"),
            JavaType::Array(ref ty) => write!(f, "[{ty}"),
        }
    }
}

/// Enum representing any java type that may be used as a return value
///
/// This type intentionally avoids capturing any heap allocated types (to avoid
/// allocations while making JNI method calls) and so it doesn't fully qualify
/// the object or array types with a String like `JavaType::Object` does.
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum ReturnType {
    Primitive(Primitive),
    Object,
    Array,
}

impl FromStr for ReturnType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parser(parse_return)
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

impl fmt::Display for ReturnType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ReturnType::Primitive(ref ty) => ty.fmt(f),
            ReturnType::Object => write!(f, "L;"),
            ReturnType::Array => write!(f, "["),
        }
    }
}

/// A method type signature. This is the structure representation of something
/// like `(Ljava/lang/String;)Z`. Used by the `call_(object|static)_method`
/// functions on jnienv to ensure safety.
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
        .map(|(_, ty)| JavaType::Array(Box::new(ty)))
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
        .map(|(_, name, _)| JavaType::Object(name))
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

fn parse_return<'a, S>(input: &mut S) -> StdParseResult<ReturnType, S>
where
    S: RangeStream<Token = char, Range = &'a str>,
    S::Error: ParseError<char, S::Range, S::Position>,
{
    parser(parse_primitive)
        .map(ReturnType::Primitive)
        .or(parser(parse_array).map(|_| ReturnType::Array))
        .or(parser(parse_object).map(|_| ReturnType::Object))
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
    (parser(parse_args), parser(parse_return))
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
            JavaType::Object("java/lang/String".into())
        );
        assert_eq!(
            "[I".parse::<JavaType>().unwrap(),
            JavaType::Array(Box::new(JavaType::Primitive(Primitive::Int)))
        );
        assert_eq!(
            "[Ljava/lang/String;".parse::<JavaType>().unwrap(),
            JavaType::Array(Box::new(JavaType::Object("java/lang/String".into())))
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
                args: vec![JavaType::Object("java/lang/String".into())],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "([I)I".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![JavaType::Array(Box::new(JavaType::Primitive(
                    Primitive::Int
                )))],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "([Ljava/lang/String;)I".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![JavaType::Array(Box::new(JavaType::Object(
                    "java/lang/String".into()
                )))],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "(I[Ljava/lang/String;Z)I".parse::<TypeSignature>().unwrap(),
            TypeSignature {
                args: vec![
                    JavaType::Primitive(Primitive::Int),
                    JavaType::Array(Box::new(JavaType::Object("java/lang/String".into()))),
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
