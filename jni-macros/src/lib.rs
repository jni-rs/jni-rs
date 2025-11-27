mod mangle;
mod native_method;
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
/// **Note:** The signature and `type_map` syntax supported by this macro is also used by the
/// [`native_method`] macro.
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
/// Note: this syntax for signature types is also used by the  [`native_method`] macro.
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
/// - Maps Java class names to Rust types (primarily for use with the [`native_method`] macro)
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
/// These mappings are marked `unsafe` since there's no way to automatically verify that these are
/// FFI-safe types - apart from checking the size and alignment.
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

/// Create a compile-time type-checked `NativeMethod` for registering native methods with the JVM.
///
/// This macro generates a [`NativeMethod`] struct with compile-time guarantees that the Rust
/// function matches the JNI signature. It can optionally:
/// - Wrap implementations with panic safety (`catch_unwind`) and error handling
/// - Generate JNI export symbols for automatic JVM resolution
/// - Perform runtime ABI checks to ensure static/instance methods are registered correctly
///
/// This macro provides strong type safety for implementing individual native methods.
///
/// [`NativeMethod`]: https://docs.rs/jni/latest/jni/struct.NativeMethod.html
///
/// # Quick Example
///
/// ```
/// # use jni::{Env, native_method, objects::JObject, sys::jint};
/// // Instance method with default settings
/// const ADD_METHOD: jni::NativeMethod = native_method! {
///     java_type = "com.example.MyClass",
///     extern fn native_add(a: jint, b: jint) -> jint,
/// };
/// // Will export `Java_com_example_MyClass_nativeAdd__II` symbol and
/// // `ADD_METHOD` can be passed to `Env::register_native_methods`
///
/// fn native_add<'local>(
///     _env: &mut Env<'local>,
///     _this: JObject<'local>,
///     a: jint,
///     b: jint,
/// ) -> Result<jint, jni::errors::Error> {
///     Ok(a + b)
/// }
/// ```
///
/// # Syntax Overview
///
/// The macro supports both property-based and shorthand syntax, which can be combined:
///
/// ```ignore
/// native_method! {
///     [java_type = "com.example.MyClass",]  // For exports (required with `extern` or `export = true`)
///     [rust_type = CustomType,]             // Type for 'this' (default: JObject)
///     [static] [raw] [extern] fn [RustType::]method_name(args) -> ret, // Shorthand signature
///     [fn = implementation_fn,]             // Function path (default: RustType::method_name from shorthand)
///     [... other properties ...]
/// }
/// ```
///
/// # Generated Code
///
/// The macro generates a `const` block containing:
/// 1. A type-checked wrapper function
/// 2. An optional runtime type-check for the second parameter (to distinguish static vs instance
///    methods)
/// 3. An optional export function with a mangled JNI name
/// 4. A `NativeMethod` struct created via `NativeMethod::from_raw_parts`
///
/// For non-raw methods with the default settings:
///
/// ```ignore
/// const {
///     // Generated wrapper with panic safety and error handling
///     extern "system" fn __native_method_wrapper<'local>(
///         mut unowned_env: EnvUnowned<'local>,
///         this: JObject<'local>,
///         a: jint,
///         b: jint,
///     ) -> jint {
///         // One-time ABI check: validates that 'this' is NOT a Class (i.e., instance method)
///         static _ABI_CHECK: ::std::sync::Once = ::std::sync::Once::new();
///         _ABI_CHECK.call_once(|| {
///             // ... check that second parameter is not java.lang.Class ...
///         });
///
///         unowned_env
///             .with_env(|env| {
///                 // Call your implementation
///                 native_add(env, this, a, b)
///             })
///             .resolve::<ThrowRuntimeExAndDefault>()
///     }
///
///     unsafe {
///         NativeMethod::from_raw_parts(
///             jni_str!("nativeAdd"),
///             jni_str!("(II)I"),
///             __native_method_wrapper as *mut c_void,
///         )
///     }
/// }
/// ```
///
/// With `export = true` or `extern` qualifier, an additional export function is generated:
///
/// ```ignore
/// #[export_name = "Java_com_example_MyClass_nativeAdd__II"]
/// pub extern "system" fn __native_method_export<'local>(...) -> jint {
///     __native_method_wrapper(...)
/// }
/// ```
///
/// # Property Reference
///
/// ## `java_type` - Java Class Name
///
/// **Required** when using `export = true` or the `extern` qualifier.
///
/// The fully-qualified Java class name containing this native method.
///
/// ```ignore
/// java_type = "com.example.MyClass"
/// ```
///
/// Can also be specified as dot-separated identifiers:
/// ```ignore
/// java_type = com.example.MyClass
/// ```
///
/// See the 'Java Object Types' section in the [`jni_sig!`] documentation for details on how to
/// specify Java types, including inner classes and default-package classes.
///
/// ## `rust_type` - Custom Type for 'this' Parameter
///
/// For instance methods, specifies the Rust type for the `this` parameter. Defaults to `JObject`.
///
/// ```ignore
/// rust_type = MyCustomType
/// ```
///
/// This type must implement `jni::refs::Reference`.
///
/// ## Shorthand Signature
///
/// The shorthand syntax allows specifying method details in a function-like form:
///
/// ```ignore
/// [static] [raw] [extern] fn [RustType::]method_name(args) -> ret
/// ```
///
/// Where:
/// - `static` - Static method (receives `class: JClass` instead of `this`)
/// - `raw` - No panic safety wrapper, receives `EnvUnowned`, returns value directly (not `Result`)
/// - `extern` - Generate JNI export symbol (requires `java_type`)
/// - `RustType::` - If present, sets `rust_type = RustType` and defaults `fn =
///   RustType::method_name`
/// - `method_name` - Converted from snake_case to lowerCamelCase for the Java method name
///
/// and the `args` and `ret` specify the method signature using the syntax from [`jni_sig!`].
///
/// Example:
/// ```
/// # use jni::{Env, native_method, sys::jint};
/// # struct MyType<'a>(std::marker::PhantomData<&'a ()>);
/// const METHOD: jni::NativeMethod = native_method! {
///     static fn MyType::compute_sum(a: jint, b: jint) -> jint,
/// };
///
/// impl MyType<'_> {
///     fn compute_sum<'local>(
///         _env: &mut Env<'local>,
///         _class: jni::objects::JClass<'local>,
///         a: jint,
///         b: jint,
///     ) -> Result<jint, jni::errors::Error> {
///         Ok(a + b)
///     }
/// }
/// ```
///
/// ## `fn` - Implementation Function Path
///
/// Path to the Rust function implementing this native method. Defaults to `RustType::method_name`
/// or `method_name` if a shorthand signature is given.
///
/// ```ignore
/// fn = my_module::my_implementation
/// ```
///
/// ## `name` - Java Method Name
///
/// The Java method name as a string literal. Defaults to the `method_name` name converted from
/// `snake_case` to `lowerCamelCase` if a shorthand signature is given.
///
/// ```ignore
/// name = "customMethodName"
/// ```
///
/// ## `sig` / Method Signature
///
/// Typically the signature will come from the shorthand syntax, but it can also be specified
/// explicitly via the `sig` property.
///
/// The method signature using the syntax from [`jni_sig!`].
///
/// ```ignore
/// sig = (param1: jint, param2: JString) -> jboolean
/// // or shorthand (part of function-like syntax):
/// fn my_method(param1: jint, param2: JString) -> jboolean
/// ```
///
/// ## `type_map` - Type Mappings
///
/// Optional type mappings for custom Rust types. See [`jni_sig!`] for full syntax.
///
/// ```ignore
/// type_map = {
///     CustomType => com.example.CustomClass,
///     unsafe HandleType => long,
/// }
/// ```
///
/// ## `static` - Static Method Flag
///
/// Indicates a static method. The second parameter will be `class: JClass` instead of a `this`
/// object.
///
/// ```ignore
/// static = true
/// // or as qualifier:
/// static fn my_method() -> jint
/// ```
///
/// ## `raw` - Raw Function Flag
///
/// When `raw = true` or the `raw` qualifier is used:
/// - Function receives `EnvUnowned<'local>` (not `&mut Env<'local>`)
/// - Function returns the value directly (not `Result`)
/// - No panic safety wrapper (`catch_unwind`)
/// - No automatic error handling
///
/// ```ignore
/// raw = true
/// // or as qualifier:
/// raw fn my_method(value: jint) -> jint
/// ```
///
/// Raw function signature:
/// ```ignore
/// fn my_raw_method<'local>(
///     env: EnvUnowned<'local>,
///     this: JObject<'local>,
///     value: jint,
/// ) -> jint {
///     value * 2
/// }
/// ```
///
/// ## `export` - JNI Export Symbol
///
/// Controls whether a JNI export symbol is generated:
/// - `true` - Generate auto-mangled JNI export name (e.g., `Java_com_example_Class_method__II`)
/// - `false` - Don't generate export
/// - `"CustomName"` - Use custom export name
///
/// Specifying the `extern` qualifier is equivalent to `export = true`.
///
/// **Note:** `java_type` must be provided when `export = true` or the `extern` qualifier is used.
///
/// ```ignore
/// export = true
/// // or as qualifier:
/// extern fn my_method() -> jint
/// // or with custom name:
/// export = "Java_custom_Name"
/// ```
///
/// ## `error_policy` - Error Handling Policy
///
/// For non-raw methods, specifies how to convert `Result` errors to JNI exceptions. Default is
/// `ThrowRuntimeExAndDefault`.
///
/// Built-in policies:
/// - `jni::errors::ThrowRuntimeExAndDefault` - Throws `RuntimeException`, returns default value
/// - `jni::errors::LogErrorAndDefault` - Logs error, returns default value
///
/// Or implement your own policy by implementing the `jni::errors::ErrorPolicy` trait.
///
/// ```ignore
/// error_policy = jni::errors::LogErrorAndDefault
/// ```
///
/// ## `catch_unwind` - Panic Safety
///
/// For non-raw methods, controls whether panics are caught and converted to Java exceptions.
/// Default is `true`.
///
/// - `true` - Use `EnvUnowned::with_env` (catches panics)
/// - `false` - Use `EnvUnowned::with_env_no_catch` (panics will abort when crossing FFI boundary)
///
/// ```ignore
/// catch_unwind = false
/// ```
///
/// **Note:** Not applicable to raw methods (which never have panic safety).
///
/// ## `abi_check` - Runtime ABI Validation
///
/// Controls runtime validation that the method is registered correctly as static/instance.
///
/// Values:
/// - `Always` - Always check (default)
/// - `UnsafeNever` - Never check (unsafe micro-optimization, for production if needed)
/// - `UnsafeDebugOnly` - Check only in debug builds (unsafe micro-optimization, for production if
///   needed)
///
/// ```ignore
/// abi_check = Always
/// ```
///
/// The check validates that the second parameter (`this` for instance, `class` for static) matches
/// how Java called the method. This is performed once per method via `std::sync::Once`.
///
/// Check failures for non-raw methods will throw an error that will be mapped via the specified
/// error handling policy. For raw methods, a panic will occur, which will abort at the FFI
/// boundary.
///
/// ## `jni` - Override JNI Crate Path
///
/// Override the path to the `jni` crate. Must be the first property if provided.
///
/// ```ignore
/// jni = ::my_jni_crate
/// ```
///
/// # Function Signature Requirements
///
/// ## Non-raw (Default)
///
/// Instance method:
/// ```ignore
/// fn<'local>(
///     env: &mut Env<'local>,
///     this: RustType<'local>,  // Or JObject<'local>
///     param1: jint,
///     param2: JString<'local>,
///     ...
/// ) -> Result<ReturnType, E>
/// where E: Into<jni::errors::Error>
/// ```
///
/// Static method:
/// ```ignore
/// fn<'local>(
///     env: &mut Env<'local>,
///     class: JClass<'local>,
///     param1: jint,
///     ...
/// ) -> Result<ReturnType, E>
/// where E: Into<jni::errors::Error>
/// ```
///
/// ## Raw
///
/// Instance method:
/// ```ignore
/// fn<'local>(
///     env: EnvUnowned<'local>,
///     this: RustType<'local>,  // Or JObject<'local>
///     param1: jint,
///     ...
/// ) -> ReturnType
/// ```
///
/// Static method:
/// ```ignore
/// fn<'local>(
///     env: EnvUnowned<'local>,
///     class: JClass<'local>,
///     param1: jint,
///     ...
/// ) -> ReturnType
/// ```
///
/// # Complete Examples
///
/// ## Basic Static Method
///
/// ```
/// # use jni::{Env, native_method, objects::JClass, sys::jint};
/// const METHOD: jni::NativeMethod = native_method! {
///     static fn native_compute(value: jint) -> jint,
/// };
///
/// fn native_compute<'local>(
///     _env: &mut Env<'local>,
///     _class: JClass<'local>,
///     value: jint,
/// ) -> Result<jint, jni::errors::Error> {
///     Ok(value * 100)
/// }
/// ```
///
/// ## Instance Method with Custom Type
///
/// ```
/// # use jni::{Env, native_method, sys::jint};
/// # struct Calculator<'a>(std::marker::PhantomData<&'a ()>);
/// const METHOD: jni::NativeMethod = native_method! {
///     fn Calculator::multiply(a: jint, b: jint) -> jint,
/// #   abi_check = UnsafeNever, // because Calculator isn't a real Reference type
/// };
///
/// impl Calculator<'_> {
///     fn multiply<'local>(
///         _env: &mut Env<'local>,
///         _this: Calculator<'local>,
///         a: jint,
///         b: jint,
///     ) -> Result<jint, jni::errors::Error> {
///         Ok(a * b)
///     }
/// }
/// ```
///
/// ## Exported Method with Type Mapping
///
/// ```
/// # use jni::{Env, native_method, objects::JString, sys::jint};
/// # struct MyHandle(*const u8);
/// # impl From<MyHandle> for jni::sys::jlong { fn from(h: MyHandle) -> jni::sys::jlong { h.0 as jni::sys::jlong } }
/// # struct MyType<'a>(std::marker::PhantomData<&'a ()>);
/// const METHOD: jni::NativeMethod = native_method! {
///     java_type = "com.example.MyClass",
///     type_map = {
///         unsafe MyHandle => long,
///     },
///     extern fn MyType::process(handle: MyHandle) -> JString,
/// #   abi_check = UnsafeNever, // because MyType isn't a real Reference type
/// };
///
/// impl MyType<'_> {
///     fn process<'local>(
///         env: &mut Env<'local>,
///         _this: MyType<'local>,
///         handle: MyHandle,
///     ) -> Result<JString<'local>, jni::errors::Error> {
///         JString::from_str(env, "processed")
///     }
/// }
/// ```
///
/// ## Raw Method (No Wrapping)
///
/// ```
/// # use jni::{EnvUnowned, native_method, objects::JObject, sys::jint};
/// const METHOD: jni::NativeMethod = native_method! {
///     raw fn fast_compute(value: jint) -> jint,
/// };
///
/// fn fast_compute<'local>(
///     _env: EnvUnowned<'local>,
///     _this: JObject<'local>,
///     value: jint,
/// ) -> jint {
///     value * 2
/// }
/// ```
///
/// ## Array of Methods for Registration
///
/// ```
/// # use jni::{Env, EnvUnowned, NativeMethod, native_method};
/// # use jni::objects::{JClass, JObject, JString};
/// # use jni::sys::jint;
/// const METHODS: &[NativeMethod] = &[
///     native_method! {
///         fn add(a: jint, b: jint) -> jint,
///     },
///     native_method! {
///         fn greet(name: JString) -> JString,
///     },
///     native_method! {
///         static fn get_version() -> jint,
///     },
///     native_method! {
///         raw fn fast_path(value: jint) -> jint,
///     },
/// ];
///
/// fn add<'local>(
///     _env: &mut Env<'local>, _this: JObject<'local>, a: jint, b: jint
/// ) -> Result<jint, jni::errors::Error> { Ok(a + b) }
///
/// fn greet<'local>(
///     env: &mut Env<'local>, _this: JObject<'local>, name: JString<'local>
/// ) -> Result<JString<'local>, jni::errors::Error> {
///     JString::from_str(env, &format!("Hello, {}", name.try_to_string(env)?))
/// }
///
/// fn get_version<'local>(
///     _env: &mut Env<'local>, _class: JClass<'local>
/// ) -> Result<jint, jni::errors::Error> { Ok(1) }
///
/// fn fast_path<'local>(
///     _env: EnvUnowned<'local>, _this: JObject<'local>, value: jint
/// ) -> jint { value }
///
/// fn register_native_methods<'local>(
///     env: &mut Env<'local>,
///     class: JClass<'local>,
/// ) -> Result<(), jni::errors::Error> {
///     unsafe { env.register_native_methods(class, METHODS) }
/// }
/// ```
///
/// # Type Safety
///
/// The macro ensures compile-time type safety by:
/// - Generating an `extern "system"` wrapper that has the correct ABI for registration with the
///   associated JNI signature
/// - Type-checking arguments when calling your implementation function
/// - Rejecting mismatches between the JNI signature and Rust types
///
/// **Important:** The macro cannot determine if a method is `static` or instance at compile time.
/// You must specify `static` correctly to ensure the second parameter type (`JClass` vs `JObject`)
/// matches. The `abi_check` property (enabled by default) adds runtime validation to catch
/// registration errors.
///
/// # Wrapper Macros
///
/// You can create wrapper macros to inject common configuration:
///
/// ```
/// # extern crate jni as jni2;
/// macro_rules! my_native_method {
///     ($($tt:tt)*) => {
///         ::jni2::native_method! {
///             jni = ::jni2,
///             type_map = {
///                 // Common type mappings
///             },
///             $($tt)*
///         }
///     };
/// }
/// ```
///
/// # See Also
///
/// - [`NativeMethod`] - The struct created by this macro
/// - [`jni_sig!`] - Signature syntax reference
/// - [`jni_mangle`] - Lower-level attribute macro for exports
///
/// [`NativeMethod`]: https://docs.rs/jni/latest/jni/struct.NativeMethod.html
#[proc_macro]
pub fn native_method(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    native_method::native_method_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
