//! Runtime tests that compile and execute generated bindings
//!
//! These tests verify that the generated bindings actually work by:
//! 1. Compiling Java source files
//! 2. Generating Rust bindings
//! 3. Creating a test Rust project that uses the bindings
//! 4. Compiling and running the test project
//!
//! Set _JBINDGEN_TEST_OUT_DIR environment variable to control where test
//! crates are generated. If not set, uses the system temp directory.

use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use jbindgen::Builder;

/// Get the output directory for test crates
fn get_test_crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

/// Helper function to set up test output directory
fn setup_test_output(test_name: &str) -> PathBuf {
    let base_dir = get_test_crate_dir();
    let out_dir = base_dir.join("jbindgen_runtime_tests").join(test_name);

    // Clean up any existing output
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("Failed to create test output directory");

    out_dir
}

/// Extract test code from rustdoc comments between <test:marker> markers
fn extract_test_code(source: &str, marker: &str) -> String {
    // Find the section between /// <test:marker> and /// </test:marker>
    let start_marker = format!("/// <test:{}>", marker);
    let end_marker = format!("/// </test:{}>", marker);

    let start = source
        .find(&start_marker)
        .unwrap_or_else(|| panic!("Start marker '{}' not found", start_marker));
    let end = source
        .find(&end_marker)
        .unwrap_or_else(|| panic!("End marker '{}' not found", end_marker));

    let section = &source[start + start_marker.len()..end];

    // Extract lines that start with "/// " and remove the prefix
    let mut code_lines = Vec::new();
    let mut in_code_block = false;

    for line in section.lines() {
        let trimmed = line.trim();

        if trimmed == "/// ```rust" {
            in_code_block = true;
            continue;
        }
        if trimmed == "/// ```" {
            break;
        }

        if in_code_block {
            if let Some(stripped) = line.strip_prefix("/// ") {
                code_lines.push(stripped);
            } else if trimmed == "///" {
                code_lines.push("");
            }
        }
    }

    code_lines.join("\n")
}

/// Helper to create a test Rust project that uses the generated bindings
fn create_test_project(
    test_name: &str,
    bindings: &str,
    test_code: &str,
    class_dir: &Path,
) -> PathBuf {
    let project_dir = setup_test_output(test_name);

    // Get the absolute path to the jni crate
    let jni_crate_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent dir")
        .join("jni");

    // Create Cargo.toml
    let cargo_toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[workspace]

[dependencies]
jni = {{ path = "{}", features = ["invocation"] }}

[lib]
name = "test_bindings"
path = "src/lib.rs"

[[bin]]
name = "test_runner"
path = "src/main.rs"
"#,
        test_name.replace('-', "_"),
        jni_crate_path.display()
    );

    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src directory");

    fs::write(project_dir.join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    // Create lib.rs with generated bindings and test module
    let lib_rs = format!(
        r#"#![allow(unused)]
{}

pub mod test_utils {{
    use std::sync::{{Arc, Once}};
    use jni::{{Env, InitArgsBuilder, JNIVersion, JavaVM}};

    pub fn jvm() -> JavaVM {{
        static INIT: Once = Once::new();

        INIT.call_once(|| {{
            let jvm_args = InitArgsBuilder::new()
                .version(JNIVersion::V1_8)
                .option("-Djava.class.path={}")
                .option("-Xcheck:jni")
                .build()
                .expect("Failed to build JVM args");

            let _jvm = JavaVM::new(jvm_args).expect("Failed to create JVM");
        }});

        JavaVM::singleton().expect("JVM not initialized")
    }}

    pub fn with_attached_jvm<F, T>(callback: F) -> jni::errors::Result<T>
    where
        F: FnOnce(&mut Env) -> jni::errors::Result<T>,
    {{
        jvm().attach_current_thread(callback)
    }}
}}

pub mod test_code {{
    use jni::Env;
    use super::*;

{}

    pub fn run_test(env: &mut Env) -> jni::errors::Result<()> {{
        test_impl(env)
    }}
}}
"#,
        bindings,
        class_dir.display(),
        test_code
    );

    fs::write(src_dir.join("lib.rs"), lib_rs).expect("Failed to write lib.rs");

    // Create main.rs that calls the test function
    let main_rs = r#"use test_bindings::test_utils::*;
use test_bindings::test_code;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    with_attached_jvm(|env| {
        test_code::run_test(env)
    })?;
    println!("All tests passed!");
    Ok(())
}
"#;

    fs::write(src_dir.join("main.rs"), main_rs).expect("Failed to write main.rs");

    project_dir
}

/// Run the test project and verify it succeeds
fn run_test_project(project_dir: &PathBuf) {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .current_dir(project_dir)
        .output()
        .expect("Failed to run cargo");

    if !output.status.success() {
        eprintln!("=== STDOUT ===");
        eprintln!("{}", String::from_utf8_lossy(&output.stdout));
        eprintln!("=== STDERR ===");
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        panic!("Test project failed to run");
    }

    println!("Test output:\n{}", String::from_utf8_lossy(&output.stdout));
}

/// Tests the following rustdoc code, copied into a standalone crate with
/// generated test class bindings.
///
/// <test:simple_class>
/// ```rust
/// fn test_impl(env: &mut jni::Env) -> jni::errors::Result<()> {
///     use super::com::example::SimpleClass;
///     // Test default constructor
///     let obj = SimpleClass::new(env)?;
///
///     // Test getValue - should return 0 initially
///     let value = obj.get_value(env)?;
///     assert_eq!(value, 0, "Initial value should be 0");
///
///     // Test setValue
///     obj.set_value(env, 42)?;
///
///     // Test getValue again
///     let value = obj.get_value(env)?;
///     assert_eq!(value, 42, "Value should be 42 after setValue");
///
///     // Test static add method
///     let result = SimpleClass::add(env, 10, 20)?;
///     assert_eq!(result, 30, "add(10, 20) should return 30");
///
///     // Test static getMessage
///     let msg = SimpleClass::get_message(env)?;
///     let msg_str = msg.to_string();
///     assert_eq!(msg_str, "Hello from SimpleClass", "getMessage should return correct string");
///
///     Ok(())
/// }
/// ```
/// </test:simple_class>
#[test]
fn test_simple_class_runtime() {
    let out_dir = setup_test_output("simple_class_compile");

    // Compile Java class
    let class_files = javac::Build::new()
        .file("tests/java/SimpleClass.java")
        .output_dir(&out_dir)
        .compile();

    let class_file = &class_files[0];

    // Generate bindings
    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file.clone())
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Extract test code
    let source = include_str!("runtime_tests.rs");
    let test_code = extract_test_code(source, "simple_class");

    // Create and run test project
    let project_dir = create_test_project("simple_class_runtime", &bindings, &test_code, &out_dir);
    run_test_project(&project_dir);
}

/// Tests the following rustdoc code, copied into a standalone crate with
/// generated test class bindings.
///
/// <test:calculator>
/// ```rust
/// fn test_impl(env: &mut jni::Env) -> jni::errors::Result<()> {
///     use super::com::example::Calculator;
///     // Test multiply
///     let result = Calculator::multiply(env, 6, 7)?;
///     assert_eq!(result, 42, "6 * 7 should be 42");
///
///     // Test multiply_long
///     let result = Calculator::multiply_long(env, 1000000, 1000000)?;
///     assert_eq!(
///         result, 1000000000000,
///         "1000000 * 1000000 should be 1000000000000"
///     );
///
///     // Test divide
///     let result = Calculator::divide(env, 10.0, 2.0)?;
///     assert!((result - 5.0).abs() < 0.001, "10.0 / 2.0 should be 5.0");
///
///     // Test instance methods
///     let calc = Calculator::new(env)?;
///     let result = calc.square(env, 5)?;
///     assert_eq!(result, 25, "square(5) should be 25");
///
///     let result = calc.power(env, 2.0, 3.0)?;
///     assert!(
///         (result - 8.0).abs() < 0.001,
///         "power(2.0, 3.0) should be 8.0"
///     );
///
///     Ok(())
/// }
/// ```
/// </test:calculator>
/// </test:calculator>
#[test]
fn test_calculator_runtime() {
    let out_dir = setup_test_output("calculator_compile");

    // Compile Java class
    let class_files = javac::Build::new()
        .file("tests/java/Calculator.java")
        .output_dir(&out_dir)
        .compile();

    let class_file = &class_files[0];

    // Generate bindings
    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file.clone())
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Extract test code
    let source = include_str!("runtime_tests.rs");
    let test_code = extract_test_code(source, "calculator");

    let project_dir = create_test_project("calculator_runtime", &bindings, &test_code, &out_dir);
    run_test_project(&project_dir);
}

/// Tests the following rustdoc code, copied into a standalone crate with
/// generated test class bindings.
///
/// <test:constructor_with_args>
/// ```rust
/// fn test_impl(env: &mut jni::Env) -> jni::errors::Result<()> {
///     use super::com::example::SimpleClass;
///     // Test constructor with argument
///     let obj = SimpleClass::new_int(env, 99)?;
///
///     let value = obj.get_value(env)?;
///     assert_eq!(value, 99, "Constructor should set initial value to 99");
///
///     Ok(())
/// }
/// ```
/// </test:constructor_with_args>
#[test]
fn test_constructor_with_args_runtime() {
    let out_dir = setup_test_output("constructor_args_compile");

    let class_files = javac::Build::new()
        .file("tests/java/SimpleClass.java")
        .output_dir(&out_dir)
        .compile();

    let class_file = &class_files[0];

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file.clone())
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Extract test code
    let source = include_str!("runtime_tests.rs");
    let test_code = extract_test_code(source, "constructor_with_args");

    let project_dir =
        create_test_project("constructor_args_runtime", &bindings, &test_code, &out_dir);
    run_test_project(&project_dir);
}

/// Tests the following rustdoc code, copied into a standalone crate with
/// generated test class bindings.
///
/// <test:string_handling>
/// ```rust
/// fn test_impl(env: &mut jni::Env) -> jni::errors::Result<()> {
///     use jni::objects::JString;
///     use super::com::example::SimpleClass;
///
///     // Test concat static method
///     let a = JString::from_str(env, "Hello, ")?;
///     let b = JString::from_str(env, "World!")?;
///     let result = SimpleClass::concat(env, &a, &b)?;
///     let result_str = result.to_string();
///     assert_eq!(
///         result_str, "Hello, World!",
///         "concat should concatenate strings"
///     );
///
///     Ok(())
/// }
/// ```
/// </test:string_handling>
#[test]
fn test_string_handling_runtime() {
    let out_dir = setup_test_output("string_handling_compile");

    let class_files = javac::Build::new()
        .file("tests/java/SimpleClass.java")
        .output_dir(&out_dir)
        .compile();

    let class_file = &class_files[0];

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file.clone())
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Extract test code
    let source = include_str!("runtime_tests.rs");
    let test_code = extract_test_code(source, "string_handling");

    let project_dir =
        create_test_project("string_handling_runtime", &bindings, &test_code, &out_dir);
    run_test_project(&project_dir);
}

/// Tests the following rustdoc code, copied into a standalone crate with
/// generated test class bindings.
///
/// <test:no_package>
/// ```rust
/// fn test_impl(env: &mut jni::Env) -> jni::errors::Result<()> {
///     use super::NoPackage;
///     // Test class in default package
///     let obj = NoPackage::new(env)?;
///
///     let answer = NoPackage::get_answer(env)?;
///     assert_eq!(answer, 42, "getAnswer should return 42");
///
///     let name = obj.get_name(env)?;
///     let name_str = name.to_string();
///     assert_eq!(name_str, "NoPackage", "getName should return 'NoPackage'");
///
///     Ok(())
/// }
/// ```
/// </test:no_package>
#[test]
fn test_no_package_runtime() {
    let out_dir = setup_test_output("no_package_compile");

    let class_files = javac::Build::new()
        .file("tests/java/NoPackage.java")
        .output_dir(&out_dir)
        .compile();

    let class_file = &class_files[0];

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file.clone())
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Extract test code
    let source = include_str!("runtime_tests.rs");
    let test_code = extract_test_code(source, "no_package");

    let project_dir = create_test_project("no_package_runtime", &bindings, &test_code, &out_dir);
    run_test_project(&project_dir);
}

/// Tests the jni_init functionality for bulk initialization of bindings.
///
/// <test:jni_init>
/// ```rust
/// fn test_impl(env: &mut jni::Env) -> jni::errors::Result<()> {
///     use jni::objects::LoaderContext;
///     use super::com::example::SimpleClass;
///     // Use the default LoaderContext
///     let loader = LoaderContext::None;
///
///     // Initialize all bindings at once
///     super::com::jni_init(env, &loader)?;
///
///     // Now we can use the bindings - the classes and method IDs are already cached
///     let obj = SimpleClass::new(env)?;
///     let value = obj.get_value(env)?;
///     assert_eq!(value, 0, "getValue should return 0");
///
///     let answer = SimpleClass::get_message(env)?;
///     let answer_str = answer.to_string();
///     assert_eq!(answer_str, "Hello from SimpleClass", "getMessage should return correct string");
///
///     Ok(())
/// }
/// ```
/// </test:jni_init>
/// </test:jni_init>
#[test]
fn test_jni_init_runtime() {
    let out_dir = setup_test_output("jni_init_compile");

    let class_files = javac::Build::new()
        .file("tests/java/SimpleClass.java")
        .output_dir(&out_dir)
        .compile();

    let class_file = &class_files[0];

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file.clone())
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Verify that jni_init is generated
    assert!(
        bindings.contains("pub fn jni_init("),
        "Bindings should contain jni_init function"
    );
    assert!(
        bindings.contains("SimpleClassAPI::get(env, loader)?"),
        "jni_init should call SimpleClassAPI::get"
    );

    // Extract test code
    let source = include_str!("runtime_tests.rs");
    let test_code = extract_test_code(source, "jni_init");

    let project_dir = create_test_project("jni_init_runtime", &bindings, &test_code, &out_dir);
    run_test_project(&project_dir);
}

/// Tests jni_init with multiple classes in a module hierarchy.
///
/// <test:jni_init_module_hierarchy>
/// ```rust
/// fn test_impl(env: &mut jni::Env) -> jni::errors::Result<()> {
///     use jni::objects::LoaderContext;
///     use super::com::example::SimpleClass;
///     // Use the default LoaderContext
///     let loader = LoaderContext::None;
///
///     // Initialize all bindings in the com module hierarchy
///     super::com::jni_init(env, &loader)?;
///
///     // Now we can use the bindings
///     let obj = SimpleClass::new(env)?;
///     let value = obj.get_value(env)?;
///     assert_eq!(value, 0, "getValue should return 0");
///
///     Ok(())
/// }
/// ```
/// </test:jni_init_module_hierarchy>
#[test]
fn test_jni_init_module_hierarchy_runtime() {
    let out_dir = setup_test_output("jni_init_hierarchy_compile");

    let class_files = javac::Build::new()
        .file("tests/java/SimpleClass.java")
        .output_dir(&out_dir)
        .compile();

    let class_file = &class_files[0];

    let bindings = Builder::new()
        .root_path("crate")
        .input_class(class_file.clone())
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Verify that jni_init is generated at multiple levels
    assert!(
        bindings.contains("pub fn jni_init("),
        "Bindings should contain jni_init functions"
    );

    // Extract test code
    let source = include_str!("runtime_tests.rs");
    let test_code = extract_test_code(source, "jni_init_module_hierarchy");

    let project_dir = create_test_project(
        "jni_init_hierarchy_runtime",
        &bindings,
        &test_code,
        &out_dir,
    );
    run_test_project(&project_dir);
}

/// Tests cross-package type references with fully-qualified type paths.
///
/// This test verifies that when generating bindings for multiple classes across
/// different packages, the type_map contains fully-qualified Rust paths that
/// allow cross-package method parameters and return types to work correctly.
///
/// <test:cross_package>
/// ```rust
/// fn test_impl(env: &mut jni::Env) -> jni::errors::Result<()> {
///     use jni::objects::JString;
///     use super::com::example::service::PersonService;
///     use super::com::example::data::Person;
///
///     // Create a person using PersonService
///     let name = JString::from_str(env, "Alice")?;
///     let person = PersonService::create_person(env, &name, 30)?;
///
///     // Get person's name
///     let retrieved_name = PersonService::get_person_name(env, &person)?;
///     let name_str = retrieved_name.to_string();
///     assert_eq!(name_str, "Alice", "Person name should be Alice");
///
///     // Get person's age
///     let age = PersonService::get_person_age(env, &person)?;
///     assert_eq!(age, 30, "Person age should be 30");
///
///     // Update person
///     let new_name = JString::from_str(env, "Bob")?;
///     PersonService::update_person(env, &person, &new_name, 25)?;
///
///     // Verify update
///     let updated_name = PersonService::get_person_name(env, &person)?;
///     let updated_name_str = updated_name.to_string();
///     assert_eq!(updated_name_str, "Bob", "Updated person name should be Bob");
///
///     let updated_age = PersonService::get_person_age(env, &person)?;
///     assert_eq!(updated_age, 25, "Updated person age should be 25");
///
///     Ok(())
/// }
/// ```
/// </test:cross_package>
#[test]
fn test_cross_package_runtime() {
    let out_dir = setup_test_output("cross_package_compile");

    // Compile both Java classes
    let _class_files = javac::Build::new()
        .file("tests/java/com/example/data/Person.java")
        .file("tests/java/com/example/service/PersonService.java")
        .output_dir(&out_dir)
        .compile();

    // Generate bindings using JAR input to get all classes with a unified type_map
    let jar_path = out_dir.join("classes.jar");

    // Create a JAR from the compiled classes
    std::process::Command::new("jar")
        .arg("cf")
        .arg(&jar_path)
        .arg("-C")
        .arg(&out_dir)
        .arg(".")
        .status()
        .expect("Failed to create JAR");

    // Generate bindings for all classes together
    // This will create a unified type_map with fully-qualified paths
    let bindings = Builder::new()
        .root_path("crate")
        .input_jar(&jar_path)
        .patterns(vec!["com.example.*".to_string()])
        .generate()
        .expect("Failed to generate bindings")
        .to_string();

    // Verify that the type_map contains fully-qualified paths
    assert!(
        bindings.contains("crate::com::example::data::Person"),
        "Type map should contain fully-qualified path for Person: {}",
        bindings
    );

    // Verify that PersonService has methods that reference Person with fully-qualified paths
    assert!(
        bindings.contains("create_person"),
        "PersonService should have create_person method"
    );

    // Extract test code
    let source = include_str!("runtime_tests.rs");
    let test_code = extract_test_code(source, "cross_package");

    let project_dir = create_test_project("cross_package_runtime", &bindings, &test_code, &out_dir);
    run_test_project(&project_dir);
}
