mod mangle;
mod signature;
mod str;
mod types;
mod utils;

/// Converts UTF-8 string literals to a MUTF-8 encoded `&'static JNIStr`.
///
/// This macro takes one or more literals and encodes them using Java's Modified UTF-8
/// (MUTF-8) format, returning a `&'static JNIStr`.
///
/// Like the `concat!` macro, multiple literals can be provided and will be converted to
/// strings and concatenated before encoding.
///
/// Supported literal types:
/// - String literals (`"..."`)
/// - Character literals (`'c'`)
/// - Integer literals (`42`, `-10`)
/// - Float literals (`3.14`, `1.0`)
/// - Boolean literals (`true`, `false`)
/// - Byte literals (`b'A'` - formatted as numeric value)
/// - C-string literals (`c"..."` - must be valid UTF-8)
///
/// MUTF-8 is Java's variant of UTF-8 that:
/// - Encodes the null character (U+0000) as `0xC0 0x80` instead of `0x00`
/// - Encodes Unicode characters above U+FFFF using CESU-8 (surrogate pairs)
///
/// This is the most type-safe way to create JNI string literals, as it returns a
/// `JNIStr` which is directly compatible with the jni crate's API.
///
/// # Syntax
///
/// ```
/// # use jni::jni_str;
/// # extern crate jni as jni2;
/// jni_str!("string literal");
/// jni_str!("part1", "part2", "part3");  // Concatenates before encoding
/// jni_str!("value: ", 42);               // Mix different literal types
/// jni_str!(jni = jni2, "string literal");  // Override jni crate path (must be first)
/// ```
///
/// # Examples
///
/// ```
/// use jni::{jni_str, strings::JNIStr};
///
/// const CLASS_NAME: &JNIStr = jni_str!("java.lang.String");
/// // Result: &'static JNIStr for "java.lang.String" (MUTF-8 encoded)
///
/// const EMOJI_CLASS: &JNIStr = jni_str!("unicode.Type😀");
/// // Result: &'static JNIStr with emoji encoded as surrogate pair
///
/// const PACKAGE_CLASS: &JNIStr = jni_str!("java.lang.", "String");
/// // Result: &'static JNIStr for "java.lang.String" (concatenated then MUTF-8 encoded)
///
/// const PORT: &JNIStr = jni_str!("localhost:", 8080);
/// // Result: &'static JNIStr for "localhost:8080" (mixed literal types concatenated)
/// ```
#[proc_macro]
pub fn jni_str(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    str::jni_str_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Converts UTF-8 string literals to a MUTF-8 encoded CStr literal.
///
/// This macro is equivalent to [`jni_str!`] but returns a `&CStr` instead of a `&'static JNIStr`.
///
/// See the [`jni_str!`] macro documentation for detailed syntax and examples.
#[proc_macro]
pub fn jni_cstr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    str::jni_cstr_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Parses a JNI method or field signature at compile time.
///
/// This macro parses method and field signatures with syntax like `(arg0: JString, arg1: jint) ->
/// JString` and generates a [MethodSignature] or [FieldSignature] struct to represent the
/// corresponding JNI signature, including the raw string like
/// "(Ljava/lang/String;I)Ljava/lang/String;" and enumerated argument plus return types.
///
/// This macro can also parse raw JNI signature strings like `"(Ljava/lang/String;I)Z"` in order to
/// validate them at compile time but it's recommended to use the structured syntax for better
/// readability.
///
/// [MethodSignature]: https://docs.rs/jni/latest/jni/signature/struct.MethodSignature.html
/// [FieldSignature]: https://docs.rs/jni/latest/jni/signature/struct.FieldSignature.html
///
/// # Syntax
///
/// The macro accepts named properties separated by commas:
/// ```ignore
/// jni_sig!(
///     [jni = <path>],
///     [type_map = { ... }],
///     [sig =] <signature>,
/// )
/// ```
/// The parser automatically detects whether it's a method signature (has parentheses) or a field
/// signature (a single, bare type).
///
/// ## Properties
///
/// - `jni = <path>` - Optionally override the jni crate path (default: auto-detected via
///   `proc_macro_crate`, must come first if given)
/// - `type_map = { RustType => java.lang.ClassName, ... }` - Optional type mappings for Rust types
/// - `sig = <signature>` - The signature ('`sig =`' prefix is optional for the signature)
///
/// The `type_map` property can be provided multiple times and mappings are merged.
///
/// The design allows for a `macro_rules` wrapper to inject `jni =` or `type_map =` properties,
/// without needing to parse anything else.
///
/// # Type Syntax
///
/// ## Primitive Types
/// - Java primitives: `jboolean`, `jbyte`, `jchar`, `jshort`, `jint`, `jlong`, `jfloat`, `jdouble`
/// - Aliases: `boolean`/`bool`, `byte`/`i8`, `char`, `short`/`i16`, `int`/`i32`, `long`/`i64`,
///   `float`/`f32`, `double`/`f64`
/// - Void: `void` or `()` or elided return type defaults to `void`
///
/// ## Java Object Types
/// - Fully qualified: `java.lang.String`, `java.util.List` or as string literal: `"java.util.List"`
/// - With inner classes: `java.lang.Outer::Inner` or as string literal: `"java.lang.Outer$Inner"`
/// - Default package: `.ClassName` or as string literal: `".ClassName"`
///
/// _(Notice that Java object types _always_ contain at least one `.` dot)_
///
/// ## Rust Reference Types
/// - Single identifier or path: `JString`, `JObject`, `jni::objects::JString`, `RustType`,
///   `custom::RustType`
///
/// ## Array Types
/// - Prefix syntax: `[jint]`, `[[java.lang.String]]`, `[RustType]`
/// - Suffix syntax: `jint[]`, `java.lang.String[][]`, `RustType[]`
///
/// ### Built-in Types
/// - Types like `JObject`, `JClass`, `JString` etc from the `jni` crate can be used without a
///   `type_map`
/// - Built-in types can also be referenced like `jni::objects::JString`
/// - Java types like `java.lang.Class` are automatically mapped to built-in types like `JClass`
///
/// ### Core Types
/// - The core types `java.lang.Object`, `java.lang.Class`, `java.lang.String` and
///   `java.lang.Throwable` can not be mapped to custom types.
/// - Other built-in types, such as `JList` (`java.util.List`) can be overridden by mapping them to
///   a different type via a `type_map`
///
/// ## Type Mappings via `type_map` Block
///
/// A `type_map` block:
/// - Maps Rust [Reference] type names to Java class names for use in method/field signatures.
/// - Maps Java class names to Rust types
/// - Allows the definition of type aliases for more ergonomic / readable signatures.
///
/// Multiple `type_map` blocks will be merged, so that wrapper macros may forward-declare common
/// type mappings to avoid repetition.
///
/// A `type_map` supports three types of mappings:
///
/// ### Reference Type Mappings
///
/// Map Rust [Reference] types to Java classes like `RustType => java.type.Name`:
///
/// ```ignore
/// type_map = {
///     CustomType => com.example.CustomClass,
///     AnotherType => "com.example.AnotherClass",
///     InnerType => com.example.Outer::Inner,
///     AnotherInnerType => "com.example.Outer$AnotherInner",
///     my_crate::MyType => com.example.MyType,
/// }
/// ```
///
/// The right-side Java type uses the syntax for Java Object Types described above.
///
/// ### Unsafe Primitive Type Mappings
///
/// Map Rust types to Java primitive types using the `unsafe` keyword. This is particularly useful
/// for Rust types that transparently wrap a pointer (e.g., handles) that need to be passed to Java
/// as a `long`:
///
/// ```ignore
/// type_map = {
///     unsafe MyHandle => long,
///     unsafe MyBoxedPointer => long,
///     unsafe MyRawFd => int,
/// }
/// ```
///
/// These mappings are marked `unsafe` because it's not possible to verify the safety of casting
/// between the Rust type and Java primitive type - apart from checking the size and alignment.
///
/// ### Type Aliases
///
/// Creates aliases for existing type mappings using the `typealias` keyword. This can improve
/// readability in signatures before defining full type bindings:
///
/// ```ignore
/// type_map = {
///     MyType => com.example.MyType,
///     typealias MyAlias => MyType,
///     typealias MyObjectAlias => JObject,
/// }
/// ```
///
/// Note: Aliases for array types are not supported.
///
/// # Method Signature Syntax
///
/// A method can be given in one of these forms:
/// - `( [args...] ) -> TYPE`
/// - `( [args...] )`
/// - `"RAW_JNI_SIG"`
///
/// An argument can be given in these forms:
/// - `name: TYPE`
/// - `TYPE`
///
/// _(with a `TYPE` as described in the `Type Syntax` section above)_
///
/// A `TYPE` may have an optional `&` prefix that is ignored
///
/// ```
/// # use jni::{jni_sig, signature::{MethodSignature, JavaType, Primitive}};
/// const JNI_SIG: MethodSignature =
///     jni_sig!((arg1: com.example.Type, arg2: JString, arg3: jint) -> JString);
/// # fn main() {
/// assert!(JNI_SIG.sig().to_bytes() == b"(Lcom/example/Type;Ljava/lang/String;I)Ljava/lang/String;");
/// assert!(JNI_SIG.args().len() == 3);
/// assert!(JNI_SIG.args()[0] == JavaType::Object);
/// assert!(JNI_SIG.args()[1] == JavaType::Object);
/// assert!(JNI_SIG.args()[2] == JavaType::Primitive(Primitive::Int));
/// assert!(JNI_SIG.ret() == JavaType::Object);
/// # }
/// ```
///
/// Traditional JNI signature syntax is also supported:
/// ```ignore
/// jni_sig!("(IILjava/lang/String;)V")
/// ```
///
/// Explicitly named 'sig' property:
/// ```ignore
/// jni_sig!(sig = (arg1: Type1, arg2: Type2, ...) -> ReturnType)
/// ```
///
/// With type mappings:
/// ```
/// # use jni::{jni_sig, signature::{MethodSignature, JavaType, Primitive}};
/// const JNI_SIG: MethodSignature = jni_sig!(
///     type_map = {
///         CustomType => java.class.Type,
///         ReturnType => java.class.ReturnType,
///     },
///     (arg1: CustomType, arg2: JString, arg3: jint) -> ReturnType,
/// );
/// ```
///
/// # Field Signature Syntax
///
/// ```ignore
/// jni_sig!(Type)
/// ```
///
/// Traditional JNI signature syntax is also supported:
/// ```ignore
/// jni_sig!("Ljava/lang/String;")
/// ```
///
/// Named:
/// ```ignore
/// jni_sig!(sig = Type)
/// ```
///
/// With type mappings:
/// ```ignore
/// jni_sig!(
///     Type,
///     type_map = {
///         RustType as java.class.Name,
///         ...
///     }
/// )
/// ```
///
/// # Examples
///
/// ## Method Signatures
///
/// Basic primitive types:
/// ```ignore
/// const SIG: MethodSignature = jni_sig!((a: jint, b: jboolean) -> void);
/// // Result: MethodSignature for "(IZ)V"
/// ```
///
/// Java object types:
/// ```ignore
/// const SIG: MethodSignature = jni_sig!(
///     (a: jint, b: java.lang.String) -> java.lang.Object
/// );
/// // Result: MethodSignature for "(ILjava/lang/String;)Ljava/lang/Object;"
/// ```
///
/// Array types:
/// ```ignore
/// const SIG: MethodSignature = jni_sig!(
///     (a: [jint], b: [java.lang.String]) -> [[jint]]
/// );
/// // Result: MethodSignature for "([I[Ljava/lang/String;)[[I"
/// ```
///
/// With type mappings:
/// ```ignore
/// const SIG: MethodSignature = jni_sig!(
///     type_map = {
///         MyString as java.lang.String,
///         MyObject as java.lang.Object,
///         MyThrowable as java.lang.Throwable,
///     },
///     (a: jint, b: MyString, c: [MyObject]) -> MyThrowable,
/// );
/// // Result: MethodSignature for "(ILjava/lang/String;[Ljava/lang/Object;)Ljava/lang/Throwable;"
/// ```
/// Multiple type_maps:
/// ```ignore
/// const SIG: MethodSignature = jni_sig!(
///     jni = ::my_jni,
///     type_map = { MyType0 => custom.Type0 },
///     type_map = { MyType1 => custom.Type1 },
///     sig = (arg0: MyType0, arg1: MyType1) -> JString,
/// );
/// ```
///
/// This makes it possible to write wrapper macros to inject a `type_map` without blocking the use
/// of `type_map` for additional types.
///
/// With named signature property:
/// ```ignore
/// const SIG: MethodSignature = jni_sig!(
///     type_map = { MyType => java.lang.MyType },
///     sig = (a: jint) -> void,
/// );
/// ```
///
/// With custom jni crate path:
/// ```ignore
/// const SIG: MethodSignature = jni_sig!(
///     jni = ::my_jni, // must come first!
///     (a: jint) -> void,
/// );
/// ```
///
/// ## Field Signatures
///
/// Primitive field:
/// ```ignore
/// const SIG: FieldSignature = jni_sig!(jint);
/// // Result: FieldSignature for "I"
/// ```
///
/// Object field:
/// ```ignore
/// const SIG: FieldSignature = jni_sig!(java.lang.String);
/// // Result: FieldSignature for "Ljava/lang/String;"
/// ```
///
/// Array field:
/// ```ignore
/// const SIG: FieldSignature = jni_sig!([jint]);
/// // Result: FieldSignature for "[I"
/// ```
///
/// Field with type mapping:
/// ```ignore
/// const SIG: FieldSignature = jni_sig!(
///     type_map = {
///         MyType as custom.Type,
///     },
///     MyType
/// );
/// // Result: FieldSignature for "Lcustom/Type;"
/// ```
/// [Reference]: https://docs.rs/jni/latest/jni/refs/trait.Reference.html
#[proc_macro]
pub fn jni_sig(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    signature::jni_sig_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Parses a JNI method or field signature at compile time and returns a `&str` literal.
///
/// This macro is similar to `jni_sig!` but returns a plain UTF-8 string literal instead
/// of a `MethodSignature` or `FieldSignature` struct.
///
/// See the `jni_sig!` macro documentation for detailed syntax and examples.
///
/// # Examples
///
/// ```ignore
/// const SIG: &str = jni_sig_str!((a: jint, b: jboolean) -> void);
/// // Result: "(IZ)V"
///
/// const FIELD_SIG: &str = jni_sig_str!(java.lang.String);
/// // Result: "Ljava/lang/String;"
/// ```
#[proc_macro]
pub fn jni_sig_str(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    signature::jni_sig_str_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Parses a JNI method or field signature at compile time and returns a CStr literal.
///
/// This macro is similar to `jni_sig!` but returns a C string literal (e.g., `c"(IZ)V"`)
/// with MUTF-8 encoding instead of a `MethodSignature` or `FieldSignature` struct.
///
/// The output is encoded using Java's modified UTF-8 (MUTF-8) format via `cesu8::to_java_cesu8`.
///
/// See the `jni_sig!` macro documentation for detailed syntax and examples.
///
/// # Examples
///
/// ```ignore
/// const SIG: &CStr = jni_sig_cstr!((a: jint, b: jboolean) -> void);
/// // Result: c"(IZ)V" (MUTF-8 encoded)
///
/// const FIELD_SIG: &CStr = jni_sig_cstr!(java.lang.String);
/// // Result: c"Ljava/lang/String;" (MUTF-8 encoded)
/// ```
#[proc_macro]
pub fn jni_sig_cstr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    signature::jni_sig_cstr_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Parses a JNI method or field signature at compile time and returns a `&'static JNIStr`.
///
/// This macro is similar to `jni_sig!` but returns a `&'static JNIStr` with MUTF-8 encoding
/// instead of a `MethodSignature` or `FieldSignature` struct.
///
/// The output is encoded using Java's modified UTF-8 (MUTF-8) format via `cesu8::to_java_cesu8`
/// and wrapped in a `JNIStr` via `jni::strings::JNIStr::from_cstr_unchecked()`.
///
/// See the `jni_sig!` macro documentation for detailed syntax and examples.
///
/// # Examples
///
/// ```ignore
/// const SIG: &JNIStr = jni_sig_jstr!((a: jint, b: jboolean) -> void);
/// // Result: &'static JNIStr for "(IZ)V" (MUTF-8 encoded)
///
/// const FIELD_SIG: &JNIStr = jni_sig_jstr!(java.lang.String);
/// // Result: &'static JNIStr for "Ljava/lang/String;" (MUTF-8 encoded)
/// ```
#[proc_macro]
pub fn jni_sig_jstr(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    signature::jni_sig_jstr_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Export a Rust function with a JNI-compatible, mangled method name.
///
/// This adds an appropriate `#[export_name = "..."]` attribute and `extern
/// "system"` ABI to the function, to allow it to be resolved by a JVM when
/// calling an associated native method.
///
/// This attribute takes one to three string literal arguments:
/// 1. Package namespace (required)
/// 2. Method name (optional)
/// 3. JNI signature (optional)
///
/// If two arguments are given, the second is inferred to be a method name if it
/// doesn't contain '(', otherwise it's treated as a signature.
///
/// The name is mangled according to the JNI Specification, under "Design" ->
/// "Resolving Native Method Names"
///
/// <https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html#resolving-native-method-names>
///
/// ## Method Name Generation
///
/// If no method name is provided, the Rust function name is converted from
/// `snake_case` to `lowerCamelCase`.
///
/// If the Rust function name is not entirely lowercase with underscores (i.e.
/// it contains any uppercase letters), the name is used directly without
/// transformation.
///
/// ## `snake_case` to `lowerCamelCase` Conversion Rules
///
/// If the input contains any uppercase letters, it's returned unchanged to
/// preserve intentional casing.
///
/// Leading underscores are preserved except for one underscore that is removed.
///
/// Trailing underscores are preserved.
///
/// When capitalizing segments after underscores, the first non-numeric
/// character is capitalized. This ensures that segments with numeric prefixes
/// are properly capitalized.
///
/// Examples:
/// - `"say_hello"` -> `"sayHello"`
/// - `"get_user_name"` -> `"getUserName"`
/// - `"_private_method"` -> `"privateMethod"` (one leading underscore removed)
/// - `"__dunder__"` -> `"_dunder__"` (one leading underscore removed)
/// - `"___priv"` -> `"__priv"` (one leading underscore removed)
/// - `"trailing_"` -> `"trailing_"`
/// - `"sayHello"` -> `"sayHello"` (unchanged)
/// - `"getUserName"` -> `"getUserName"` (unchanged)
/// - `"Foo_Bar"` -> `"Foo_Bar"` (unchanged - contains uppercase)
/// - `"XMLParser"` -> `"XMLParser"` (unchanged - contains uppercase)
/// - `"init"` -> `"init"` (unchanged - no underscores)
/// - `"test_αλφα"` -> `"testΑλφα"` (Unicode-aware)
/// - `"array_2d_foo"` -> `"array2DFoo"` (capitalizes first char after digits)
/// - `"test_3d"` -> `"test3D"` (capitalizes first char after digits)
///
/// ## ABI Handling
///
/// The macro requires the ABI to be `extern "system"` (required for JNI).
/// - If no ABI is specified, it will automatically be set to `extern "system"`
/// - If `extern "system"` is already specified, it will be preserved
/// - If any other ABI (e.g., `extern "C"`) is specified, a compile error will
///   be generated
///
/// ## Examples
///
/// Basic usage with just namespace (function name converted to lowerCamelCase):
/// ```
/// # use jni::{ EnvUnowned, objects::{ JObject, JString } };
/// # use jni_macros::jni_mangle;
///
/// // Rust function in snake_case
/// #[jni_mangle("com.example.RustBindings")]
/// pub fn say_hello<'local>(mut env: EnvUnowned<'local>, _: JObject<'local>, name: JString<'local>) -> JString<'local> {
///     // ...
/// #     unimplemented!()
/// }
/// // Generates: Java_com_example_RustBindings_sayHello
/// ```
///
/// Or already in lowerCamelCase (idempotent):
/// ```
/// # use jni::{ EnvUnowned, objects::{ JObject, JString } };
/// # use jni_macros::jni_mangle;
/// #[allow(non_snake_case)]
/// #[jni_mangle("com.example.RustBindings")]
/// pub fn sayHello<'local>(mut env: EnvUnowned<'local>, _: JObject<'local>, name: JString<'local>) -> JString<'local> {
///     // ...
/// #     unimplemented!()
/// }
/// // Generates: Java_com_example_RustBindings_sayHello
/// ```
///
/// The `sayHello` function will automatically be expanded to have the correct
/// ABI specification and the appropriate JNI-compatible name, i.e. in this case
/// - `Java_com_example_RustBindings_sayHello`.
///
/// Then it can be accessed by, for example, Kotlin code as follows:
/// ```kotlin
/// package com.example.RustBindings
///
/// class RustBindings {
///     private external fun sayHello(name: String): String
///
///     fun greetWorld() {
///         println(sayHello("world"))
///     }
/// }
/// ```
///
/// With custom method name:
/// ```
/// # use jni::{ EnvUnowned, objects::JObject };
/// # use jni_macros::jni_mangle;
/// #[jni_mangle("com.example.RustBindings", "customMethodName")]
/// pub fn some_rust_function<'local>(env: EnvUnowned<'local>, _: JObject<'local>) { }
/// // Generates: Java_com_example_RustBindings_customMethodName
/// ```
///
/// With signature only (overloaded method):
/// ```
/// # use jni::{ EnvUnowned, objects::JObject };
/// # use jni_macros::jni_mangle;
/// #[jni_mangle("com.example.RustBindings", "(I)Z")]
/// pub fn boolean_method<'local>(env: EnvUnowned<'local>, _: JObject<'local>) { }
/// // Generates: Java_com_example_RustBindings_booleanMethod__I
/// // Note: Only argument types are encoded (I), return type (Z) is ignored
/// ```
///
/// With method name and signature:
/// ```
/// # use jni::{ EnvUnowned, objects::JObject };
/// # use jni_macros::jni_mangle;
/// #[jni_mangle("com.example.RustBindings", "customName", "(Ljava/lang/String;)V")]
/// pub fn another_function<'local>(env: EnvUnowned<'local>, _: JObject<'local>) { }
/// // Generates: Java_com_example_RustBindings_customName__Ljava_lang_String_2
/// // Note: Only argument types are encoded, return type (V) is ignored
/// ```
///
/// Pre-existing "system" ABI is preserved:
/// ```
/// # use jni::{ EnvUnowned, objects::JObject };
/// # use jni_macros::jni_mangle;
/// #[jni_mangle("com.example.RustBindings")]
/// pub extern "system" fn my_function<'local>(env: EnvUnowned<'local>, _: JObject<'local>) { }
/// // The ABI will be set to "system" but you can also set it explicitly
/// ```
#[proc_macro_attribute]
pub fn jni_mangle(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    mangle::jni_mangle2(attr.into(), item.into()).into()
}
