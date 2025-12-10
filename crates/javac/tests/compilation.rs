use std::fs;
use std::path::{Path, PathBuf};

/// Helper function to validate a .class file using cafebabe
fn validate_class_file(class_path: &Path) -> Result<(), String> {
    let bytes = fs::read(class_path).map_err(|e| format!("Failed to read class file: {}", e))?;

    let class =
        cafebabe::parse_class(&bytes).map_err(|e| format!("Failed to parse class file: {}", e))?;

    // Each class should have at least one method (even if just the implicit constructor)
    if class.methods.is_empty() {
        return Err(format!("Class {} has no methods", class.this_class));
    }

    Ok(())
}

/// Helper function to validate class files and check they have methods
fn validate_class_files(class_files: &[PathBuf]) {
    assert!(!class_files.is_empty(), "No class files generated");

    for class_path in class_files {
        validate_class_file(class_path)
            .unwrap_or_else(|_| panic!("Invalid class file: {:?}", class_path));
    }
}

#[test]
fn test_compile_single_file() {
    let out_dir = setup_test_output("single_file");

    let class_files = javac::Build::new()
        .file("tests/java/com/example/SimpleClass.java")
        .output_dir(&out_dir)
        .compile();

    assert!(!class_files.is_empty(), "No class files were generated");
    assert!(
        class_files
            .iter()
            .any(|p| p.file_name().unwrap() == "SimpleClass.class"),
        "SimpleClass.class not found in output"
    );

    // Verify the file exists
    let simple_class = out_dir.join("com/example/SimpleClass.class");
    assert!(
        simple_class.exists(),
        "SimpleClass.class does not exist at expected location"
    );

    // Validate the compiled class files
    validate_class_files(&class_files);
}

#[test]
fn test_compile_multiple_files() {
    let out_dir = setup_test_output("multiple_files");

    let class_files = javac::Build::new()
        .file("tests/java/com/example/Foo.java")
        .file("tests/java/com/example/Bar.java")
        .output_dir(&out_dir)
        .compile();

    assert!(
        class_files.len() >= 2,
        "Expected at least 2 class files, got {}",
        class_files.len()
    );

    // Check that both Foo and Bar were compiled
    assert!(
        class_files
            .iter()
            .any(|p| p.file_name().unwrap() == "Foo.class"),
        "Foo.class not found in output"
    );
    assert!(
        class_files
            .iter()
            .any(|p| p.file_name().unwrap() == "Bar.class"),
        "Bar.class not found in output"
    );

    // Validate the compiled class files
    validate_class_files(&class_files);
}

#[test]
fn test_compile_with_files_method() {
    let out_dir = setup_test_output("files_method");

    let files = vec![
        "tests/java/com/example/Foo.java",
        "tests/java/com/example/Bar.java",
        "tests/java/com/example/SimpleClass.java",
    ];

    let class_files = javac::Build::new()
        .files(files)
        .output_dir(&out_dir)
        .compile();

    assert!(class_files.len() >= 3, "Expected at least 3 class files");

    // Validate the compiled class files
    validate_class_files(&class_files);
}

#[test]
fn test_compile_source_dir() {
    let out_dir = setup_test_output("source_dir");

    let class_files = javac::Build::new()
        .source_dir("tests/java")
        .output_dir(&out_dir)
        .compile();

    // Should find and compile all .java files in the directory
    assert!(
        class_files.len() >= 3,
        "Expected at least 3 class files from source_dir"
    );

    assert!(
        class_files
            .iter()
            .any(|p| p.file_name().unwrap() == "Foo.class"),
        "Foo.class not found"
    );
    assert!(
        class_files
            .iter()
            .any(|p| p.file_name().unwrap() == "Bar.class"),
        "Bar.class not found"
    );
    assert!(
        class_files
            .iter()
            .any(|p| p.file_name().unwrap() == "SimpleClass.class"),
        "SimpleClass.class not found"
    );

    // Validate the compiled class files
    validate_class_files(&class_files);
}

#[test]
fn test_compile_with_debug() {
    let out_dir = setup_test_output("with_debug");

    let class_files = javac::Build::new()
        .file("tests/java/com/example/SimpleClass.java")
        .output_dir(&out_dir)
        .debug(true)
        .compile();

    assert!(!class_files.is_empty());

    // Validate the compiled class files
    validate_class_files(&class_files);
}

#[test]
fn test_compile_with_version_flags() {
    let out_dir = setup_test_output("with_version");

    let class_files = javac::Build::new()
        .file("tests/java/com/example/SimpleClass.java")
        .output_dir(&out_dir)
        .source_version("8")
        .target_version("8")
        .compile();

    assert!(!class_files.is_empty());

    // Validate the compiled class files
    validate_class_files(&class_files);
}

#[test]
fn test_compile_with_release() {
    let out_dir = setup_test_output("with_release");

    let result = javac::Build::new()
        .file("tests/java/com/example/SimpleClass.java")
        .output_dir(&out_dir)
        .release("11")
        .try_compile();

    match result {
        Ok(class_files) => {
            assert!(!class_files.is_empty());
            // Validate the compiled class files
            validate_class_files(&class_files);
        }
        Err(javac::Error::Unsupported(msg)) => {
            // Skip test if --release is not supported (e.g., Java 8)
            println!("Skipping test: {}", msg);
        }
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_compile_fails_with_no_files() {
    let out_dir = setup_test_output("no_files");

    let result = javac::Build::new().output_dir(&out_dir).try_compile();

    assert!(
        result.is_err(),
        "Should fail when no source files are specified"
    );
    assert!(result.unwrap_err().to_string().contains("No source files"));
}

// Helper function to set up test output directory
fn setup_test_output(test_name: &str) -> PathBuf {
    let out_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("javac_tests")
        .join(test_name);

    // Clean up any existing output
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("Failed to create test output directory");

    out_dir
}
