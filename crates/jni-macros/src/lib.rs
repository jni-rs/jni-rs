mod bind_java_type;
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

#[doc = include_str!("../docs/jni_sig.md")]
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

#[doc = include_str!("../docs/native_method.md")]
#[proc_macro]
pub fn native_method(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    native_method::native_method_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[doc = include_str!("../docs/bind_java_type_overview.md")]
#[doc = include_str!("../docs/bind_java_type_properties.md")]
#[doc = include_str!("../docs/bind_java_type_examples.md")]
#[doc = include_str!("../docs/bind_java_type_advanced.md")]
#[proc_macro]
pub fn bind_java_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    bind_java_type::bind_java_type_impl(input.into())
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
