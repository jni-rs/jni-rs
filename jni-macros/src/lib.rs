use crate::mangle::jni_mangle2;

mod mangle;

/// Annotate a function with this procedural macro attribute to expose it over the JNI.
///
/// This attribute takes a single string literal as an argument, specifying the package namespace
/// this function should be placed under.
///
/// ```
/// use jni::{ EnvUnowned, objects::{ JClass, JString }, sys::jstring };
/// use jni_macros::jni_mangle;
///
/// #[jni_mangle("com.example.RustBindings")]
/// pub fn sayHello<'local>(mut unowned_env: EnvUnowned<'local>, _: JClass<'local>, name: JString<'local>) -> JString<'local> {
///     unowned_env.with_env(|env| {
///         let name = name.to_string();
///
///         env.new_string(format!("Hello, {}!", name))
///     }).resolve::<jni::errors::ThrowRuntimeExAndDefault>()
/// }
/// ```
///
/// The `sayHello` function will automatically be expanded to have the correct ABI specification
/// and the appropriate JNI-compatible name, i.e. in this case -
/// `Java_com_example_RustBindings_sayHello`.
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
#[proc_macro_attribute]
pub fn jni_mangle(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    jni_mangle2(attr.into(), item.into()).into()
}
