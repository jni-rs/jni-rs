use std::{fmt, str::FromStr};

use crate::{
    errors::*,
    strings::{JNIStr, JNIString},
};

#[cfg(doc)]
use crate::{Env, jni_sig};

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
        let (ty, rest) = parse_type(s)?;
        match rest {
            "" => Ok(ty),
            _ => Err(Error::ParseFailed(format!(
                "Trailing input: '{rest}' while parsing '{s}'"
            ))),
        }
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
/// This is a structured representation of a JNI method signature, such as
/// `(Ljava/lang/String;)Z`.
///
/// The decomposed types are guaranteed to match the signature string and so
/// they can be used for safe JNI calls without further validation.
///
/// Most of the time you should use the [`jni_sig!`] macro to derive method
/// signatures at compile time.
///
/// If you need to parse method signatures at runtime, use
/// [`RuntimeMethodSignature::from_str`] followed by
/// [`RuntimeMethodSignature::method_signature`].
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
    pub const fn sig(&self) -> &JNIStr {
        self.sig
    }

    /// Get the argument types
    pub const fn args(&self) -> &[JavaType] {
        self.args
    }

    /// Get the return type
    pub const fn ret(&self) -> JavaType {
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
///
/// Most of the time you should use the [`jni_sig!`] macro to derive field
/// signatures at compile time.
///
/// If you need to parse field signatures at runtime, use
/// [`RuntimeFieldSignature::from_str`] followed by
/// [`RuntimeFieldSignature::field_signature`].
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
    pub const fn sig(&self) -> &JNIStr {
        self.sig
    }

    /// Get the field type
    pub const fn ty(&self) -> JavaType {
        self.ty
    }

    /// Check if the type of this field is compatible with the given type. This
    /// is not quite an equality check on `Self::ty()` because `JavaType::Array`
    /// signatures are considered compatible with `JavaType::Object` values
    pub fn type_is_compatible_with(&self, ty: JavaType) -> bool {
        match (self.ty, ty) {
            (JavaType::Array | JavaType::Object, JavaType::Array | JavaType::Object) => true,
            (a, b) if a == b => true,
            _ => false,
        }
    }
}

/// A runtime-parsed JNI method signature.
///
/// This is a structured representation of a JNI method signature, such
/// as `(Ljava/lang/String;)Z`.
///
/// The decomposed types are guaranteed to match the signature string and so
/// they can be used for safe JNI calls without further validation.
///
/// A reference to `RuntimeSignature` can be converted into a `Signature`, which
/// is used by the JNI functions, such as `call_(object|static)_method`.
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct RuntimeMethodSignature {
    sig: JNIString,
    args: Vec<JavaType>,
    ret: JavaType,
}

impl<'sig, 'args> AsRef<MethodSignature<'sig, 'args>> for MethodSignature<'sig, 'args> {
    fn as_ref(&self) -> &MethodSignature<'sig, 'args> {
        self
    }
}

impl<'a> From<&'a RuntimeMethodSignature> for MethodSignature<'a, 'a> {
    fn from(sig: &'a RuntimeMethodSignature) -> Self {
        Self {
            sig: &sig.sig,
            args: &sig.args,
            ret: sig.ret,
        }
    }
}

impl<'sig, 'args, M> From<M> for RuntimeMethodSignature
where
    M: AsRef<MethodSignature<'sig, 'args>>,
{
    fn from(sig: M) -> Self {
        let sig = sig.as_ref();
        Self {
            sig: sig.sig().into(),
            args: sig.args().to_vec(),
            ret: sig.ret(),
        }
    }
}

impl RuntimeMethodSignature {
    /// Parse a method signature string into a RuntimeMethodSignature enum.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<RuntimeMethodSignature> {
        let input = s.as_ref();
        match parse_method_sig(input)? {
            // Note: the parser initially returns a placeholder, empty signature string,
            (RuntimeMethodSignature { sig: _, args, ret }, "") => Ok(RuntimeMethodSignature {
                sig: JNIString::new(input),
                args,
                ret,
            }),
            (RuntimeMethodSignature { .. }, tail) => Err(Error::ParseFailed(format!(
                "Trailing input: '{tail}' while parsing '{input}'"
            ))),
        }
    }

    /// Convert to a [MethodSignature] (for use with [Env] calls like [`Env::call_method`]).
    ///
    /// This is a cheap conversion, which borrows the signature string and
    /// slices the argument vector, without any allocations.
    pub fn method_signature(&self) -> MethodSignature<'_, '_> {
        self.into()
    }
}

impl FromStr for RuntimeMethodSignature {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        RuntimeMethodSignature::from_str(s)
    }
}

impl fmt::Display for RuntimeMethodSignature {
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

/// A runtime-parsed JNI field signature.
///
/// This is a structured representation of a JNI field signature, such as `[Ljava/lang/String;`.
///
/// The field type is guaranteed to match the signature string and so it can be
/// used for safe JNI calls without further validation.
///
/// A reference to `RuntimeFieldSignature` can be converted into a
/// `FieldSignature`, which is used by the JNI functions, such as
/// `(get|set)_(static)_field`.
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct RuntimeFieldSignature {
    sig: JNIString,
    ty: JavaType,
}

impl<'sig> AsRef<FieldSignature<'sig>> for FieldSignature<'sig> {
    fn as_ref(&self) -> &FieldSignature<'sig> {
        self
    }
}
impl<'a> From<&'a RuntimeFieldSignature> for FieldSignature<'a> {
    fn from(sig: &'a RuntimeFieldSignature) -> Self {
        Self {
            sig: &sig.sig,
            ty: sig.ty,
        }
    }
}

impl<'sig, F> From<F> for RuntimeFieldSignature
where
    F: AsRef<FieldSignature<'sig>>,
{
    fn from(sig: F) -> Self {
        let sig = sig.as_ref();
        Self {
            sig: sig.sig().into(),
            ty: sig.ty(),
        }
    }
}

impl RuntimeFieldSignature {
    /// Parse a field signature string into a RuntimeFieldSignature.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str<S: AsRef<str>>(s: S) -> Result<RuntimeFieldSignature> {
        let input = s.as_ref();
        match parse_field_sig(input)? {
            // Note: the parser initially returns a placeholder, empty signature string,
            (RuntimeFieldSignature { sig: _, ty }, "") => Ok(RuntimeFieldSignature {
                sig: JNIString::new(input),
                ty,
            }),
            (RuntimeFieldSignature { .. }, tail) => Err(Error::ParseFailed(format!(
                "Trailing input: '{tail}' while parsing '{input}'"
            ))),
        }
    }

    /// Convert to a [FieldSignature] (for use with [Env] calls like [`Env::get_field`]).
    ///
    /// This is a cheap conversion, which borrows the signature string and
    /// slices the argument vector, without any allocations.
    pub fn field_signature(&self) -> FieldSignature<'_> {
        self.into()
    }
}

impl FromStr for RuntimeFieldSignature {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        RuntimeFieldSignature::from_str(s)
    }
}

impl fmt::Display for RuntimeFieldSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.ty)
    }
}

/// Parse a primitive type descriptor from the start of `input`.
/// Returns the parsed `Primitive` and the remaining input.
fn parse_primitive(input: &str) -> Result<(Primitive, &str)> {
    let (&first, _rest) = input
        .as_bytes()
        .split_first()
        .ok_or_else(|| Error::ParseFailed("unexpected end of input".into()))?;

    let prim = match first {
        b'Z' => Primitive::Boolean,
        b'B' => Primitive::Byte,
        b'C' => Primitive::Char,
        b'D' => Primitive::Double,
        b'F' => Primitive::Float,
        b'I' => Primitive::Int,
        b'J' => Primitive::Long,
        b'S' => Primitive::Short,
        b'V' => Primitive::Void,
        _ => {
            return Err(Error::ParseFailed(format!(
                "expected primitive type, got '{}'",
                first as char
            )));
        }
    };

    // SAFETY: we split one ASCII byte, rest is valid UTF-8 at that offset
    Ok((prim, &input[1..]))
}

/// Parse a non-void primitive type descriptor.
fn parse_non_void_primitive(input: &str) -> Result<(Primitive, &str)> {
    let (prim, rest) = parse_primitive(input)?;
    if matches!(prim, Primitive::Void) {
        return Err(Error::ParseFailed(
            "void is not valid in this position".into(),
        ));
    }
    Ok((prim, rest))
}

/// Parse an array type descriptor (`[<element_type>`).
fn parse_array(input: &str) -> Result<(JavaType, &str)> {
    let input = input
        .strip_prefix('[')
        .ok_or_else(|| Error::ParseFailed("expected '[' for array type".into()))?;
    let (_, rest) = parse_non_void_type(input)?;
    Ok((JavaType::Array, rest))
}

/// Parse an object type descriptor (`Lclassname;`).
fn parse_object(input: &str) -> Result<(JavaType, &str)> {
    let input = input
        .strip_prefix('L')
        .ok_or_else(|| Error::ParseFailed("expected 'L' for object type".into()))?;

    // JVMS §4.2.2: unqualified names must not contain '.', ';', '[', or '/'
    fn is_unqualified(c: char) -> bool {
        !matches!(c, '.' | ';' | '[' | '/')
    }

    let bytes = input.as_bytes();
    let mut i = 0;
    // Must start with at least one unqualified char (no leading '/')
    if i >= bytes.len() || !is_unqualified(bytes[i] as char) {
        return Err(Error::ParseFailed("expected class name after 'L'".into()));
    }

    while i < bytes.len() && is_unqualified(bytes[i] as char) {
        i += 1;
    }

    // Optionally followed by ('/' segment)* where each segment is non-empty
    while i < bytes.len() && bytes[i] == b'/' {
        i += 1; // consume '/'
        let seg_start = i;
        while i < bytes.len() && is_unqualified(bytes[i] as char) {
            i += 1;
        }

        if i == seg_start {
            return Err(Error::ParseFailed(
                "expected class name segment after '/'".into(),
            ));
        }
    }

    // Must end with ';'
    if i >= bytes.len() || bytes[i] != b';' {
        return Err(Error::ParseFailed(
            "expected ';' to close object type".into(),
        ));
    }

    Ok((JavaType::Object, &input[i + 1..]))
}

/// Parse any type descriptor (primitive, object, or array), including void.
fn parse_type(input: &str) -> Result<(JavaType, &str)> {
    match input.as_bytes().first() {
        Some(b'L') => parse_object(input),
        Some(b'[') => parse_array(input),
        Some(_) => {
            let (prim, rest) = parse_primitive(input)?;
            Ok((JavaType::Primitive(prim), rest))
        }
        None => Err(Error::ParseFailed("unexpected end of input".into())),
    }
}

/// Parse any non-void type descriptor (primitive, object, or array).
fn parse_non_void_type(input: &str) -> Result<(JavaType, &str)> {
    match input.as_bytes().first() {
        Some(b'L') => parse_object(input),
        Some(b'[') => parse_array(input),
        Some(_) => {
            let (prim, rest) = parse_non_void_primitive(input)?;
            Ok((JavaType::Primitive(prim), rest))
        }
        None => Err(Error::ParseFailed("unexpected end of input".into())),
    }
}

/// Parse argument types enclosed in parentheses (`(<type>*)`).
fn parse_args(input: &str) -> Result<(Vec<JavaType>, &str)> {
    let input = input
        .strip_prefix('(')
        .ok_or_else(|| Error::ParseFailed("expected '(' to start arguments".into()))?;

    let mut args = Vec::new();
    let mut rest = input;
    loop {
        match rest.as_bytes().first() {
            Some(b')') => return Ok((args, &rest[1..])),
            Some(_) => {
                let (ty, remaining) = parse_non_void_type(rest)?;
                args.push(ty);
                rest = remaining;
            }
            None => {
                return Err(Error::ParseFailed(
                    "unexpected end of input, expected ')'".into(),
                ));
            }
        }
    }
}

/// Note: we initially return a placeholder signature string, which is replaced later
fn parse_method_sig(input: &str) -> Result<(RuntimeMethodSignature, &str)> {
    let (args, rest) = parse_args(input)?;
    let (ret, rest) = parse_type(rest)?;
    Ok((
        RuntimeMethodSignature {
            sig: JNIString::new(""),
            args,
            ret,
        },
        rest,
    ))
}

/// Note: we initially return a placeholder signature string, which is replaced later
fn parse_field_sig(input: &str) -> Result<(RuntimeFieldSignature, &str)> {
    let (ty, rest) = parse_non_void_type(input)?;
    Ok((
        RuntimeFieldSignature {
            sig: JNIString::new(""),
            ty,
        },
        rest,
    ))
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
            "V".parse::<JavaType>().unwrap(),
            JavaType::Primitive(Primitive::Void)
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

        // Void field types are invalid
        assert_matches!("V".parse::<RuntimeFieldSignature>(), Err(_));

        // Void arrays are invalid
        assert_matches!("[V".parse::<JavaType>(), Err(_));
        assert_matches!("[V".parse::<RuntimeFieldSignature>(), Err(_));

        // Multi-dimensional arrays
        assert_eq!("[[I".parse::<JavaType>().unwrap(), JavaType::Array);
        assert_eq!("[[[I".parse::<JavaType>().unwrap(), JavaType::Array);
        assert_eq!(
            "[[Ljava/lang/String;".parse::<JavaType>().unwrap(),
            JavaType::Array
        );

        // Array with no element type
        assert_matches!("[".parse::<JavaType>(), Err(_));
        assert_matches!("[[".parse::<JavaType>(), Err(_));

        // Single-segment class name (no slashes)
        assert_eq!("LString;".parse::<JavaType>().unwrap(), JavaType::Object);
        assert_eq!("LA;".parse::<JavaType>().unwrap(), JavaType::Object);

        // Deeply nested class path
        assert_eq!("La/b/c/d/e;".parse::<JavaType>().unwrap(), JavaType::Object);

        // Trailing slash in class name
        assert_matches!("Ljava/lang/String/;".parse::<JavaType>(), Err(_));
        assert_matches!("LString/;".parse::<JavaType>(), Err(_));

        // Double slash in class name
        assert_matches!("Ljava//lang;".parse::<JavaType>(), Err(_));

        // Dots in class name (Java-style, not JVM-style)
        assert_matches!("Ljava.lang.String;".parse::<JavaType>(), Err(_));

        // '[' in class name
        assert_matches!("Ljava[lang;".parse::<JavaType>(), Err(_));
    }

    #[test]
    fn test_parser_field_signatures() {
        // Primitive field signatures
        assert_eq!(
            "I".parse::<RuntimeFieldSignature>().unwrap(),
            RuntimeFieldSignature {
                sig: JNIString::new("I"),
                ty: JavaType::Primitive(Primitive::Int),
            }
        );
        assert_eq!(
            "Z".parse::<RuntimeFieldSignature>().unwrap(),
            RuntimeFieldSignature {
                sig: JNIString::new("Z"),
                ty: JavaType::Primitive(Primitive::Boolean),
            }
        );

        // Object field signature
        assert_eq!(
            "Ljava/lang/String;"
                .parse::<RuntimeFieldSignature>()
                .unwrap(),
            RuntimeFieldSignature {
                sig: JNIString::new("Ljava/lang/String;"),
                ty: JavaType::Object,
            }
        );

        // Array field signatures
        assert_eq!(
            "[I".parse::<RuntimeFieldSignature>().unwrap(),
            RuntimeFieldSignature {
                sig: JNIString::new("[I"),
                ty: JavaType::Array,
            }
        );
        assert_eq!(
            "[Ljava/lang/String;"
                .parse::<RuntimeFieldSignature>()
                .unwrap(),
            RuntimeFieldSignature {
                sig: JNIString::new("[Ljava/lang/String;"),
                ty: JavaType::Array,
            }
        );
        assert_eq!(
            "[[I".parse::<RuntimeFieldSignature>().unwrap(),
            RuntimeFieldSignature {
                sig: JNIString::new("[[I"),
                ty: JavaType::Array,
            }
        );

        // Invalid field signatures
        assert_matches!("".parse::<RuntimeFieldSignature>(), Err(_));
        assert_matches!("V".parse::<RuntimeFieldSignature>(), Err(_));
        assert_matches!("[V".parse::<RuntimeFieldSignature>(), Err(_));
        assert_matches!("IZ".parse::<RuntimeFieldSignature>(), Err(_));
        assert_matches!(
            "Ljava/lang/String;I".parse::<RuntimeFieldSignature>(),
            Err(_)
        );
        assert_matches!("X".parse::<RuntimeFieldSignature>(), Err(_));
    }

    #[test]
    fn test_parser_signatures() {
        assert_eq!(
            "()V".parse::<RuntimeMethodSignature>().unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("()V"),
                args: vec![],
                ret: ReturnType::Primitive(Primitive::Void)
            }
        );
        assert_eq!(
            "(I)V".parse::<RuntimeMethodSignature>().unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("(I)V"),
                args: vec![JavaType::Primitive(Primitive::Int)],
                ret: ReturnType::Primitive(Primitive::Void)
            }
        );
        assert_eq!(
            "(Ljava/lang/String;)I"
                .parse::<RuntimeMethodSignature>()
                .unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("(Ljava/lang/String;)I"),
                args: vec![JavaType::Object],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "([I)I".parse::<RuntimeMethodSignature>().unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("([I)I"),
                args: vec![JavaType::Array],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "([Ljava/lang/String;)I"
                .parse::<RuntimeMethodSignature>()
                .unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("([Ljava/lang/String;)I"),
                args: vec![JavaType::Array],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );
        assert_eq!(
            "(I[Ljava/lang/String;Z)I"
                .parse::<RuntimeMethodSignature>()
                .unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("(I[Ljava/lang/String;Z)I"),
                args: vec![
                    JavaType::Primitive(Primitive::Int),
                    JavaType::Array,
                    JavaType::Primitive(Primitive::Boolean),
                ],
                ret: ReturnType::Primitive(Primitive::Int)
            }
        );

        assert_matches!("".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("()".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("V".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(I".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("I)I".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(I)".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(Invalid)I".parse::<RuntimeMethodSignature>(), Err(_));
        // We shouldn't recursively allow method signatures as method argument types (#597)
        assert_matches!("((()I)I)I".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(I)V ".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("()java/lang/List".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!(
            "(L/java/lang/String)V".parse::<RuntimeMethodSignature>(),
            Err(_)
        );

        // Void arrays are invalid in method arguments
        assert_matches!("([V)V".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(I[V)V".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("([VI)V".parse::<RuntimeMethodSignature>(), Err(_));
        // Void arrays are invalid as return types
        assert_matches!("()[V".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(I)[V".parse::<RuntimeMethodSignature>(), Err(_));

        // Void as method argument is invalid
        assert_matches!("(V)I".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(V)V".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(IV)I".parse::<RuntimeMethodSignature>(), Err(_));
        assert_matches!("(VI)I".parse::<RuntimeMethodSignature>(), Err(_));

        // Object return type
        assert_eq!(
            "()Ljava/lang/String;"
                .parse::<RuntimeMethodSignature>()
                .unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("()Ljava/lang/String;"),
                args: vec![],
                ret: ReturnType::Object
            }
        );

        // Array return type
        assert_eq!(
            "()[I".parse::<RuntimeMethodSignature>().unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("()[I"),
                args: vec![],
                ret: ReturnType::Array
            }
        );
        assert_eq!(
            "()[Ljava/lang/String;"
                .parse::<RuntimeMethodSignature>()
                .unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("()[Ljava/lang/String;"),
                args: vec![],
                ret: ReturnType::Array
            }
        );

        // Multiple object arguments
        assert_eq!(
            "(Ljava/lang/String;Ljava/lang/Object;)V"
                .parse::<RuntimeMethodSignature>()
                .unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("(Ljava/lang/String;Ljava/lang/Object;)V"),
                args: vec![JavaType::Object, JavaType::Object],
                ret: ReturnType::Primitive(Primitive::Void)
            }
        );

        // Array arguments
        assert_eq!(
            "([[I[Ljava/lang/String;)V"
                .parse::<RuntimeMethodSignature>()
                .unwrap(),
            RuntimeMethodSignature {
                sig: JNIString::new("([[I[Ljava/lang/String;)V"),
                args: vec![JavaType::Array, JavaType::Array],
                ret: ReturnType::Primitive(Primitive::Void)
            }
        );
    }
}
