//! Tests for @RustName annotation support

use jbindgen::Builder;
use std::path::PathBuf;

#[test]
fn test_rust_name_annotation_parsing() {
    let test_file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/java/AnnotationTest.java");

    // Add the annotations source directory to classpath so the annotation is available
    let annotations_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("annotations/src/main/java");

    let sources = vec![test_file];
    let classpath = vec![annotations_dir];
    let patterns = vec!["com.example.test.*".to_string()];

    // Use the Builder API to parse and generate bindings
    let output = Builder::new()
        .input_sources(sources, classpath, patterns)
        .root_path("crate")
        .generate()
        .expect("Failed to generate bindings");

    let code = output.to_string();

    // Test that class name override is used
    assert!(
        code.contains("pub CustomAnnotatedClass =>"),
        "Generated code should use CustomAnnotatedClass from @RustName"
    );

    // Test field name overrides
    assert!(
        code.contains("custom_static_field"),
        "Generated code should use custom_static_field from @RustName"
    );
    assert!(
        code.contains("custom_instance_field"),
        "Generated code should use custom_instance_field from @RustName"
    );
    assert!(
        code.contains("normal_field"),
        "Generated code should have normal_field without override"
    );

    // Test constructor name overrides
    assert!(
        code.contains("fn new_default()"),
        "Generated code should use new_default from @RustName"
    );
    assert!(
        code.contains("fn new_with_value("),
        "Generated code should use new_with_value from @RustName"
    );

    // Test method name overrides
    assert!(
        code.contains("custom_static_method"),
        "Generated code should use custom_static_method from @RustName"
    );
    assert!(
        code.contains("custom_instance_method"),
        "Generated code should use custom_instance_method from @RustName"
    );
    assert!(
        code.contains("normal_method"),
        "Generated code should have normal_method without override"
    );

    // Test native method name overrides
    assert!(
        code.contains("custom_native_method"),
        "Generated code should use custom_native_method from @RustName"
    );
    assert!(
        code.contains("normal_native_method"),
        "Generated code should have normal_native_method without override"
    );

    // Verify the Java type name is correct (not overridden)
    assert!(
        code.contains("\"com.example.test.AnnotationTest\""),
        "Java type name should remain unchanged"
    );
}

#[test]
fn test_rust_name_takes_precedence_over_auto_naming() {
    let test_file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/java/AnnotationTest.java");

    let annotations_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("annotations/src/main/java");

    let sources = vec![test_file];
    let classpath = vec![annotations_dir];
    let patterns = vec!["com.example.test.*".to_string()];

    let output = Builder::new()
        .input_sources(sources, classpath, patterns)
        .root_path("crate")
        .generate()
        .expect("Failed to generate bindings");

    let code = output.to_string();

    // The staticMethod would normally be converted to static_method
    // but with @RustName it should be custom_static_method
    assert!(
        code.contains("custom_static_method"),
        "@RustName should override automatic snake_case conversion"
    );

    // Verify that the automatic name (static_method) is NOT present as a function name
    // The automatic conversion would be "fn static_method" or "static fn static_method"
    // We should have "static fn custom_static_method" instead
    assert!(
        !code.contains("fn static_method"),
        "Automatic name 'fn static_method' should not be used when @RustName is present"
    );
}

#[test]
fn test_mixed_annotated_and_non_annotated_members() {
    let test_file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/java/AnnotationTest.java");

    let annotations_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("annotations/src/main/java");

    let sources = vec![test_file];
    let classpath = vec![annotations_dir];
    let patterns = vec!["com.example.test.*".to_string()];

    let output = Builder::new()
        .input_sources(sources, classpath, patterns)
        .root_path("crate")
        .generate()
        .expect("Failed to generate bindings");

    let code = output.to_string();

    // Count how many constructors are present
    // We should have: new_default, new_with_value, and one for int parameter
    let new_count = code.matches("fn new").count();
    assert!(
        new_count >= 3,
        "Should have at least 3 constructors (2 with @RustName, 1 without)"
    );

    // Verify both annotated and non-annotated fields are present
    assert!(
        code.contains("custom_static_field") && code.contains("normal_field"),
        "Both annotated and non-annotated fields should be present"
    );

    // Verify both annotated and non-annotated methods are present
    assert!(
        code.contains("custom_instance_method") && code.contains("normal_method"),
        "Both annotated and non-annotated methods should be present"
    );
}

#[test]
fn test_rust_primitive_annotation() {
    let test_file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/java/RustPrimitiveTest.java");

    let annotations_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("annotations/src/main/java");

    let sources = vec![test_file];
    let classpath = vec![annotations_dir];
    let patterns = vec!["com.example.test.*".to_string()];

    // Set up type mappings for the primitive handle types
    let output = Builder::new()
        .input_sources(sources, classpath, patterns)
        .root_path("crate")
        .type_mapping("unsafe ThingHandle", "long")
        .type_mapping("unsafe OtherHandle", "long")
        .generate()
        .expect("Failed to generate bindings");

    let code = output.to_string();

    // Verify that ThingHandle is used instead of jlong in native method signatures
    assert!(
        code.contains("fn process_handle") && code.contains("handle: ThingHandle"),
        "Native method should use ThingHandle type from @RustPrimitive annotation"
    );

    // Verify OtherHandle is used in the parameter (not return type which remains jlong)
    assert!(
        code.contains("fn create_handle") && code.contains("existingHandle: OtherHandle"),
        "Native method should use OtherHandle for annotated parameters"
    );

    // Verify mixed parameters work correctly
    assert!(
        code.contains("fn process_multiple"),
        "Native method with mixed parameters should be present"
    );
    assert!(
        code.contains("handle1: ThingHandle"),
        "First handle parameter should use ThingHandle"
    );
    assert!(
        code.contains("normalInt: jint"),
        "Non-annotated int should use jint"
    );
    assert!(
        code.contains("handle2: ThingHandle"),
        "Second handle parameter should use ThingHandle"
    );
    assert!(
        code.contains("normalLong: jlong"),
        "Non-annotated long should use jlong"
    );

    // Verify static native method works
    assert!(
        code.contains("static") && code.contains("fn static_with_handle"),
        "Static native method should be present"
    );
    assert!(
        code.contains("handle: ThingHandle"),
        "Static native method should use ThingHandle"
    );

    // Verify that unsafe type mappings are included in the type_map
    assert!(
        code.contains("type_map = {"),
        "Generated code should have a type_map block"
    );
    assert!(
        code.contains("unsafe ThingHandle => long"),
        "type_map should include unsafe ThingHandle => long mapping"
    );
    assert!(
        code.contains("unsafe OtherHandle => long"),
        "type_map should include unsafe OtherHandle => long mapping"
    );
}

#[test]
fn test_rust_primitive_without_type_mapping_fails() {
    let test_file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/java/RustPrimitiveTest.java");

    let annotations_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("annotations/src/main/java");

    let sources = vec![test_file];
    let classpath = vec![annotations_dir];
    let patterns = vec!["com.example.test.*".to_string()];

    // Try to generate without providing the required type mapping
    let result = Builder::new()
        .input_sources(sources, classpath, patterns)
        .root_path("crate")
        // Intentionally NOT adding type_mapping for ThingHandle
        .generate();

    // Should fail with an error about missing primitive type mapping
    assert!(
        result.is_err(),
        "Generation should fail when @RustPrimitive type mapping is not provided"
    );

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(
        err_msg.contains("RustPrimitive") && err_msg.contains("ThingHandle"),
        "Error message should mention the missing RustPrimitive mapping for ThingHandle, got: {}",
        err_msg
    );
}
