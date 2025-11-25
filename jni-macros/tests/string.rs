//! Tests for jni_str! and jni_cstr! macros
//!
//! These tests verify the macros properly:
//! - Encode strings to MUTF-8 format
//! - Support concatenation of multiple literals
//! - Support different literal types
//! - Handle Unicode correctly (including surrogate pairs)
//! - Roundtrip encode/decode correctly
//! - Work in const contexts

use jni::strings::JNIStr;
use jni_macros::{jni_cstr, jni_str};
use std::ffi::CStr;

// Helper function to verify MUTF-8 roundtrip encoding
fn verify_roundtrip(original: &str, mutf8_bytes: &[u8]) {
    match cesu8::from_java_cesu8(mutf8_bytes) {
        Ok(decoded) => {
            assert_eq!(
                decoded, original,
                "Roundtrip failed: decoded '{}' != original '{}'",
                decoded, original
            );
        }
        Err(e) => {
            panic!("Failed to decode MUTF-8: {:?}", e);
        }
    }
}

#[test]
fn test_jni_cstr_basic_ascii() {
    const CLASS: &CStr = jni_cstr!("java.lang.String");

    assert_eq!(CLASS.to_str().unwrap(), "java.lang.String");
    verify_roundtrip("java.lang.String", CLASS.to_bytes());
}

#[test]
fn test_jni_str_basic_ascii() {
    const CLASS: &JNIStr = jni_str!("java.lang.String");

    let bytes = CLASS.to_bytes();
    assert_eq!(bytes, b"java.lang.String");
    verify_roundtrip("java.lang.String", bytes);
}

#[test]
fn test_jni_cstr_and_jni_str_produce_same_bytes() {
    const CSTR: &CStr = jni_cstr!("com.example.TestClass");
    const JSTR: &JNIStr = jni_str!("com.example.TestClass");

    assert_eq!(CSTR.to_bytes(), JSTR.to_bytes());
}

#[test]
fn test_concatenation_multiple_strings() {
    const PACKAGE: &CStr = jni_cstr!("com.example.", "MyClass");
    const JPACKAGE: &JNIStr = jni_str!("com.example.", "MyClass");

    assert_eq!(PACKAGE.to_str().unwrap(), "com.example.MyClass");
    assert_eq!(PACKAGE.to_bytes(), JPACKAGE.to_bytes());
    verify_roundtrip("com.example.MyClass", PACKAGE.to_bytes());
}

#[test]
fn test_concatenation_three_strings() {
    const PATH: &CStr = jni_cstr!("java", ".", "lang", ".", "Object");

    assert_eq!(PATH.to_str().unwrap(), "java.lang.Object");
    verify_roundtrip("java.lang.Object", PATH.to_bytes());
}

#[test]
fn test_mixed_literal_types_string_and_int() {
    const PORT: &CStr = jni_cstr!("Port: ", 8080);
    const JPORT: &JNIStr = jni_str!("Port: ", 8080);

    assert_eq!(PORT.to_str().unwrap(), "Port: 8080");
    assert_eq!(PORT.to_bytes(), JPORT.to_bytes());
    verify_roundtrip("Port: 8080", PORT.to_bytes());
}

#[test]
fn test_mixed_literal_types_with_char() {
    const VERSION: &CStr = jni_cstr!("Version ", 1, '.', 2);

    assert_eq!(VERSION.to_str().unwrap(), "Version 1.2");
    verify_roundtrip("Version 1.2", VERSION.to_bytes());
}

#[test]
fn test_boolean_literal() {
    const ENABLED: &CStr = jni_cstr!("enabled=", true);
    const DISABLED: &CStr = jni_cstr!("disabled=", false);

    assert_eq!(ENABLED.to_str().unwrap(), "enabled=true");
    assert_eq!(DISABLED.to_str().unwrap(), "disabled=false");
}

#[test]
fn test_float_literal() {
    const PI: &CStr = jni_cstr!("pi=", 3.14);

    assert_eq!(PI.to_str().unwrap(), "pi=3.14");
}

#[test]
fn test_char_literal() {
    const SEPARATOR: &CStr = jni_cstr!("separator", '=', "value");

    assert_eq!(SEPARATOR.to_str().unwrap(), "separator=value");
}

#[test]
fn test_byte_literal() {
    // Byte literals are converted to their numeric value
    const BYTE_VAL: &CStr = jni_cstr!("byte=", b'A');

    assert_eq!(BYTE_VAL.to_str().unwrap(), "byte=65"); // ASCII value of 'A'
}

#[test]
fn test_cstr_literal() {
    const FROM_CSTR: &CStr = jni_cstr!(c"hello");

    assert_eq!(FROM_CSTR.to_str().unwrap(), "hello");
}

#[test]
fn test_unicode_emoji_surrogate_pairs() {
    // Emoji above U+FFFF require surrogate pair encoding in MUTF-8
    const EMOJI: &CStr = jni_cstr!("😀");
    const JEMOJI: &JNIStr = jni_str!("😀");

    let bytes = EMOJI.to_bytes();
    let jbytes = JEMOJI.to_bytes();

    // Both should produce same bytes
    assert_eq!(bytes, jbytes);

    // MUTF-8 should be longer than UTF-8 for emoji
    let utf8_bytes = "😀".as_bytes();
    assert_eq!(utf8_bytes.len(), 4); // UTF-8: 4 bytes
    assert_eq!(bytes.len(), 6); // MUTF-8: 6 bytes (surrogate pair)

    // Verify roundtrip
    verify_roundtrip("😀", bytes);
}

#[test]
fn test_unicode_japanese_no_surrogate_pairs() {
    // Japanese characters below U+FFFF don't need surrogate pairs
    const JP: &CStr = jni_cstr!("こんにちは");

    let bytes = JP.to_bytes();
    let utf8_bytes = "こんにちは".as_bytes();

    // UTF-8 and MUTF-8 should be identical for these characters
    assert_eq!(bytes, utf8_bytes);

    verify_roundtrip("こんにちは", bytes);
}

#[test]
fn test_unicode_in_class_name() {
    const CLASS: &CStr = jni_cstr!("emoji.Type😀");

    verify_roundtrip("emoji.Type😀", CLASS.to_bytes());
}

#[test]
fn test_const_context() {
    // Verify macros work in const contexts
    const CONST_CSTR: &CStr = jni_cstr!("const.test");
    const CONST_JSTR: &JNIStr = jni_str!("const.test");

    assert_eq!(CONST_CSTR.to_str().unwrap(), "const.test");
    assert_eq!(CONST_CSTR.to_bytes(), CONST_JSTR.to_bytes());

    // Test in const fn
    const fn get_class_name() -> &'static CStr {
        jni_cstr!("const.fn.Test")
    }

    const CLASS: &CStr = get_class_name();
    assert_eq!(CLASS.to_str().unwrap(), "const.fn.Test");
}

#[test]
fn test_static_context() {
    static CLASS: &CStr = jni_cstr!("static.test.Class");
    static JCLASS: &JNIStr = jni_str!("static.test.Class");

    assert_eq!(CLASS.to_bytes(), JCLASS.to_bytes());
}

#[test]
fn test_empty_package() {
    // Classes in default package (no package)
    const DEFAULT_PKG: &CStr = jni_cstr!(".DefaultClass");

    assert_eq!(DEFAULT_PKG.to_str().unwrap(), ".DefaultClass");
    verify_roundtrip(".DefaultClass", DEFAULT_PKG.to_bytes());
}

#[test]
fn test_inner_class_notation() {
    // Inner classes use $ in JNI
    const INNER: &CStr = jni_cstr!("com.example.Outer$Inner");

    assert_eq!(INNER.to_str().unwrap(), "com.example.Outer$Inner");
    verify_roundtrip("com.example.Outer$Inner", INNER.to_bytes());
}

#[test]
fn test_concatenation_builds_inner_class() {
    const INNER: &CStr = jni_cstr!("com.example.Outer", "$", "Inner");

    assert_eq!(INNER.to_str().unwrap(), "com.example.Outer$Inner");
}

#[test]
fn test_method_name() {
    const METHOD: &CStr = jni_cstr!("toString");
    const JMETHOD: &JNIStr = jni_str!("toString");

    assert_eq!(METHOD.to_bytes(), JMETHOD.to_bytes());
    verify_roundtrip("toString", METHOD.to_bytes());
}

#[test]
fn test_field_name() {
    const FIELD: &CStr = jni_cstr!("value");

    assert_eq!(FIELD.to_str().unwrap(), "value");
    verify_roundtrip("value", FIELD.to_bytes());
}

#[test]
fn test_special_characters() {
    // Test various special characters that might appear in class names
    const SPECIAL: &CStr = jni_cstr!("test_class-name.v2");

    assert_eq!(SPECIAL.to_str().unwrap(), "test_class-name.v2");
    verify_roundtrip("test_class-name.v2", SPECIAL.to_bytes());
}

#[test]
fn test_long_package_name() {
    const LONG: &CStr = jni_cstr!("com.example.very.long.package.name.with.many.segments.MyClass");

    assert_eq!(
        LONG.to_str().unwrap(),
        "com.example.very.long.package.name.with.many.segments.MyClass"
    );
    verify_roundtrip(
        "com.example.very.long.package.name.with.many.segments.MyClass",
        LONG.to_bytes(),
    );
}

#[test]
fn test_jnistr_to_bytes_method() {
    const JSTR: &JNIStr = jni_str!("test.Class");

    let bytes = JSTR.to_bytes();
    assert_eq!(bytes, b"test.Class");
}

#[test]
fn test_roundtrip_multiple_emoji() {
    const MULTI_EMOJI: &CStr = jni_cstr!("😀🚀👍");

    let bytes = MULTI_EMOJI.to_bytes();

    // Each emoji should be 6 bytes in MUTF-8
    assert_eq!(bytes.len(), 18); // 3 emoji * 6 bytes each

    verify_roundtrip("😀🚀👍", bytes);
}

#[test]
fn test_mixed_ascii_and_unicode() {
    const MIXED: &CStr = jni_cstr!("Hello", "世界", "😀");

    verify_roundtrip("Hello世界😀", MIXED.to_bytes());
}

#[test]
fn test_trailing_comma_in_concatenation() {
    const TRAILING: &CStr = jni_cstr!("hello", "world",);

    assert_eq!(TRAILING.to_str().unwrap(), "helloworld");
}

#[test]
fn test_negative_numbers() {
    const NEG: &CStr = jni_cstr!("value=", -42);

    assert_eq!(NEG.to_str().unwrap(), "value=-42");
}

#[test]
fn test_zero() {
    const ZERO: &CStr = jni_cstr!("count=", 0);

    assert_eq!(ZERO.to_str().unwrap(), "count=0");
}
