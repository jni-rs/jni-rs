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

#[test]
fn test_jar_file_bindings() {
    let out_dir = setup_test_output("jar_test");

    // Compile multiple Java files
    let class_files = javac::Build::new()
        .files(["tests/java/SimpleClass.java", "tests/java/Calculator.java"])
        .output_dir(&out_dir)
        .compile();

    assert!(!class_files.is_empty(), "No class files were generated");

    // Create a JAR file
    let jar_path = out_dir.join("test.jar");
    let jar_file = fs::File::create(&jar_path).expect("Failed to create JAR file");
    let mut zip_writer = zip::ZipWriter::new(jar_file);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add each class file to the JAR
    for class_file in &class_files {
        let rel_path = class_file
            .strip_prefix(&out_dir)
            .expect("Failed to get relative path");
        let class_bytes = fs::read(class_file).expect("Failed to read class file");

        zip_writer
            .start_file(rel_path.to_string_lossy().to_string(), options)
            .expect("Failed to start file in JAR");
        std::io::Write::write_all(&mut zip_writer, &class_bytes).expect("Failed to write to JAR");
    }

    zip_writer.finish().expect("Failed to finish JAR");

    // Test reading from JAR
    let bindings = Builder::new()
        .input_jar(jar_path)
        .generate()
        .expect("Failed to generate bindings from JAR");

    assert!(!bindings.is_empty(), "No bindings generated from JAR");
    assert_eq!(bindings.len(), 2, "Expected 2 classes in JAR");

    // Verify bindings contain expected content
    let bindings_str = bindings.to_string();
    println!("Generated bindings:\n{}", bindings_str);

    assert!(bindings_str.contains("bind_java_type!"));
    assert!(bindings_str.contains("use jni::bind_java_type;"));
    assert!(bindings_str.contains("SimpleClass"));
    assert!(bindings_str.contains("com.example.SimpleClass"));
    assert!(bindings_str.contains("Calculator"));
    assert!(bindings_str.contains("com.example.Calculator"));
}

#[test]
fn test_jar_empty() {
    let out_dir = setup_test_output("jar_empty");

    // Create an empty JAR
    let jar_path = out_dir.join("empty.jar");
    let jar_file = fs::File::create(&jar_path).expect("Failed to create JAR file");
    let zip_writer = zip::ZipWriter::new(jar_file);
    zip_writer.finish().expect("Failed to finish JAR");

    // Test reading empty JAR
    let bindings = Builder::new()
        .input_jar(jar_path)
        .generate()
        .expect("Failed to read empty JAR");

    assert!(bindings.is_empty(), "Expected no bindings from empty JAR");
}

#[test]
fn test_write_to_files() {
    let out_dir = setup_test_output("write_to_files");

    // Compile multiple Java files
    let class_files = javac::Build::new()
        .files(["tests/java/SimpleClass.java", "tests/java/Calculator.java"])
        .output_dir(&out_dir)
        .compile();

    assert!(!class_files.is_empty(), "No class files were generated");

    // Create a JAR file
    let jar_path = out_dir.join("test.jar");
    let jar_file = fs::File::create(&jar_path).expect("Failed to create JAR file");
    let mut zip_writer = zip::ZipWriter::new(jar_file);

    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Add each class file to the JAR
    for class_file in &class_files {
        let rel_path = class_file
            .strip_prefix(&out_dir)
            .expect("Failed to get relative path");
        let class_bytes = fs::read(class_file).expect("Failed to read class file");

        zip_writer
            .start_file(rel_path.to_string_lossy().to_string(), options)
            .expect("Failed to start file in JAR");
        std::io::Write::write_all(&mut zip_writer, &class_bytes).expect("Failed to write to JAR");
    }

    zip_writer.finish().expect("Failed to finish JAR");

    // Generate bindings
    let bindings = Builder::new()
        .input_jar(jar_path)
        .generate()
        .expect("Failed to generate bindings from JAR");

    assert!(!bindings.is_empty(), "No bindings generated from JAR");

    // Write to files
    let modules_dir = out_dir.join("modules");
    bindings
        .write_to_files(&modules_dir)
        .expect("Failed to write bindings to files");

    // Verify directory structure was created
    assert!(modules_dir.exists(), "Modules directory should exist");
    assert!(
        modules_dir.join("mod.rs").exists(),
        "Root mod.rs should exist"
    );
    assert!(
        modules_dir.join("com").exists(),
        "com directory should exist"
    );
    assert!(
        modules_dir.join("com/mod.rs").exists(),
        "com/mod.rs should exist"
    );
    assert!(
        modules_dir.join("com/example").exists(),
        "com/example directory should exist"
    );
    assert!(
        modules_dir.join("com/example/mod.rs").exists(),
        "com/example/mod.rs should exist"
    );

    // Verify root mod.rs declares the com module
    let root_mod =
        fs::read_to_string(modules_dir.join("mod.rs")).expect("Failed to read root mod.rs");
    assert!(
        root_mod.contains("pub mod com;"),
        "Root mod.rs should declare com module"
    );

    // Verify com/mod.rs declares the example module
    let com_mod =
        fs::read_to_string(modules_dir.join("com/mod.rs")).expect("Failed to read com/mod.rs");
    assert!(
        com_mod.contains("pub mod example;"),
        "com/mod.rs should declare example module"
    );

    // Verify com/example/mod.rs contains the bindings
    let example_mod = fs::read_to_string(modules_dir.join("com/example/mod.rs"))
        .expect("Failed to read com/example/mod.rs");
    assert!(
        example_mod.contains("bind_java_type!"),
        "example/mod.rs should contain bindings"
    );
    assert!(
        example_mod.contains("SimpleClass"),
        "example/mod.rs should contain SimpleClass"
    );
    assert!(
        example_mod.contains("Calculator"),
        "example/mod.rs should contain Calculator"
    );

    println!("Root mod.rs:\n{}\n", root_mod);
    println!("com/mod.rs:\n{}\n", com_mod);
    println!(
        "com/example/mod.rs (first 500 chars):\n{}...\n",
        &example_mod[..example_mod.len().min(500)]
    );
}
