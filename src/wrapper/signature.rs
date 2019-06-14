use combine::{stream::state::State, *};
use errors::*;
use std::str::FromStr;

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

impl ::std::fmt::Display for Primitive {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
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
    Method(Box<TypeSignature>),
}

impl FromStr for JavaType {
    type Err = String;

    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        parser(parse_type)
            .parse(State::new(s))
            .map(|res| res.0)
            .map_err(|e| format_error_message(&e, s))
    }
}

impl ::std::fmt::Display for JavaType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            JavaType::Primitive(ref ty) => ty.fmt(f),
            JavaType::Object(ref name) => write!(f, "L{};", name),
            JavaType::Array(ref ty) => write!(f, "[{}", ty),
            JavaType::Method(ref m) => m.fmt(f),
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
    pub ret: JavaType,
}

impl TypeSignature {
    /// Parse a signature string into a TypeSignature enum.
    ///
    /// Clippy suggests implementing `FromStr` or renaming it which is not possible in our case.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<TypeSignature> {
        Ok(
            match parser(parse_sig)
                .parse(State::new(s.as_ref()))
                .map(|res| res.0)
            {
                Ok(JavaType::Method(sig)) => *sig,
                Err(e) => return Err(format_error_message(&e, s.as_ref()).into()),
                _ => unreachable!(),
            },
        )
    }
}

impl ::std::fmt::Display for TypeSignature {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "(")?;
        for a in &self.args {
            write!(f, "{}", a)?;
        }
        write!(f, ")")?;
        write!(f, "{}", self.ret)?;
        Ok(())
    }
}

fn parse_primitive<S: Stream<Item = char>>(input: &mut S) -> ParseResult<JavaType, S>
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
    .map(JavaType::Primitive)
    .parse_stream(input)
}

fn parse_array<S: Stream<Item = char>>(input: &mut S) -> ParseResult<JavaType, S>
where
    S::Error: ParseError<char, S::Range, S::Position>,
{
    let marker = token('[');
    (marker, parser(parse_type))
        .map(|(_, ty)| JavaType::Array(Box::new(ty)))
        .parse_stream(input)
}

fn parse_object<S: Stream<Item = char>>(input: &mut S) -> ParseResult<JavaType, S>
where
    S::Error: ParseError<char, S::Range, S::Position>,
{
    let marker = token('L');
    let end = token(';');
    let obj = between(marker, end, many1(satisfy(|c| c != ';')));

    obj.map(JavaType::Object).parse_stream(input)
}

fn parse_type<S: Stream<Item = char>>(input: &mut S) -> ParseResult<JavaType, S>
where
    S::Error: ParseError<char, S::Range, S::Position>,
{
    parser(parse_primitive)
        .or(parser(parse_array))
        .or(parser(parse_object))
        .or(parser(parse_sig))
        .parse_stream(input)
}

fn parse_args<S: Stream<Item = char>>(input: &mut S) -> ParseResult<Vec<JavaType>, S>
where
    S::Error: ParseError<char, S::Range, S::Position>,
{
    between(token('('), token(')'), many(parser(parse_type))).parse_stream(input)
}

fn parse_sig<S: Stream<Item = char>>(input: &mut S) -> ParseResult<JavaType, S>
where
    S::Error: ParseError<char, S::Range, S::Position>,
{
    (parser(parse_args), parser(parse_type))
        .map(|(a, r)| TypeSignature { args: a, ret: r })
        .map(|sig| JavaType::Method(Box::new(sig)))
        .parse_stream(input)
}

fn format_error_message<E: ::std::fmt::Display>(err: &E, input_string: &str) -> String {
    format!("{}Input: {}", err, input_string)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parser() {
        let inputs = [
            "(Ljava/lang/String;I)V",
            "[Lherp;",
            "(IBVZ)Ljava/lang/String;",
        ];

        for each in inputs.iter() {
            let res = JavaType::from_str(*each).unwrap();
            println!("{:#?}", res);
            let s = format!("{}", res);
            assert_eq!(s, *each);
            let res2 = JavaType::from_str(*each).unwrap();
            println!("{:#?}", res2);
            assert_eq!(res2, res);
        }
    }

    #[test]
    fn test_parser_invalid_signature() {
        let signature = "()Ljava/lang/List"; // no semicolon
        let res = JavaType::from_str(signature);

        match res {
            Ok(any) => {
                panic!("Unexpected result: {}", any);
            }
            Err(err) => {
                let error_message = err.to_string();
                assert!(error_message.contains("Input: ()Ljava/lang/List"));
            }
        }
    }
}
