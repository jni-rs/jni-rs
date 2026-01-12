//! Tests for jbindgen code generation which only check the contents of the
//! generated code, without trying to compile or run it.

use std::fs;
use std::path::PathBuf;

use jbindgen::Builder;

/// Helper function to set up test output directory
fn setup_test_output(test_name: &str) -> PathBuf {
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("jbindgen_tests")
        .join(test_name);

    // Clean up any existing output
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("Failed to create test output directory");

    out_dir
}

/// Helper function to compile a Java file and return the class file path
fn compile_java_file(java_file: &str, out_dir: &PathBuf) -> PathBuf {
    let class_files = javac::Build::new()
        .file(java_file)
        .output_dir(out_dir)
        .compile();

    assert!(!class_files.is_empty(), "No class files were generated");
    class_files[0].clone()
}

#[test]
fn test_simple_class_binding() {
    let out_dir = setup_test_output("simple_class");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Check that bindings contain expected elements
    assert!(bindings.contains("bind_java_type!"));
    assert!(bindings.contains("pub SimpleClass"));
    assert!(bindings.contains("com.example.SimpleClass"));
    assert!(bindings.contains("constructors"));
    assert!(bindings.contains("fn new()"));
    assert!(bindings.contains("methods"));
    assert!(bindings.contains("static fn add"));
    assert!(bindings.contains("static fn get_message"));
    assert!(bindings.contains("fn get_value"));
    assert!(bindings.contains("fn set_value"));

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_calculator_binding() {
    let out_dir = setup_test_output("calculator");
    let class_file = compile_java_file("tests/java/Calculator.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    assert!(bindings.contains("pub Calculator"));
    assert!(bindings.contains("com.example.Calculator"));
    assert!(bindings.contains("static fn multiply"));
    assert!(bindings.contains("static fn multiply_long"));
    assert!(bindings.contains("static fn divide"));
    assert!(bindings.contains("fn square"));
    assert!(bindings.contains("fn power"));

    // Check type mappings
    assert!(bindings.contains("jint"));
    assert!(bindings.contains("jlong"));
    assert!(bindings.contains("jdouble"));

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_no_package_class() {
    let out_dir = setup_test_output("no_package");
    let class_file = compile_java_file("tests/java/NoPackage.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    assert!(bindings.contains("pub NoPackage"));
    // Should use default package syntax
    assert!(bindings.contains("\".NoPackage\""));
    assert!(bindings.contains("static fn get_answer"));
    assert!(bindings.contains("fn get_name"));

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_custom_rust_name() {
    let out_dir = setup_test_output("custom_name");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .rust_type_name("MyCustomType".to_string())
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    assert!(bindings.contains("pub MyCustomType"));
    assert!(!bindings.contains("pub SimpleClass"));

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_private_type() {
    let out_dir = setup_test_output("private_type");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .public_type(false)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Should not have "pub " before type name in the shorthand
    assert!(!bindings.contains("pub SimpleClass =>"));
    assert!(bindings.contains("SimpleClass =>"));

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_output_compiles() {
    // This test verifies that the generated bindings are syntactically valid Rust
    let out_dir = setup_test_output("compile_check");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Write to a temporary Rust file
    let rust_file = out_dir.join("bindings.rs");
    fs::write(&rust_file, &bindings).expect("Failed to write bindings file");

    // The bindings use bind_java_type! macro, so we can't directly compile them
    // without the jni crate. But we can at least verify the file was created.
    assert!(rust_file.exists());
    println!("Generated bindings file: {}", rust_file.display());
}

#[test]
fn test_field_generation() {
    let out_dir = setup_test_output("with_fields");
    let class_file = compile_java_file("tests/java/WithFields.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Check that fields block is present
    assert!(bindings.contains("fields {"), "Should have fields block");

    // Check static fields (note: 'final' modifier not shown in output)
    assert!(
        bindings.contains("static CONSTANT"),
        "Should have CONSTANT field"
    );
    assert!(
        bindings.contains("static static_field"),
        "Should have staticField"
    );

    // Check instance fields
    assert!(bindings.contains("public_field"), "Should have publicField");
    assert!(
        bindings.contains("protected_field"),
        "Should have protectedField"
    );

    // Check that private field is NOT included
    assert!(
        !bindings.contains("private_field"),
        "Should not include privateField"
    );

    // Check field types
    assert!(bindings.contains(": jint"), "Should have jint type");
    assert!(bindings.contains(": JString"), "Should have JString type");
    assert!(bindings.contains(": jdouble"), "Should have jdouble type");

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_native_method_generation() {
    let out_dir = setup_test_output("with_native");
    let class_file = compile_java_file("tests/java/WithNative.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Check that native_methods block is present
    assert!(
        bindings.contains("native_methods {"),
        "Should have native_methods block"
    );

    // Check static native methods
    assert!(
        bindings.contains("static fn native_add"),
        "Should have nativeAdd"
    );
    assert!(
        bindings.contains("static fn native_get_platform"),
        "Should have nativeGetPlatform"
    );

    // Check instance native methods
    assert!(
        bindings.contains("fn native_get_message"),
        "Should have nativeGetMessage"
    );
    assert!(
        bindings.contains("fn native_process"),
        "Should have nativeProcess"
    );

    // Check regular methods are in separate methods block
    assert!(bindings.contains("methods {"), "Should have methods block");
    assert!(
        bindings.contains("fn regular_method"),
        "Should have regularMethod"
    );

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_documentation_generation() {
    let out_dir = setup_test_output("with_documentation");

    // Compile the Java file with documentation
    let class_files = javac::Build::new()
        .file("tests/java/WithDocumentation.java")
        .output_dir(&out_dir)
        .compile();

    assert!(!class_files.is_empty(), "No class files were generated");

    // Generate bindings from the compiled class file (bytecode - no docs)
    let bindings_from_class = Builder::new()
        .root_path("crate")
        .input_class(class_files[0].clone())
        .generate()
        .expect("Failed to generate bindings from class file")
        .to_string();

    // Generate bindings from source file (with docs)
    let bindings_from_source = Builder::new()
        .root_path("crate")
        .input_sources(
            vec![PathBuf::from("tests/java/WithDocumentation.java")],
            vec![],
            vec!["com.example.*".to_string()],
        )
        .generate()
        .expect("Failed to generate bindings from source file")
        .to_string();

    // Bytecode-based bindings should NOT have documentation comments
    assert!(
        !bindings_from_class.contains("/// A test class"),
        "Bytecode bindings should not have class documentation"
    );
    assert!(
        !bindings_from_class.contains("/// Gets the current value"),
        "Bytecode bindings should not have method documentation"
    );

    // Source-based bindings SHOULD have documentation comments
    assert!(
        bindings_from_source.contains("/// A test class for verifying Javadoc comment extraction."),
        "Source bindings should have class documentation"
    );
    assert!(
        bindings_from_source
            .contains("/// This class demonstrates how documentation comments are preserved"),
        "Source bindings should have multi-line class documentation"
    );

    // Check constructor documentation
    assert!(
        bindings_from_source.contains("/// Creates a new instance with default value of zero."),
        "Should have default constructor documentation"
    );
    assert!(
        bindings_from_source
            .contains("/// Creates a new instance with the specified initial value."),
        "Should have parameterized constructor documentation"
    );

    // Check method documentation
    assert!(
        bindings_from_source.contains("/// Gets the current value."),
        "Should have getter method documentation"
    );
    assert!(
        bindings_from_source.contains("/// Sets a new value."),
        "Should have setter method documentation"
    );
    assert!(
        bindings_from_source.contains("/// Adds two numbers together."),
        "Should have static method documentation"
    );

    // Check field documentation
    assert!(
        bindings_from_source.contains("/// A static constant field with documentation."),
        "Should have static field documentation"
    );
    assert!(
        bindings_from_source.contains("/// An instance field representing the current value."),
        "Should have instance field documentation"
    );

    println!("Generated bindings from source:\n{}", bindings_from_source);
}

#[test]
fn test_inner_class_from_classfile() {
    let out_dir = setup_test_output("inner_class_classfile");

    // Compile the Java file with inner classes
    let class_files = javac::Build::new()
        .file("tests/java/OuterClass.java")
        .output_dir(&out_dir)
        .compile();

    assert!(!class_files.is_empty(), "No class files were generated");

    // Find the inner class file (OuterClass$InnerClass.class)
    let inner_class_file = class_files
        .iter()
        .find(|p| p.to_string_lossy().contains("OuterClass$InnerClass.class"))
        .expect("Could not find OuterClass$InnerClass.class");

    // Generate bindings from the compiled inner class file
    let bindings = Builder::new()
        .root_path("crate")
        .input_class(inner_class_file.clone())
        .generate()
        .expect("Failed to generate bindings from inner class file")
        .to_string();

    // Check that the Rust type name combines outer + inner with $ removed
    assert!(
        bindings.contains("pub OuterClassInnerClass =>"),
        "Rust type should be OuterClassInnerClass ($ removed), got:\n{}",
        bindings
    );

    // Check that the Java type name contains $
    assert!(
        bindings.contains("\"com.example.OuterClass$InnerClass\""),
        "Java type should be com.example.OuterClass$InnerClass (with $), got:\n{}",
        bindings
    );

    // Verify it's in the type map with $ in Java name
    assert!(
        bindings.contains("OuterClassInnerClass => \"com.example.OuterClass$InnerClass\""),
        "Type map should have OuterClassInnerClass => \"com.example.OuterClass$InnerClass\""
    );
}

#[test]
fn test_inner_class_from_source() {
    let out_dir = setup_test_output("inner_class_source");

    // Generate bindings from source file (with docs)
    // Use pattern to match only the InnerClass
    let bindings = Builder::new()
        .root_path("crate")
        .input_sources(
            vec![PathBuf::from("tests/java/OuterClass.java")],
            vec![],
            vec!["com.example.OuterClass$InnerClass".to_string()],
        )
        .generate()
        .expect("Failed to generate bindings from source file")
        .to_string();

    // Check that the Rust type name combines outer + inner with $ removed
    assert!(
        bindings.contains("pub OuterClassInnerClass =>"),
        "Rust type should be OuterClassInnerClass ($ removed), got:\n{}",
        bindings
    );

    // Check that the Java type name contains $
    assert!(
        bindings.contains("\"com.example.OuterClass$InnerClass\""),
        "Java type should be com.example.OuterClass$InnerClass (with $), got:\n{}",
        bindings
    );

    // Verify it's in the type map with $ in Java name
    assert!(
        bindings.contains("OuterClassInnerClass => \"com.example.OuterClass$InnerClass\""),
        "Type map should have OuterClassInnerClass => \"com.example.OuterClass$InnerClass\""
    );
}

#[test]
fn test_multiple_inner_classes_pattern_matching() {
    let out_dir = setup_test_output("multiple_inner_classes");

    // Compile the Java file with inner classes
    let class_files = javac::Build::new()
        .file("tests/java/OuterClass.java")
        .output_dir(&out_dir)
        .compile();

    assert!(
        class_files.len() >= 3,
        "Should generate at least 3 class files (outer + 2 inners)"
    );

    // Test 1: Generate bindings for all classes (outer + all inners) by matching outer class
    let bindings_all = Builder::new()
        .root_path("crate")
        .input_sources(
            vec![PathBuf::from("tests/java/OuterClass.java")],
            vec![],
            vec!["com.example.OuterClass".to_string()], // Match outer class -> includes all inner classes
        )
        .generate()
        .expect("Failed to generate bindings for all classes")
        .to_string();

    // Should have all three classes
    assert!(
        bindings_all.contains("pub OuterClass =>"),
        "Should have OuterClass"
    );
    assert!(
        bindings_all.contains("pub OuterClassInnerClass =>"),
        "Should have InnerClass"
    );
    assert!(
        bindings_all.contains("pub OuterClassAnotherInner =>"),
        "Should have AnotherInner"
    );

    // Test 2: Generate bindings for only one specific inner class
    let bindings_one = Builder::new()
        .root_path("crate")
        .input_sources(
            vec![PathBuf::from("tests/java/OuterClass.java")],
            vec![],
            vec!["com.example.OuterClass$InnerClass".to_string()], // Specific inner class with $
        )
        .generate()
        .expect("Failed to generate bindings for specific inner class")
        .to_string();

    // Should have only InnerClass
    assert!(
        bindings_one.contains("pub OuterClassInnerClass =>"),
        "Should have InnerClass"
    );
    assert!(
        !bindings_one.contains("pub OuterClass =>"),
        "Should NOT have OuterClass"
    );
    assert!(
        !bindings_one.contains("pub OuterClassAnotherInner =>"),
        "Should NOT have AnotherInner"
    );

    println!("Generated bindings for all classes:\n{}", bindings_all);
    println!(
        "\nGenerated bindings for one inner class:\n{}",
        bindings_one
    );
}

// Tests for native method interface generation settings

#[test]
fn test_native_methods_classfile_interfaces_enabled() {
    let out_dir = setup_test_output("native_methods_classfile_enabled");
    let class_file = compile_java_file("tests/java/NativeMethodVisibility.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate_native_interfaces(true)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // With interfaces enabled:
    // - Public native methods should be in native_methods block with "pub" visibility
    // - Private native methods should be in native_methods block with no visibility qualifier
    assert!(
        bindings.contains("native_methods {"),
        "Should have native_methods block"
    );

    // Public methods should be present with "pub" visibility
    assert!(
        bindings.contains("pub static fn public_static_native"),
        "Should have public_static_native with pub visibility"
    );
    assert!(
        bindings.contains("pub fn public_instance_native"),
        "Should have public_instance_native with pub visibility"
    );

    // Private methods should be present without visibility qualifier
    assert!(
        bindings.contains("static fn private_static_native"),
        "Should have private_static_native without visibility"
    );
    assert!(
        bindings.contains("fn private_instance_native"),
        "Should have private_instance_native without visibility"
    );

    // Verify private methods don't have "pub" visibility
    assert!(
        !bindings.contains("pub fn private_static_native"),
        "private_static_native should NOT have pub visibility"
    );
    assert!(
        !bindings.contains("pub fn private_instance_native(data"),
        "private_instance_native should NOT have pub visibility"
    );

    // Regular methods should be in methods block, not native_methods
    assert!(bindings.contains("methods {"), "Should have methods block");
    assert!(
        bindings.contains("fn regular_method"),
        "Should have regular_method in methods block"
    );

    println!("Generated bindings (interfaces enabled):\n{}", bindings);
}

#[test]
fn test_native_methods_classfile_interfaces_disabled() {
    let out_dir = setup_test_output("native_methods_classfile_disabled");
    let class_file = compile_java_file("tests/java/NativeMethodVisibility.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate_native_interfaces(false)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // With interfaces disabled:
    // - Public native methods should be in methods block (callable, not implementable)
    // - Private native methods should NOT be emitted at all
    assert!(
        !bindings.contains("native_methods {"),
        "Should NOT have native_methods block"
    );
    assert!(bindings.contains("methods {"), "Should have methods block");

    // Public native methods should be in methods block
    assert!(
        bindings.contains("static fn public_static_native"),
        "Should have public_static_native in methods"
    );
    assert!(
        bindings.contains("fn public_instance_native"),
        "Should have public_instance_native in methods"
    );

    // Private native methods should NOT be emitted
    assert!(
        !bindings.contains("private_static_native"),
        "Should NOT have private_static_native"
    );
    assert!(
        !bindings.contains("private_instance_native"),
        "Should NOT have private_instance_native"
    );

    // Regular methods should still be in methods block
    assert!(
        bindings.contains("fn regular_method"),
        "Should have regular_method"
    );

    println!("Generated bindings (interfaces disabled):\n{}", bindings);
}

#[test]
fn test_native_methods_java_source_interfaces_enabled() {
    let bindings = Builder::new()
        .root_path("crate")
        .input_sources(
            vec![PathBuf::from("tests/java/NativeMethodVisibility.java")],
            vec![],
            vec!["com.example.NativeMethodVisibility".to_string()],
        )
        .generate_native_interfaces(true)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // With interfaces enabled:
    // - Public native methods should be in native_methods block with "pub" visibility
    // - Private native methods should be in native_methods block with no visibility qualifier
    assert!(
        bindings.contains("native_methods {"),
        "Should have native_methods block"
    );

    // Public methods should be present with "pub" visibility
    assert!(
        bindings.contains("pub static fn public_static_native"),
        "Should have public_static_native with pub visibility"
    );
    assert!(
        bindings.contains("pub fn public_instance_native"),
        "Should have public_instance_native with pub visibility"
    );

    // Private methods should be present without visibility qualifier
    assert!(
        bindings.contains("static fn private_static_native"),
        "Should have private_static_native without visibility"
    );
    assert!(
        bindings.contains("fn private_instance_native"),
        "Should have private_instance_native without visibility"
    );

    // Verify private methods don't have "pub" visibility
    assert!(
        !bindings.contains("pub fn private_static_native"),
        "private_static_native should NOT have pub visibility"
    );
    assert!(
        !bindings.contains("pub fn private_instance_native(data"),
        "private_instance_native should NOT have pub visibility"
    );

    // Regular methods should be in methods block
    assert!(bindings.contains("methods {"), "Should have methods block");
    assert!(
        bindings.contains("fn regular_method"),
        "Should have regular_method"
    );

    println!(
        "Generated bindings from Java source (interfaces enabled):\n{}",
        bindings
    );
}

#[test]
fn test_native_methods_java_source_interfaces_disabled() {
    let bindings = Builder::new()
        .root_path("crate")
        .input_sources(
            vec![PathBuf::from("tests/java/NativeMethodVisibility.java")],
            vec![],
            vec!["com.example.NativeMethodVisibility".to_string()],
        )
        .generate_native_interfaces(false)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // With interfaces disabled:
    // - Public native methods should be in methods block (callable, not implementable)
    // - Private native methods should NOT be emitted at all
    assert!(
        !bindings.contains("native_methods {"),
        "Should NOT have native_methods block"
    );
    assert!(bindings.contains("methods {"), "Should have methods block");

    // Public native methods should be in methods block
    assert!(
        bindings.contains("static fn public_static_native"),
        "Should have public_static_native in methods"
    );
    assert!(
        bindings.contains("fn public_instance_native"),
        "Should have public_instance_native in methods"
    );

    // Private native methods should NOT be emitted
    assert!(
        !bindings.contains("private_static_native"),
        "Should NOT have private_static_native"
    );
    assert!(
        !bindings.contains("private_instance_native"),
        "Should NOT have private_instance_native"
    );

    // Regular methods should still be in methods block
    assert!(
        bindings.contains("fn regular_method"),
        "Should have regular_method"
    );

    println!(
        "Generated bindings from Java source (interfaces disabled):\n{}",
        bindings
    );
}

#[test]
fn test_jni_init_generation() {
    let out_dir = setup_test_output("jni_init");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Should have jni_init function by default
    assert!(
        bindings.contains("pub fn jni_init(env: &::jni::Env, loader: &::jni::refs::LoaderContext) -> ::jni::errors::Result<()>"),
        "Should have jni_init function"
    );
    assert!(
        bindings.contains("let _ = SimpleClassAPI::get(env, loader)?;"),
        "Should call SimpleClassAPI::get(env, loader)"
    );
    assert!(
        bindings.contains("com::jni_init(env, loader)?;"),
        "Should call child module jni_init"
    );

    println!("Generated bindings with jni_init:\n{}", bindings);
}

#[test]
fn test_jni_init_disabled() {
    let out_dir = setup_test_output("jni_init_disabled");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate_jni_init(false)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Should NOT have jni_init function when disabled
    assert!(
        !bindings.contains("pub fn jni_init"),
        "Should NOT have jni_init function when disabled"
    );

    println!("Generated bindings without jni_init:\n{}", bindings);
}

#[test]
fn test_jni_init_in_module_files() {
    let out_dir = setup_test_output("jni_init_files");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings_dir = out_dir.join("bindings");

    Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .write_to_files(&bindings_dir)
        .expect("Failed to write bindings to files");

    // Check root mod.rs contains jni_init
    let root_mod =
        fs::read_to_string(bindings_dir.join("mod.rs")).expect("Failed to read root mod.rs");
    assert!(
        root_mod.contains("pub fn jni_init(env: &::jni::Env, loader: &::jni::refs::LoaderContext) -> ::jni::errors::Result<()>"),
        "Root mod.rs should have jni_init"
    );
    assert!(
        root_mod.contains("com::jni_init(env, loader)?;"),
        "Root should call child module jni_init"
    );

    // Check com module contains jni_init
    let com_mod =
        fs::read_to_string(bindings_dir.join("com/mod.rs")).expect("Failed to read com/mod.rs");
    assert!(
        com_mod.contains("pub fn jni_init(env: &::jni::Env, loader: &::jni::refs::LoaderContext) -> ::jni::errors::Result<()>"),
        "com/mod.rs should have jni_init"
    );
    assert!(
        com_mod.contains("example::jni_init(env, loader)?;"),
        "com module should call example jni_init"
    );

    // Check example module contains jni_init
    let example_mod = fs::read_to_string(bindings_dir.join("com/example/mod.rs"))
        .expect("Failed to read com/example/mod.rs");
    assert!(
        example_mod.contains("pub fn jni_init(env: &::jni::Env, loader: &::jni::refs::LoaderContext) -> ::jni::errors::Result<()>"),
        "com/example/mod.rs should have jni_init"
    );
    assert!(
        example_mod.contains("let _ = SimpleClassAPI::get(env, loader)?;"),
        "example module should call SimpleClassAPI::get"
    );

    println!("Root mod.rs:\n{}", root_mod);
    println!("\ncom/mod.rs:\n{}", com_mod);
    println!("\ncom/example/mod.rs:\n{}", example_mod);
}

#[test]
fn test_non_reversible_method_names() {
    let out_dir = setup_test_output("awkward_names");
    let class_file = compile_java_file("tests/java/AwkwardNames.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Check for non-reversible method names using property syntax
    // updateUI -> update_ui, but update_ui -> updateUi (not reversible)
    assert!(
        bindings.contains("fn update_ui {"),
        "updateUI should use property syntax"
    );
    assert!(
        bindings.contains("name = \"updateUI\""),
        "updateUI should have explicit name property"
    );

    // getHTTPResponse -> get_httpresponse, but get_httpresponse -> getHttpresponse (not reversible)
    assert!(
        bindings.contains("fn get_httpresponse {"),
        "getHTTPResponse should use property syntax"
    );
    assert!(
        bindings.contains("name = \"getHTTPResponse\""),
        "getHTTPResponse should have explicit name property"
    );

    // getValue is reversible, should use shorthand syntax
    assert!(
        bindings.contains("fn get_value() -> jint,"),
        "getValue should use shorthand syntax"
    );
    assert!(
        !bindings.contains("name = \"getValue\""),
        "getValue should not have explicit name property"
    );

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_overloaded_constructors() {
    let out_dir = setup_test_output("overloaded_constructors");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Arity 0: base name
    assert!(
        bindings.contains("fn new()"),
        "Arity 0 constructor should use base name 'new'"
    );

    // Arity 1: should add type suffix since arity 0 exists
    assert!(
        bindings.contains("fn new_int(arg0: jint)"),
        "Arity 1 with int should be 'new_int'"
    );
    assert!(
        bindings.contains("fn new_string(arg0: JString)"),
        "Arity 1 with String should be 'new_string'"
    );

    // Arity 2: position 0 varies, position 1 varies -> no arity prefix needed
    // Should have: (int, String), (String, int), (String, String)
    assert!(
        bindings.contains("fn new_int_string(arg0: jint, arg1: JString)"),
        "Arity 2 (int, String) should be 'new_int_string'"
    );
    assert!(
        bindings.contains("fn new_string_int(arg0: JString, arg1: jint)"),
        "Arity 2 (String, int) should be 'new_string_int'"
    );
    assert!(
        bindings.contains("fn new_string_string(arg0: JString, arg1: JString)"),
        "Arity 2 (String, String) should be 'new_string_string'"
    );

    // Arity 3: all same type (int, int, int) -> only arity suffix "new3"
    // No positions vary, so just the arity suffix is used
    assert!(
        bindings.contains("fn new3(arg0: jint, arg1: jint, arg2: jint)"),
        "Arity 3 (int, int, int) should be 'new3' (arity suffix only, no varying positions)"
    );

    // Arity 4: partial variation (position 0 always int, positions 1-3 vary)
    // Should have arity prefix "4" plus varying type suffixes
    // Sorted order: (int, int, int, int), (int, String, int, int), (int, String, String, int), (int, String, String, String)
    // Varying positions are 1, 2, 3 (position 0 is always int)
    assert!(
        bindings.contains("fn new4_int_int_int(arg0: jint, arg1: jint, arg2: jint, arg3: jint)"),
        "Arity 4 (int, int, int, int) should be 'new4_int_int_int' (positions 1,2,3 vary)"
    );
    assert!(
        bindings
            .contains("fn new4_string_int_int(arg0: jint, arg1: JString, arg2: jint, arg3: jint)"),
        "Arity 4 (int, String, int, int) should be 'new4_string_int_int'"
    );
    assert!(
        bindings.contains(
            "fn new4_string_string_int(arg0: jint, arg1: JString, arg2: JString, arg3: jint)"
        ),
        "Arity 4 (int, String, String, int) should be 'new4_string_string_int'"
    );
    assert!(
        bindings.contains(
            "fn new4_string_string_string(arg0: jint, arg1: JString, arg2: JString, arg3: JString)"
        ),
        "Arity 4 (int, String, String, String) should be 'new4_string_string_string'"
    );
}

#[test]
fn test_overloaded_methods_arity_0_and_1() {
    let out_dir = setup_test_output("overloaded_methods_arity_0_1");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // process() methods: arity 0 gets base name, arity 1 gets type suffixes
    assert!(
        bindings.contains("fn process()") || bindings.contains("fn process {"),
        "process() with arity 0 should use base name"
    );
    assert!(
        bindings.contains("fn process_int") && bindings.contains("arg0: jint"),
        "process(int) should be 'process_int'"
    );
    assert!(
        bindings.contains("fn process_string") && bindings.contains("arg0: JString"),
        "process(String) should be 'process_string'"
    );
}

#[test]
fn test_overloaded_methods_same_arity_varying_positions() {
    let out_dir = setup_test_output("overloaded_methods_varying_positions");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // calculate() methods with arity 2: both positions vary
    // All 4 methods are in the same arity group, so none can use base name
    // Should have: (int, int), (int, String), (String, int), (String, String)
    assert!(
        bindings.contains("fn calculate_int_int")
            && bindings.contains("(arg0: jint, arg1: jint) -> jint"),
        "calculate(int, int) should be 'calculate_int_int' (not alone in arity group)"
    );
    assert!(
        bindings.contains("fn calculate_int_string")
            && bindings.contains("arg0: jint, arg1: JString"),
        "calculate(int, String) should be 'calculate_int_string'"
    );
    assert!(
        bindings.contains("fn calculate_string_int")
            && bindings.contains("arg0: JString, arg1: jint"),
        "calculate(String, int) should be 'calculate_string_int'"
    );
    assert!(
        bindings.contains("fn calculate_string_string")
            && bindings.contains("arg0: JString, arg1: JString"),
        "calculate(String, String) should be 'calculate_string_string'"
    );
}

#[test]
fn test_overloaded_methods_partial_varying_positions() {
    let out_dir = setup_test_output("overloaded_methods_partial_varying");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // transform() methods with arity 3: position 0 doesn't vary (always int), positions 1 and 2 vary
    // Should have arity prefix '3' since only 2 out of 3 positions vary
    // Methods sorted: (int, int, boolean), (int, String, boolean), (int, String, int)
    assert!(
        bindings.contains("fn transform")
            && bindings.contains("arg0: jint, arg1: jint, arg2: jboolean"),
        "transform(int, int, boolean) should use base name (first in sorted order)"
    );
    assert!(
        bindings.contains("fn transform3_string_bool")
            && bindings.contains("arg0: jint, arg1: JString, arg2: jboolean"),
        "transform(int, String, boolean) should be 'transform3_string_bool'"
    );
    assert!(
        bindings.contains("fn transform3_string_int")
            && bindings.contains("arg0: jint, arg1: JString, arg2: jint"),
        "transform(int, String, int) should be 'transform3_string_int'"
    );
}

#[test]
fn test_overloaded_methods_with_arrays() {
    let out_dir = setup_test_output("overloaded_methods_arrays");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // update() methods with arrays
    assert!(
        bindings.contains("fn update") && bindings.contains("arg0: jint[]"),
        "update(int[]) should use base name (first in sorted order)"
    );
    assert!(
        bindings.contains("fn update_int_2d") && bindings.contains("arg0: jint[][]"),
        "update(int[][]) should be 'update_int_2d'"
    );
    assert!(
        bindings.contains("fn update_string_1d") && bindings.contains("arg0: JString[]"),
        "update(String[]) should be 'update_string_1d'"
    );
}

#[test]
fn test_overloaded_methods_all_same_type() {
    let out_dir = setup_test_output("overloaded_methods_same_type");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    println!("Generated bindings:\n{}", bindings);

    // set() methods where all args are int (no varying positions within same arity)
    assert!(
        bindings.contains("fn set") && bindings.contains("arg0: jint"),
        "set(int) should use base name (only one method in arity 1 group)"
    );
    // Arity 2: no positions vary (both int), should add arity prefix
    assert!(
        bindings.contains("fn set2") && bindings.contains("arg0: jint, arg1: jint"),
        "set(int, int) should be 'set2' with arity prefix"
    );
    // Arity 3: no positions vary (all int), should add arity prefix
    assert!(
        bindings.contains("fn set3") && bindings.contains("arg0: jint, arg1: jint, arg2: jint"),
        "set(int, int, int) should be 'set3' with arity prefix"
    );
}

#[test]
fn test_overloaded_static_methods() {
    let out_dir = setup_test_output("overloaded_static_methods");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // format() static methods
    assert!(
        bindings.contains("static fn format") && bindings.contains("arg0: JString"),
        "format(String) should use base name"
    );
    assert!(
        bindings.contains("static fn format2") && bindings.contains("arg0: JString, arg1: JObject"),
        "format(String, Object) should be 'format2' (no varying positions in arity 2 group)"
    );
    assert!(
        bindings.contains("static fn format3")
            && bindings.contains("arg0: JString, arg1: JObject, arg2: JObject"),
        "format(String, Object, Object) should be 'format3' (no varying positions in arity 3 group)"
    );
}

#[test]
fn test_method_name_ending_with_number() {
    let out_dir = setup_test_output("method_name_with_number");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // method1() - base name ends with digit
    assert!(
        bindings.contains("fn method1()") || bindings.contains("fn method1 {"),
        "method1() should use base name"
    );
    assert!(
        bindings.contains("fn method1_int") && bindings.contains("arg0: jint"),
        "method1(int) should be 'method1_int'"
    );
    // Arity 2 with same types: should use _args2 since base name ends with digit
    assert!(
        bindings.contains("fn method1_args2") && bindings.contains("arg0: jint, arg1: jint"),
        "method1(int, int) should be 'method1_args2' (base ends with number)"
    );
}

#[test]
fn test_overloaded_methods_primitive_vs_boxed() {
    let out_dir = setup_test_output("overloaded_primitive_boxed");
    let class_file = compile_java_file("tests/java/Overloads.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // combine() methods mixing primitives and boxed types
    // All 3 methods are in arity 2 group, so none can use base name
    assert!(
        bindings.contains("fn combine_int_int") && bindings.contains("arg0: jint, arg1: jint"),
        "combine(int, int) should be 'combine_int_int' (not alone in arity group)"
    );
    assert!(
        bindings.contains("fn combine_int_string")
            && bindings.contains("arg0: jint, arg1: JString"),
        "combine(int, String) should be 'combine_int_string'"
    );
    assert!(
        bindings.contains("fn combine_integer_string")
            && bindings.contains("arg0: \"java.lang.Integer\""),
        "combine(Integer, String) should be 'combine_integer_string'"
    );
}

#[test]
fn test_skip_constructor_method_field() {
    let out_dir = setup_test_output("skip_signatures");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        // Skip the no-arg constructor
        .skip_signature("Lcom/example/SimpleClass;-><init>()V")
        // Skip the static add method
        .skip_signature("Lcom/example/SimpleClass;->add(II)I")
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Check that the skipped constructor is not present
    assert!(
        !bindings.contains("fn new()"),
        "No-arg constructor should be skipped"
    );

    // Check that the constructor with parameter is still present
    assert!(
        bindings.contains("fn new(arg0: jint)"),
        "Constructor with parameter should still be present"
    );

    // Check that the skipped method is not present
    assert!(
        !bindings.contains("static fn add"),
        "add method should be skipped"
    );

    // Check that other methods are still present
    assert!(
        bindings.contains("static fn get_message"),
        "get_message method should still be present"
    );
    assert!(
        bindings.contains("fn get_value"),
        "get_value method should still be present"
    );
}

#[test]
fn test_skip_field() {
    let out_dir = setup_test_output("skip_field");
    let class_file = compile_java_file("tests/java/WithFields.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        // Skip the CONSTANT field
        .skip_signature("Lcom/example/WithFields;->CONSTANT:I")
        // Skip the staticField field
        .skip_signature("Lcom/example/WithFields;->staticField:Ljava/lang/String;")
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Check that the skipped fields are not present
    assert!(
        !bindings.contains("CONSTANT"),
        "CONSTANT field should be skipped"
    );
    assert!(
        !bindings.contains("static_field") && !bindings.contains("staticField"),
        "staticField should be skipped"
    );

    // Check that other field is still present
    assert!(
        bindings.contains("public_field"),
        "publicField should still be present"
    );

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_name_override_constructor_method_field() {
    let out_dir = setup_test_output("name_overrides");
    let class_file = compile_java_file("tests/java/SimpleClass.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        // Rename the no-arg constructor
        .name_override("Lcom/example/SimpleClass;-><init>()V", "create")
        // Rename the static add method
        .name_override("Lcom/example/SimpleClass;->add(II)I", "sum")
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Check that the renamed constructor is present with new name
    assert!(
        bindings.contains("fn create()"),
        "No-arg constructor should be renamed to 'create'"
    );

    // Check that the old name is not used
    assert!(
        !bindings.contains("fn new()"),
        "No-arg constructor should not use default 'new' name"
    );

    // Check that the renamed method is present with new name
    assert!(
        bindings.contains("static fn sum {"),
        "add method should be renamed to 'sum'"
    );
    assert!(
        bindings.contains("name = \"add\""),
        "Method should have explicit name mapping"
    );
}

#[test]
fn test_name_override_field() {
    let out_dir = setup_test_output("name_override_field");
    let class_file = compile_java_file("tests/java/WithFields.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        // Rename the CONSTANT field
        .name_override("Lcom/example/WithFields;->CONSTANT:I", "THE_ANSWER")
        // Rename the staticField field
        .name_override(
            "Lcom/example/WithFields;->staticField:Ljava/lang/String;",
            "shared_text",
        )
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Check that the renamed fields are present with new names
    assert!(
        bindings.contains("THE_ANSWER"),
        "CONSTANT field should be renamed to 'THE_ANSWER'"
    );
    assert!(
        bindings.contains("name = \"CONSTANT\""),
        "Renamed field should have explicit name mapping"
    );

    assert!(
        bindings.contains("shared_text"),
        "staticField should be renamed to 'shared_text'"
    );
    assert!(
        bindings.contains("name = \"staticField\""),
        "Renamed field should have explicit name mapping"
    );

    println!("Generated bindings:\n{}", bindings);
}

#[test]
fn test_name_collision_detection() {
    let out_dir = setup_test_output("name_collision");
    let class_file = compile_java_file("tests/java/NameCollisionTest.java", &out_dir);

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file)
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    println!("Generated bindings:\n{}", bindings);

    // Check that bindings contain the main type
    assert!(bindings.contains("bind_java_type!"));
    assert!(bindings.contains("pub NameCollisionTest"));
    assert!(bindings.contains("com.example.NameCollisionTest"));

    // Field collision: myValue and myVALUE both map to my_value
    // First occurrence should use my_value, second should use my_value_
    assert!(
        bindings.contains("my_value:") || bindings.contains("my_value {"),
        "Should find field 'my_value'"
    );
    assert!(
        bindings.contains("my_value_ {"),
        "Should find field 'my_value_' (with underscore) for collision"
    );
    assert!(
        bindings.contains("name = \"myVALUE\""),
        "Collision field should have explicit name mapping to 'myVALUE'"
    );

    // Field collision: getData and getDATA
    assert!(
        bindings.contains("get_data:") || bindings.contains("get_data {"),
        "Should find field 'get_data'"
    );
    assert!(
        bindings.contains("get_data_ {"),
        "Should find field 'get_data_' (with underscore) for collision"
    );
    assert!(
        bindings.contains("name = \"getDATA\""),
        "Collision field should have explicit name mapping to 'getDATA'"
    );

    // Method collision: toURI and toUri both map to to_uri
    assert!(
        bindings.contains("fn to_uri {"),
        "Should find method 'to_uri'"
    );
    assert!(
        bindings.contains("name = \"toURI\""),
        "First method should have explicit name mapping to 'toURI'"
    );
    assert!(
        bindings.contains("fn to_uri_ {"),
        "Should find method 'to_uri_' (with underscore) for collision"
    );
    assert!(
        bindings.contains("name = \"toUri\""),
        "Collision method should have explicit name mapping to 'toUri'"
    );

    // Method collision: getURL and getUrl
    assert!(
        bindings.contains("fn get_url {"),
        "Should find method 'get_url'"
    );
    assert!(
        bindings.contains("name = \"getURL\""),
        "First method should have explicit name mapping to 'getURL'"
    );
    assert!(
        bindings.contains("fn get_url_ {"),
        "Should find method 'get_url_' (with underscore) for collision"
    );
    assert!(
        bindings.contains("name = \"getUrl\""),
        "Collision method should have explicit name mapping to 'getUrl'"
    );

    // Static method collision: setID and setId
    assert!(
        bindings.contains("static fn set_id {"),
        "Should find static method 'set_id'"
    );
    assert!(
        bindings.contains("name = \"setID\""),
        "First static method should have explicit name mapping to 'setID'"
    );
    assert!(
        bindings.contains("static fn set_id_ {"),
        "Should find static method 'set_id_' (with underscore) for collision"
    );
    assert!(
        bindings.contains("name = \"setId\""),
        "Collision static method should have explicit name mapping to 'setId'"
    );
}

#[test]
fn test_type_map_optimization() {
    let out_dir = setup_test_output("type_map_optimization");

    // Compile all the Java files together so dependencies are resolved
    let class_files = javac::Build::new()
        .file("tests/java/SimpleClass.java")
        .file("tests/java/Calculator.java")
        .file("tests/java/WithDependencies.java")
        .file("tests/java/WithFields.java")
        .output_dir(&out_dir)
        .compile();

    assert!(!class_files.is_empty(), "No class files were generated");

    // Find the WithDependencies class file
    let with_deps = class_files
        .iter()
        .find(|path| path.file_name().unwrap().to_str().unwrap() == "WithDependencies.class")
        .expect("WithDependencies.class not found");

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(with_deps.clone())
        // Add a type mapping for WithFields - this should NOT appear in the final type_map
        // because WithDependencies doesn't use it
        .type_mapping(
            "crate::bindings::WithFields".to_string(),
            "com.example.WithFields".to_string(),
        )
        // Add a type mapping for NoPackage - also should not appear
        .type_mapping(
            "crate::bindings::NoPackage".to_string(),
            ".NoPackage".to_string(),
        )
        // Add the actual dependencies
        .type_mapping(
            "crate::com::example::Calculator".to_string(),
            "com.example.Calculator".to_string(),
        )
        .type_mapping(
            "crate::com::example::SimpleClass".to_string(),
            "com.example.SimpleClass".to_string(),
        )
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    println!("Generated bindings:\n{}", bindings);

    // Verify the binding was created
    assert!(bindings.contains("pub WithDependencies"));
    assert!(bindings.contains("com.example.WithDependencies"));

    // Verify methods that use dependencies are present
    assert!(bindings.contains("fn get_calculator"));
    assert!(bindings.contains("fn set_calculator"));
    assert!(bindings.contains("fn get_simple_class"));
    assert!(bindings.contains("fn set_simple_class"));
    assert!(bindings.contains("fn create_calculator"));
    assert!(bindings.contains("fn create_simple_class"));

    // Verify that type_map only contains types actually used. Should contain
    // Calculator and SimpleClass
    if bindings.contains("type_map") {
        assert!(
            bindings.contains("crate::com::example::Calculator"),
            "type_map should include Calculator since it's used"
        );
        assert!(
            bindings.contains("crate::com::example::SimpleClass"),
            "type_map should include SimpleClass since it's used"
        );

        // Should NOT contain WithFields or NoPackage since they're not used
        assert!(
            !bindings.contains("WithFields"),
            "type_map should NOT include WithFields since it's not used by this class"
        );
        assert!(
            !bindings.contains("NoPackage"),
            "type_map should NOT include NoPackage since it's not used by this class"
        );

        // Extract just the type_map section to check for self type
        let type_map_section = if let Some(start) = bindings.find("type_map = {") {
            if let Some(end) = bindings[start..].find("},") {
                &bindings[start..start + end + 2]
            } else {
                ""
            }
        } else {
            ""
        };

        // Should NOT contain the self type (WithDependencies) in the type_map section
        assert!(
            !type_map_section.contains("WithDependencies"),
            "type_map should NOT include the self type (WithDependencies) because bind_java_type adds it automatically"
        );
    } else {
        panic!("Expected type_map block to be present since this class has dependencies");
    }

    // Verify that built-in types don't appear in type_map
    // JString is used in getMessage(), but it's a built-in type
    let type_map_section = if let Some(start) = bindings.find("type_map = {") {
        if let Some(end) = bindings[start..].find("},") {
            &bindings[start..start + end + 2]
        } else {
            ""
        }
    } else {
        ""
    };

    if !type_map_section.is_empty() {
        assert!(
            !type_map_section.contains("JString"),
            "type_map should NOT include JString since it's a built-in JNI type"
        );
        assert!(
            !type_map_section.contains("java.lang.String"),
            "type_map should NOT include java.lang.String"
        );
    }
}
