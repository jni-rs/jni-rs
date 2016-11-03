use errors::*;
use combine::*;

#[derive(Eq, PartialEq, Debug)]
pub enum Primitive {
    Boolean, // Z
    Byte, // B
    Char, // C
    Double, // D
    Float, // F
    Int, // I
    Long, // J
    Short, // S
    Void, // V
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

#[derive(Eq, PartialEq, Debug)]
pub enum JavaType {
    Primitive(Primitive),
    Object(String),
    Array(Box<JavaType>),
    Method(Box<TypeSignature>),
}

impl JavaType {
    pub fn from_str(s: &str) -> Result<JavaType> {
        Ok(match parser(parse_type).parse(s).map(|res| res.0) {
            Ok(sig) => sig,
            Err(e) => return Err(format!("{}", e).into()),
        })
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

#[derive(Eq, PartialEq, Debug)]
pub struct TypeSignature {
    pub args: Vec<JavaType>,
    pub ret: JavaType,
}

impl TypeSignature {
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<TypeSignature> {
        Ok(match parser(parse_sig).parse(s.as_ref()).map(|res| res.0) {
            Ok(JavaType::Method(sig)) => *sig,
            Err(e) => return Err(format!("{}", e).into()),
            _ => unreachable!(),
        })
    }
}

impl ::std::fmt::Display for TypeSignature {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "(")?;
        for a in self.args.iter() {
            write!(f, "{}", a)?;
        }
        write!(f, ")")?;
        write!(f, "{}", self.ret)?;
        Ok(())
    }
}

fn parse_primitive<S: Stream<Item = char>>(input: S)
                                           -> ParseResult<JavaType, S> {
    let boolean = token('Z').map(|_| Primitive::Boolean);
    let byte = token('B').map(|_| Primitive::Byte);
    let char_type = token('C').map(|_| Primitive::Char);
    let double = token('D').map(|_| Primitive::Double);
    let float = token('F').map(|_| Primitive::Float);
    let int = token('I').map(|_| Primitive::Int);
    let long = token('J').map(|_| Primitive::Long);
    let short = token('S').map(|_| Primitive::Short);
    let void = token('V').map(|_| Primitive::Void);

    (boolean.or(byte)
            .or(char_type)
            .or(double)
            .or(float)
            .or(int)
            .or(long)
            .or(short)
            .or(void))
        .map(|ty| JavaType::Primitive(ty))
        .parse_stream(input)
}

fn parse_array<S: Stream<Item = char>>(input: S) -> ParseResult<JavaType, S> {
    let marker = token('[');
    (marker, parser(parse_type))
        .map(|(_, ty)| JavaType::Array(Box::new(ty)))
        .parse_stream(input)
}

fn parse_object<S: Stream<Item = char>>(input: S) -> ParseResult<JavaType, S> {
    let marker = token('L');
    let end = token(';');
    let obj = between(marker, end, many1(satisfy(|c| c != ';')));

    obj.map(|name| JavaType::Object(name)).parse_stream(input)
}

fn parse_type<S: Stream<Item = char>>(input: S) -> ParseResult<JavaType, S> {
    parser(parse_primitive)
        .or(parser(parse_array))
        .or(parser(parse_object))
        .or(parser(parse_sig))
        .parse_stream(input)
}

fn parse_args<S: Stream<Item = char>>(input: S)
                                      -> ParseResult<Vec<JavaType>, S> {
    between(token('('), token(')'), many(parser(parse_type)))
        .parse_stream(input)
}

fn parse_sig<S: Stream<Item = char>>(input: S) -> ParseResult<JavaType, S> {
    (parser(parse_args), parser(parse_type))
        .map(|(a, r)| TypeSignature { args: a, ret: r })
        .map(|sig| JavaType::Method(Box::new(sig)))
        .parse_stream(input)
}


#[cfg(test)]
mod test {
    use combine::*;
    use super::*;

    #[test]
    fn test_parser() {
        let inputs =
            ["(Ljava/lang/String;I)V", "[Lherp;", "(IBVZ)Ljava/lang/String;"];
        for each in inputs.iter() {
            let mut res = JavaType::from_str(*each);
            println!("{:#?}", res);
            let out = res.unwrap();
            let s = format!("{}", out);
            assert_eq!(s, *each);
            res = JavaType::from_str(*each);
            println!("{:#?}", res);
            assert_eq!(res.unwrap(), out);
        }
    }
}
