//! Example demonstrating the jni_str! macro
//!
//! This macro converts UTF-8 string literals to MUTF-8 encoded literals at compile time.
//! MUTF-8 is Java's modified UTF-8 encoding used for JNI string operations.

use jni::strings::JNIStr;
use jni_macros::jni_str;

#[allow(unused)]
fn main() {
    // Basic usage - creates &'static JNIStr with MUTF-8 encoding
    const CLASS_NAME: &JNIStr = jni_str!("java.lang.String");
    const PACKAGE: &JNIStr = jni_str!("com.example.myapp");

    // Concatenating multiple literals
    const FULL_CLASS: &JNIStr = jni_str!("java.lang.", "String");
    const VERSION: &JNIStr = jni_str!("Version ", 1, '.', 0);

    // Common JNI use cases
    const ARRAY_LIST: &JNIStr = jni_str!("java.util.ArrayList");
    const HASH_MAP: &JNIStr = jni_str!("java.util.HashMap");
    const TO_STRING: &JNIStr = jni_str!("toString");
    const GET_VALUE: &JNIStr = jni_str!("getValue");
    const VALUE_FIELD: &JNIStr = jni_str!("value");
    const COUNT_FIELD: &JNIStr = jni_str!("count");

    // Inner classes (using $ separator)
    const OUTER_INNER: &JNIStr = jni_str!("com.example.Outer$Inner");
    const MAP_ENTRY: &JNIStr = jni_str!("java.util.Map$Entry");

    // Unicode support - Japanese characters
    const JP_CLASS: &JNIStr = jni_str!("jp.こんにちは.App");

    // Emoji (encoded as MUTF-8 surrogate pairs)
    // Note: High Unicode chars (U+10000+) are encoded as 6-byte surrogate pairs
    const EMOJI_CLASS: &JNIStr = jni_str!("emoji.MyApp😀");

    // Using in const fn
    const fn get_class() -> &'static JNIStr {
        jni_str!("com.example.MyClass")
    }

    const MY_CLASS: &JNIStr = get_class();
}
