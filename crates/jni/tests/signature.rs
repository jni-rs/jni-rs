#![cfg(feature = "invocation")]
use jni::signature::{JavaType, Primitive};
use jni_macros::jni_sig;

#[test]
fn test_primitive_types() {
    let sig = jni_sig!((a: jint, b: jboolean) -> void);

    assert_eq!(sig.sig().to_bytes(), b"(IZ)V");
    assert_eq!(sig.args().len(), 2);
    assert_eq!(sig.args()[0], JavaType::Primitive(Primitive::Int));
    assert_eq!(sig.args()[1], JavaType::Primitive(Primitive::Boolean));
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_java_object_types() {
    let sig = jni_sig!((a: jint, b: java.lang.String) -> java.lang.Object);

    assert_eq!(
        sig.sig().to_bytes(),
        b"(ILjava/lang/String;)Ljava/lang/Object;"
    );
    assert_eq!(sig.args().len(), 2);
    assert_eq!(sig.args()[0], JavaType::Primitive(Primitive::Int));
    assert_eq!(sig.args()[1], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Object);
}

#[test]
fn test_array_types() {
    let sig = jni_sig!((a: [jint], b: [java.lang.String]) -> [[jint]]);

    assert_eq!(sig.sig().to_bytes(), b"([I[Ljava/lang/String;)[[I");
    assert_eq!(sig.args().len(), 2);
    assert_eq!(sig.args()[0], JavaType::Array);
    assert_eq!(sig.args()[1], JavaType::Array);
    assert_eq!(sig.ret(), JavaType::Array);
}

#[test]
fn test_suffix_array_syntax() {
    let sig = jni_sig!((a: jint[], b: java.lang.String[][]) -> void);

    assert_eq!(sig.sig().to_bytes(), b"([I[[Ljava/lang/String;)V");
    assert_eq!(sig.args().len(), 2);
    assert_eq!(sig.args()[0], JavaType::Array);
    assert_eq!(sig.args()[1], JavaType::Array);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_type_mappings() {
    let sig = jni_sig!(
        type_map = {
            RustType0 => java.lang.Type0,
            RustType1 => java.lang.Type1,
            RustType2 => java.lang.Type2,
        },
        (a: jint, b: RustType0, c: [RustType1], d: JString) -> RustType2,
    );

    assert_eq!(
        sig.sig().to_bytes(),
        b"(ILjava/lang/Type0;[Ljava/lang/Type1;Ljava/lang/String;)Ljava/lang/Type2;"
    );
    assert_eq!(sig.args().len(), 4);
    assert_eq!(sig.args()[0], JavaType::Primitive(Primitive::Int));
    assert_eq!(sig.args()[1], JavaType::Object);
    assert_eq!(sig.args()[2], JavaType::Array);
    assert_eq!(sig.args()[3], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Object);
}

#[test]
fn test_trailing_comma_no_type_map() {
    let sig = jni_sig!((a: jint, b: jboolean) -> void,);

    assert_eq!(sig.sig().to_bytes(), b"(IZ)V");
    assert_eq!(sig.args().len(), 2);
    assert_eq!(sig.args()[0], JavaType::Primitive(Primitive::Int));
    assert_eq!(sig.args()[1], JavaType::Primitive(Primitive::Boolean));
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_trailing_comma_with_type_map() {
    let sig = jni_sig!(
        (a: RustType) -> void,
        type_map = {
            RustType => java.lang.Type,
        },
    );

    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/Type;)V");
    assert_eq!(sig.args().len(), 1);
    assert_eq!(sig.args()[0], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_inner_classes() {
    let sig = jni_sig!((a: java.lang.Outer::Inner) -> void);

    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/Outer$Inner;)V");
    assert_eq!(sig.args().len(), 1);
    assert_eq!(sig.args()[0], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_default_package() {
    let sig = jni_sig!((a: .NoPackage) -> void);

    assert_eq!(sig.sig().to_bytes(), b"(LNoPackage;)V");
    assert_eq!(sig.args().len(), 1);
    assert_eq!(sig.args()[0], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_primitive_aliases() {
    let sig = jni_sig!((a: i32, b: bool, c: f64) -> i64);

    assert_eq!(sig.sig().to_bytes(), b"(IZD)J");
    assert_eq!(sig.args().len(), 3);
    assert_eq!(sig.args()[0], JavaType::Primitive(Primitive::Int));
    assert_eq!(sig.args()[1], JavaType::Primitive(Primitive::Boolean));
    assert_eq!(sig.args()[2], JavaType::Primitive(Primitive::Double));
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Long));
}

#[test]
fn test_builtin_rust_string_type() {
    let sig = jni_sig!((a: JString) -> void);

    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/String;)V");
    assert_eq!(sig.args().len(), 1);
    assert_eq!(sig.args()[0], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_empty_args() {
    let sig = jni_sig!(() -> jint);

    assert_eq!(sig.sig().to_bytes(), b"()I");
    assert_eq!(sig.args().len(), 0);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Int));
}

#[test]
fn test_mixed_builtin_rust_types() {
    let sig = jni_sig!((a: jint, b: JString, c: [java.lang.Object], d: JThrowable) -> void);

    assert_eq!(
        sig.sig().to_bytes(),
        b"(ILjava/lang/String;[Ljava/lang/Object;Ljava/lang/Throwable;)V"
    );
    assert_eq!(sig.args().len(), 4);
    assert_eq!(sig.args()[0], JavaType::Primitive(Primitive::Int));
    assert_eq!(sig.args()[1], JavaType::Object);
    assert_eq!(sig.args()[2], JavaType::Array);
    assert_eq!(sig.args()[3], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

// Field signature tests

#[test]
fn test_field_primitive() {
    let sig = jni_sig!(jint);

    assert_eq!(sig.sig().to_bytes(), b"I");
    assert_eq!(sig.ty(), JavaType::Primitive(Primitive::Int));
}

#[test]
fn test_field_object() {
    let sig = jni_sig!(java.lang.String);

    assert_eq!(sig.sig().to_bytes(), b"Ljava/lang/String;");
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_field_array() {
    let sig = jni_sig!([jint]);

    assert_eq!(sig.sig().to_bytes(), b"[I");
    assert_eq!(sig.ty(), JavaType::Array);
}

#[test]
fn test_field_array_suffix() {
    let sig = jni_sig!(java.lang.String[]);

    assert_eq!(sig.sig().to_bytes(), b"[Ljava/lang/String;");
    assert_eq!(sig.ty(), JavaType::Array);
}

#[test]
fn test_field_with_type_mapping() {
    let sig = jni_sig!(
        RustType,
        type_map = {
            RustType => java.lang.Type,
        }
    );

    assert_eq!(sig.sig().to_bytes(), b"Ljava/lang/Type;");
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_field_2d_array() {
    let sig = jni_sig!([[jint]]);

    assert_eq!(sig.sig().to_bytes(), b"[[I");
    assert_eq!(sig.ty(), JavaType::Array);
}

#[test]
fn test_field_inner_class() {
    let sig = jni_sig!(java.lang.Outer::Inner);

    assert_eq!(sig.sig().to_bytes(), b"Ljava/lang/Outer$Inner;");
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_field_default_package() {
    let sig = jni_sig!(.NoPackage);

    assert_eq!(sig.sig().to_bytes(), b"LNoPackage;");
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_multiple_inner_classes() {
    let sig = jni_sig!((a: java.lang.Outer::Inner::Nested) -> void);

    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/Outer$Inner$Nested;)V");
    assert_eq!(sig.args().len(), 1);
    assert_eq!(sig.args()[0], JavaType::Object);
}

#[test]
fn test_inner_class_field() {
    let sig = jni_sig!(java.lang.Thread::State);

    assert_eq!(sig.sig().to_bytes(), b"Ljava/lang/Thread$State;");
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_default_package_with_inner_class() {
    let sig = jni_sig!(.OuterClass::InnerClass);

    assert_eq!(sig.sig().to_bytes(), b"LOuterClass$InnerClass;");
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_array_of_inner_classes() {
    let sig = jni_sig!([java.lang.Thread::State]);

    assert_eq!(sig.sig().to_bytes(), b"[Ljava/lang/Thread$State;");
    assert_eq!(sig.ty(), JavaType::Array);
}

#[test]
fn test_inner_class_array_suffix() {
    let sig = jni_sig!(java.lang.Outer::Inner[][]);

    assert_eq!(sig.sig().to_bytes(), b"[[Ljava/lang/Outer$Inner;");
    assert_eq!(sig.ty(), JavaType::Array);
}

#[test]
fn test_deeply_nested_inner_classes() {
    let sig = jni_sig!(com.example.Outer::Middle::Inner::Deep);

    assert_eq!(
        sig.sig().to_bytes(),
        b"Lcom/example/Outer$Middle$Inner$Deep;"
    );
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_default_package_class_explicit() {
    let sig = jni_sig!((arg: .DefaultPackageClass) -> void);

    assert_eq!(sig.sig().to_bytes(), b"(LDefaultPackageClass;)V");
    assert_eq!(sig.args().len(), 1);
    assert_eq!(sig.args()[0], JavaType::Object);
}

#[test]
fn test_default_package_nested_inner_classes() {
    let sig = jni_sig!(.Outer::Inner1::Inner2);

    assert_eq!(sig.sig().to_bytes(), b"LOuter$Inner1$Inner2;");
    assert_eq!(sig.ty(), JavaType::Object);
}

// Test parsing of raw signature strings

#[test]
fn test_raw_method_signature_str() {
    let sig = jni_sig!("(ILjava/lang/String;)V");
    assert_eq!(sig.sig().as_ptr(), c"(ILjava/lang/String;)V".as_ptr());
    assert_eq!(sig.args().len(), 2);
    assert_eq!(sig.args()[0], JavaType::Primitive(Primitive::Int));
    assert_eq!(sig.args()[1], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_raw_method_signature_cstr() {
    let sig = jni_sig!(c"(Ljava/lang/String;)I");
    assert_eq!(sig.sig().as_ptr(), c"(Ljava/lang/String;)I".as_ptr());
    assert_eq!(sig.args().len(), 1);
    assert_eq!(sig.args()[0], JavaType::Object);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Int));
}

#[test]
fn test_raw_field_signature_primitive() {
    let sig = jni_sig!("I");
    assert_eq!(sig.sig().as_ptr(), c"I".as_ptr());
    assert_eq!(sig.ty(), JavaType::Primitive(Primitive::Int));
}

#[test]
fn test_raw_field_signature_object() {
    let sig = jni_sig!("Ljava/lang/String;");
    assert_eq!(sig.sig().as_ptr(), c"Ljava/lang/String;".as_ptr());
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_raw_field_signature_array() {
    let sig = jni_sig!("[I");
    assert_eq!(sig.sig().as_ptr(), c"[I".as_ptr());
    assert_eq!(sig.ty(), JavaType::Array);
}

#[test]
fn test_raw_field_signature_multidim_array() {
    let sig = jni_sig!("[[Ljava/lang/String;");
    assert_eq!(sig.sig().as_ptr(), c"[[Ljava/lang/String;".as_ptr());
    assert_eq!(sig.ty(), JavaType::Array);
}

#[test]
fn test_raw_method_with_arrays() {
    let sig = jni_sig!("([I[Ljava/lang/String;)[[I");
    assert_eq!(sig.sig().as_ptr(), c"([I[Ljava/lang/String;)[[I".as_ptr());
    assert_eq!(sig.args().len(), 2);
    assert_eq!(sig.args()[0], JavaType::Array);
    assert_eq!(sig.args()[1], JavaType::Array);
    assert_eq!(sig.ret(), JavaType::Array);
}

#[test]
fn test_raw_method_empty_args() {
    let sig = jni_sig!("()V");
    assert_eq!(sig.sig().as_ptr(), c"()V".as_ptr());
    assert_eq!(sig.args().len(), 0);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_raw_field_with_inner_class() {
    let sig = jni_sig!("Ljava/lang/Outer$Inner;");
    assert_eq!(sig.sig().as_ptr(), c"Ljava/lang/Outer$Inner;".as_ptr());
    assert_eq!(sig.ty(), JavaType::Object);
}

#[test]
fn test_raw_field_default_package() {
    let sig = jni_sig!("LNoPackage;");
    assert_eq!(sig.sig().as_ptr(), c"LNoPackage;".as_ptr());
    assert_eq!(sig.ty(), JavaType::Object);
}

// Tests for new named property syntax

#[test]
fn test_named_sig_property() {
    // Signature can be named with sig =
    let sig = jni_sig!(sig = (a: jint) -> void);
    assert_eq!(sig.sig().to_bytes(), b"(I)V");
    assert_eq!(sig.args().len(), 1);
    assert_eq!(sig.ret(), JavaType::Primitive(Primitive::Void));
}

#[test]
fn test_named_sig_property_field() {
    // Field signature can also be named
    let sig = jni_sig!(sig = jint);
    assert_eq!(sig.sig().to_bytes(), b"I");
    assert_eq!(sig.ty(), JavaType::Primitive(Primitive::Int));
}

#[test]
fn test_type_map_with_named_sig() {
    // type_map can be used with named sig in any order
    let sig1 = jni_sig!(
        sig = (a: RustType) -> void,
        type_map = {
            RustType => java.lang.Type,
        }
    );

    let sig2 = jni_sig!(
        type_map = {
            RustType => java.lang.Type,
        },
        sig = (a: RustType) -> void
    );

    assert_eq!(sig1.sig().to_bytes(), b"(Ljava/lang/Type;)V");
    assert_eq!(sig2.sig().to_bytes(), b"(Ljava/lang/Type;)V");
}

#[test]
fn test_jni_crate_override() {
    // Test that jni crate path can be overridden
    // This should work even though we're specifying a custom path
    let sig = jni_sig!(
        jni = ::jni,
        (a: jint) -> void
    );
    assert_eq!(sig.sig().to_bytes(), b"(I)V");
}

#[test]
fn test_all_properties_together() {
    // Test all properties together
    let sig = jni_sig!(
        jni = ::jni,
        type_map = {
            CustomType => java.lang.Custom,
        },
        sig = (a: CustomType) -> void,
    );
    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/Custom;)V");
}

#[test]
fn test_unnamed_sig_with_other_props() {
    // Unnamed signature can be anywhere (after jni property)
    let sig = jni_sig!(
        jni = ::jni,
        (a: CustomType) -> void,
        type_map = {
            CustomType => java.lang.Custom,
        }
    );
    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/Custom;)V");
}

#[test]
fn test_unnamed_sig_in_middle() {
    // Unnamed signature in the middle of named properties (after jni)
    let sig = jni_sig!(
        jni = ::jni,
        type_map = {
            CustomType => java.lang.Custom,
        },
        (a: CustomType) -> void
    );
    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/Custom;)V");
}

#[test]
fn test_unnamed_sig_at_end() {
    // Unnamed signature at the end
    let sig = jni_sig!(
        jni = ::jni,
        type_map = {
            CustomType => java.lang.Custom,
        },
        (a: CustomType) -> void
    );
    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/Custom;)V");
}

#[test]
fn test_unnamed_sig_with_trailing_comma() {
    // Unnamed signature at end with trailing comma
    let sig = jni_sig!(
        jni = ::jni,
        type_map = {
            CustomType => java.lang.Custom,
        },
        (a: CustomType) -> void,
    );
    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/Custom;)V");
}

#[test]
fn test_type_map_prefix_required() {
    // This should compile - type_map = is now required
    let sig = jni_sig!(
        CustomType,
        type_map = {
            CustomType => java.lang.Custom,
        }
    );
    assert_eq!(sig.sig().to_bytes(), b"Ljava/lang/Custom;");
}

// Tests for MUTF-8 encoding

#[test]
fn test_mutf8_encoding_basic_ascii() {
    use jni_macros::{jni_sig_cstr, jni_sig_jstr};

    // Basic ASCII should be the same in MUTF-8
    let sig = jni_sig!((a: java.lang.String) -> void);
    assert_eq!(sig.sig().to_bytes(), b"(Ljava/lang/String;)V");

    let cstr_sig = jni_sig_cstr!((a: java.lang.String) -> void);
    assert_eq!(cstr_sig.to_bytes(), b"(Ljava/lang/String;)V");

    let jstr_sig = jni_sig_jstr!((a: java.lang.String) -> void);
    assert_eq!(jstr_sig.to_bytes(), b"(Ljava/lang/String;)V");
}

#[test]
fn test_mutf8_encoding_unicode_emoji() {
    use jni_macros::{jni_sig_cstr, jni_sig_jstr};

    // Test Unicode emoji in class name - requires MUTF-8 encoding
    // The emoji 😀 (U+1F600) in MUTF-8 is encoded as surrogate pairs:
    // U+1F600 -> UTF-16 surrogate pair: D83D DE00
    // In MUTF-8: ED A0 BD ED B8 80
    let sig = jni_sig!((a: "unicode.Type😀") -> bool);
    let expected = b"(Lunicode/Type\xed\xa0\xbd\xed\xb8\x80;)Z";
    assert_eq!(sig.sig().to_bytes(), expected);

    let cstr_sig = jni_sig_cstr!((a: "unicode.Type😀") -> bool);
    assert_eq!(cstr_sig.to_bytes(), expected);

    let jstr_sig = jni_sig_jstr!((a: "unicode.Type😀") -> bool);
    assert_eq!(jstr_sig.to_bytes(), expected);
}

#[test]
fn test_mutf8_encoding_unicode_field() {
    use jni_macros::{jni_sig_cstr, jni_sig_jstr};

    // Test Unicode emoji in field signature
    let sig = jni_sig!("unicode.Field😀");
    let expected = b"Lunicode/Field\xed\xa0\xbd\xed\xb8\x80;";
    assert_eq!(sig.sig().to_bytes(), expected);

    let cstr_sig = jni_sig_cstr!("unicode.Field😀");
    assert_eq!(cstr_sig.to_bytes(), expected);

    let jstr_sig = jni_sig_jstr!("unicode.Field😀");
    assert_eq!(jstr_sig.to_bytes(), expected);
}

#[test]
fn test_mutf8_encoding_unicode_various_chars() {
    use jni_macros::jni_sig_cstr;

    // Test various Unicode characters
    // Japanese: こんにちは (Konnichiwa)
    let sig_jp = jni_sig_cstr!("jp.こんにちは");
    // こ (U+3053) -> E3 81 93
    // ん (U+3093) -> E3 82 93
    // に (U+306B) -> E3 81 AB
    // ち (U+3061) -> E3 81 A1
    // は (U+306F) -> E3 81 AF
    assert_eq!(
        sig_jp.to_bytes(),
        b"Ljp/\xe3\x81\x93\xe3\x82\x93\xe3\x81\xab\xe3\x81\xa1\xe3\x81\xaf;"
    );

    // Emoji combination: 👍 (U+1F44D)
    // U+1F44D -> UTF-16 surrogate pair: D83D DC4D
    // In MUTF-8: ED A0 BD ED B1 8D
    let sig_emoji = jni_sig_cstr!("emoji.👍");
    assert_eq!(sig_emoji.to_bytes(), b"Lemoji/\xed\xa0\xbd\xed\xb1\x8d;");
}

#[test]
fn test_all_three_macros_produce_same_encoding() {
    use jni_macros::{jni_sig_cstr, jni_sig_jstr};

    // All three macros should produce the same byte sequence for the signature
    let sig = jni_sig!((a: jint, b: java.lang.String) -> java.lang.Object);
    let cstr_sig = jni_sig_cstr!((a: jint, b: java.lang.String) -> java.lang.Object);
    let jstr_sig = jni_sig_jstr!((a: jint, b: java.lang.String) -> java.lang.Object);

    assert_eq!(sig.sig().to_bytes(), cstr_sig.to_bytes());
    assert_eq!(sig.sig().to_bytes(), jstr_sig.to_bytes());
    assert_eq!(
        sig.sig().to_bytes(),
        b"(ILjava/lang/String;)Ljava/lang/Object;"
    );
}

#[test]
fn test_const_evaluation_all_macros() {
    use jni::strings::JNIStr;
    use jni_macros::{jni_sig_cstr, jni_sig_jstr, jni_sig_str};
    use std::ffi::CStr;

    // Test that all macros work in const contexts
    const STR_SIG: &str = jni_sig_str!((a: jint) -> void);
    const CSTR_SIG: &CStr = jni_sig_cstr!((a: jint) -> void);
    const JSTR_SIG: &JNIStr = jni_sig_jstr!((a: jint) -> void);

    assert_eq!(STR_SIG, "(I)V");
    assert_eq!(CSTR_SIG.to_bytes(), b"(I)V");
    assert_eq!(JSTR_SIG.to_bytes(), b"(I)V");
}

// Tests for jni_sig_str! macro

#[test]
fn test_jni_sig_str_basic() {
    use jni_macros::jni_sig_str;

    const SIG: &str = jni_sig_str!((a: jint, b: jboolean) -> void);
    assert_eq!(SIG, "(IZ)V");
}

#[test]
fn test_jni_sig_str_object() {
    use jni_macros::jni_sig_str;

    const SIG: &str = jni_sig_str!((a: java.lang.String) -> java.lang.Object);
    assert_eq!(SIG, "(Ljava/lang/String;)Ljava/lang/Object;");
}

#[test]
fn test_jni_sig_str_field() {
    use jni_macros::jni_sig_str;

    const FIELD_SIG: &str = jni_sig_str!(java.lang.String);
    assert_eq!(FIELD_SIG, "Ljava/lang/String;");
}

#[test]
fn test_jni_sig_str_array() {
    use jni_macros::jni_sig_str;

    const SIG: &str = jni_sig_str!(([jint], [java.lang.String]) -> [[jint]]);
    assert_eq!(SIG, "([I[Ljava/lang/String;)[[I");
}

#[test]
fn test_jni_sig_str_with_type_map() {
    use jni_macros::jni_sig_str;

    const SIG: &str = jni_sig_str!(
        (a: CustomType) -> void,
        type_map = {
            CustomType => java.lang.Custom,
        }
    );
    assert_eq!(SIG, "(Ljava/lang/Custom;)V");
}

// Tests for jni_sig_cstr! macro

#[test]
fn test_jni_sig_cstr_basic() {
    use jni_macros::jni_sig_cstr;
    use std::ffi::CStr;

    const SIG: &CStr = jni_sig_cstr!((a: jint, b: jboolean) -> void);
    assert_eq!(SIG.to_bytes(), b"(IZ)V");
}

#[test]
fn test_jni_sig_cstr_object() {
    use jni_macros::jni_sig_cstr;
    use std::ffi::CStr;

    const SIG: &CStr = jni_sig_cstr!((a: java.lang.String) -> java.lang.Object);
    assert_eq!(SIG.to_bytes(), b"(Ljava/lang/String;)Ljava/lang/Object;");
}

#[test]
fn test_jni_sig_cstr_field() {
    use jni_macros::jni_sig_cstr;
    use std::ffi::CStr;

    const FIELD_SIG: &CStr = jni_sig_cstr!(java.lang.String);
    assert_eq!(FIELD_SIG.to_bytes(), b"Ljava/lang/String;");
}

#[test]
fn test_jni_sig_cstr_array() {
    use jni_macros::jni_sig_cstr;
    use std::ffi::CStr;

    const SIG: &CStr = jni_sig_cstr!(([jint], [java.lang.String]) -> [[jint]]);
    assert_eq!(SIG.to_bytes(), b"([I[Ljava/lang/String;)[[I");
}

// Tests for jni_sig_jstr! macro

#[test]
fn test_jni_sig_jstr_basic() {
    use jni::strings::JNIStr;
    use jni_macros::jni_sig_jstr;

    const SIG: &JNIStr = jni_sig_jstr!((a: jint, b: jboolean) -> void);
    assert_eq!(SIG.to_bytes(), b"(IZ)V");
}

#[test]
fn test_jni_sig_jstr_object() {
    use jni::strings::JNIStr;
    use jni_macros::jni_sig_jstr;

    const SIG: &JNIStr = jni_sig_jstr!((a: java.lang.String) -> java.lang.Object);
    assert_eq!(SIG.to_bytes(), b"(Ljava/lang/String;)Ljava/lang/Object;");
}

#[test]
fn test_jni_sig_jstr_field() {
    use jni::strings::JNIStr;
    use jni_macros::jni_sig_jstr;

    const FIELD_SIG: &JNIStr = jni_sig_jstr!(java.lang.String);
    assert_eq!(FIELD_SIG.to_bytes(), b"Ljava/lang/String;");
}

#[test]
fn test_jni_sig_jstr_array() {
    use jni::strings::JNIStr;
    use jni_macros::jni_sig_jstr;

    const SIG: &JNIStr = jni_sig_jstr!(([jint], [java.lang.String]) -> [[jint]]);
    assert_eq!(SIG.to_bytes(), b"([I[Ljava/lang/String;)[[I");
}

#[test]
fn test_jni_sig_jstr_with_type_map() {
    use jni::strings::JNIStr;
    use jni_macros::jni_sig_jstr;

    const SIG: &JNIStr = jni_sig_jstr!(
        (a: CustomType) -> void,
        type_map = {
            CustomType => java.lang.Custom,
        }
    );
    assert_eq!(SIG.to_bytes(), b"(Ljava/lang/Custom;)V");
}

#[test]
fn test_jni_sig_str_produces_utf8_string() {
    use jni_macros::jni_sig_str;

    // jni_sig_str! should produce a regular UTF-8 string
    const SIG: &str = jni_sig_str!((a: jint, b: java.lang.String) -> void);
    assert_eq!(SIG, "(ILjava/lang/String;)V");
    assert_eq!(SIG.as_bytes(), b"(ILjava/lang/String;)V");
}

#[test]
fn test_field_signatures_all_variants() {
    use jni_macros::{jni_sig_cstr, jni_sig_jstr, jni_sig_str};

    // Test that field signatures work with all macro variants
    let field_sig = jni_sig!(java.util.List);
    assert_eq!(field_sig.sig().to_bytes(), b"Ljava/util/List;");

    let cstr_field = jni_sig_cstr!(java.util.List);
    assert_eq!(cstr_field.to_bytes(), b"Ljava/util/List;");

    let jstr_field = jni_sig_jstr!(java.util.List);
    assert_eq!(jstr_field.to_bytes(), b"Ljava/util/List;");

    const STR_FIELD: &str = jni_sig_str!(java.util.List);
    assert_eq!(STR_FIELD, "Ljava/util/List;");
}

#[test]
fn test_raw_signature_all_variants() {
    use jni_macros::{jni_sig_cstr, jni_sig_jstr, jni_sig_str};

    // Test raw signatures with all macro variants
    let sig = jni_sig!("(ILjava/lang/String;)V");
    assert_eq!(sig.sig().to_bytes(), b"(ILjava/lang/String;)V");

    let cstr_sig = jni_sig_cstr!("(ILjava/lang/String;)V");
    assert_eq!(cstr_sig.to_bytes(), b"(ILjava/lang/String;)V");

    let jstr_sig = jni_sig_jstr!("(ILjava/lang/String;)V");
    assert_eq!(jstr_sig.to_bytes(), b"(ILjava/lang/String;)V");

    const STR_SIG: &str = jni_sig_str!("(ILjava/lang/String;)V");
    assert_eq!(STR_SIG, "(ILjava/lang/String;)V");
}

// Tests for jni_cstr! and jni_str! macros

#[test]
fn test_jni_cstr_basic() {
    use jni_macros::jni_cstr;
    use std::ffi::CStr;

    const CLASS_NAME: &CStr = jni_cstr!("java.lang.String");
    assert_eq!(CLASS_NAME.to_bytes(), b"java.lang.String");
}

#[test]
fn test_jni_cstr_unicode_emoji() {
    use jni_macros::jni_cstr;
    use std::ffi::CStr;

    // Test Unicode emoji (above U+FFFF) - requires MUTF-8 encoding with surrogate pairs
    const EMOJI_CLASS: &CStr = jni_cstr!("unicode.Type😀");
    // The emoji 😀 (U+1F600) in MUTF-8 is encoded as surrogate pairs:
    // U+1F600 -> UTF-16 surrogate pair: D83D DE00
    // In MUTF-8: ED A0 BD ED B8 80
    assert_eq!(
        EMOJI_CLASS.to_bytes(),
        b"unicode.Type\xed\xa0\xbd\xed\xb8\x80"
    );

    // Verify roundtrip decode
    let decoded = cesu8::from_java_cesu8(EMOJI_CLASS.to_bytes()).expect("Failed to decode MUTF-8");
    assert_eq!(decoded, "unicode.Type😀");
}

#[test]
fn test_jni_str_basic() {
    use jni::strings::JNIStr;
    use jni_macros::jni_str;

    const CLASS_NAME: &JNIStr = jni_str!("java.lang.String");
    assert_eq!(CLASS_NAME.to_bytes(), b"java.lang.String");
}

#[test]
fn test_jni_str_unicode_emoji() {
    use jni::strings::JNIStr;
    use jni_macros::jni_str;

    // Test Unicode emoji (above U+FFFF) - requires MUTF-8 encoding with surrogate pairs
    const EMOJI_CLASS: &JNIStr = jni_str!("unicode.Type😀");
    assert_eq!(
        EMOJI_CLASS.to_bytes(),
        b"unicode.Type\xed\xa0\xbd\xed\xb8\x80"
    );

    // Verify roundtrip decode
    let decoded = cesu8::from_java_cesu8(EMOJI_CLASS.to_bytes()).expect("Failed to decode MUTF-8");
    assert_eq!(decoded, "unicode.Type😀");
}

#[test]
fn test_jni_cstr_and_jni_str_produce_same_encoding() {
    use jni_macros::{jni_cstr, jni_str};

    // Test that jni_cstr! and jni_str! produce the same byte encoding
    const CSTR: &std::ffi::CStr = jni_cstr!("java.util.ArrayList");
    const JSTR: &jni::strings::JNIStr = jni_str!("java.util.ArrayList");

    assert_eq!(CSTR.to_bytes(), JSTR.to_bytes());
    assert_eq!(CSTR.to_bytes(), b"java.util.ArrayList");
}

#[test]
fn test_jni_cstr_const_evaluation() {
    use jni_macros::jni_cstr;
    use std::ffi::CStr;

    // Verify that jni_cstr! works in const contexts
    const PATH_SEP: &CStr = jni_cstr!("/");
    const PACKAGE: &CStr = jni_cstr!("com.example");
    const NESTED: &CStr = jni_cstr!("com.example.nested.Class");

    assert_eq!(PATH_SEP.to_bytes(), b"/");
    assert_eq!(PACKAGE.to_bytes(), b"com.example");
    assert_eq!(NESTED.to_bytes(), b"com.example.nested.Class");
}

#[test]
fn test_jni_str_const_evaluation() {
    use jni::strings::JNIStr;
    use jni_macros::jni_str;

    // Verify that jni_str! works in const contexts
    const PATH_SEP: &JNIStr = jni_str!("/");
    const PACKAGE: &JNIStr = jni_str!("com.example");
    const NESTED: &JNIStr = jni_str!("com.example.nested.Class");

    assert_eq!(PATH_SEP.to_bytes(), b"/");
    assert_eq!(PACKAGE.to_bytes(), b"com.example");
    assert_eq!(NESTED.to_bytes(), b"com.example.nested.Class");
}

#[test]
fn test_jni_cstr_concat_multiple_literals() {
    use jni_macros::jni_cstr;
    use std::ffi::CStr;

    // Test concat-like behavior with multiple string literals
    const CONCATENATED: &CStr = jni_cstr!("java", ".", "lang", ".", "String");
    assert_eq!(CONCATENATED.to_bytes(), b"java.lang.String");

    const TWO_PART: &CStr = jni_cstr!("com.example", ".MyClass");
    assert_eq!(TWO_PART.to_bytes(), b"com.example.MyClass");

    const WITH_TRAILING_COMMA: &CStr = jni_cstr!("hello", " ", "world",);
    assert_eq!(WITH_TRAILING_COMMA.to_bytes(), b"hello world");
}

#[test]
fn test_jni_str_concat_multiple_literals() {
    use jni::strings::JNIStr;
    use jni_macros::jni_str;

    // Test concat-like behavior with multiple string literals
    const CONCATENATED: &JNIStr = jni_str!("java", ".", "lang", ".", "String");
    assert_eq!(CONCATENATED.to_bytes(), b"java.lang.String");

    const TWO_PART: &JNIStr = jni_str!("com.example", ".MyClass");
    assert_eq!(TWO_PART.to_bytes(), b"com.example.MyClass");

    const WITH_TRAILING_COMMA: &JNIStr = jni_str!("hello", " ", "world",);
    assert_eq!(WITH_TRAILING_COMMA.to_bytes(), b"hello world");
}

#[test]
fn test_jni_str_with_jni_crate_override() {
    use jni_macros::jni_str;

    // Test that we can override the jni crate path
    // This should compile successfully with jni = jni
    const WITH_JNI_OVERRIDE: &jni::strings::JNIStr = jni_str!(jni = jni, "test.Class");
    assert_eq!(WITH_JNI_OVERRIDE.to_bytes(), b"test.Class");

    // Test with multiple literals after jni override
    const MULTI_WITH_OVERRIDE: &jni::strings::JNIStr = jni_str!(jni = jni, "test", ".", "Class");
    assert_eq!(MULTI_WITH_OVERRIDE.to_bytes(), b"test.Class");
}

#[test]
fn test_jni_cstr_and_jni_str_concat_same_encoding() {
    use jni_macros::{jni_cstr, jni_str};

    // Test that jni_cstr! and jni_str! produce the same byte encoding when concatenating
    const CSTR: &std::ffi::CStr = jni_cstr!("java", ".", "util", ".", "ArrayList");
    const JSTR: &jni::strings::JNIStr = jni_str!("java", ".", "util", ".", "ArrayList");

    assert_eq!(CSTR.to_bytes(), JSTR.to_bytes());
    assert_eq!(CSTR.to_bytes(), b"java.util.ArrayList");
}

#[test]
fn test_jni_cstr_mixed_literal_types() {
    use jni_macros::jni_cstr;
    use std::ffi::CStr;

    // Test mixing different literal types
    const PORT: &CStr = jni_cstr!("localhost:", 8080);
    assert_eq!(PORT.to_bytes(), b"localhost:8080");

    const VERSION: &CStr = jni_cstr!("Version ", 1, '.', 2, '.', 3);
    assert_eq!(VERSION.to_bytes(), b"Version 1.2.3");

    const FLAG: &CStr = jni_cstr!("enabled=", true);
    assert_eq!(FLAG.to_bytes(), b"enabled=true");
}

#[test]
fn test_jni_str_mixed_literal_types() {
    use jni::strings::JNIStr;
    use jni_macros::jni_str;

    // Test mixing different literal types
    const PORT: &JNIStr = jni_str!("localhost:", 8080);
    assert_eq!(PORT.to_bytes(), b"localhost:8080");

    const VERSION: &JNIStr = jni_str!("v", 2, '.', 0);
    assert_eq!(VERSION.to_bytes(), b"v2.0");

    const BYTE_VAL: &JNIStr = jni_str!("byte=", b'A');
    assert_eq!(BYTE_VAL.to_bytes(), b"byte=65"); // ASCII value of 'A'
}

#[test]
fn test_jni_cstr_float_literal() {
    use jni_macros::jni_cstr;
    use std::ffi::CStr;

    const PI: &CStr = jni_cstr!("pi=", 3.14159);
    assert_eq!(PI.to_bytes(), b"pi=3.14159");
}

#[test]
fn test_jni_str_cstr_literal() {
    use jni::strings::JNIStr;
    use jni_macros::jni_str;

    // CStr literals can be used if they're valid UTF-8
    const HELLO: &JNIStr = jni_str!(c"Hello", " ", c"World");
    assert_eq!(HELLO.to_bytes(), b"Hello World");
}
