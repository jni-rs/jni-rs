//! Basic example of using jbindgen to generate bindings from a Java class file.
//!
//! This example shows how to:
//! 1. Compile a Java class file
//! 2. Generate Rust bindings using jbindgen
//! 3. Print the generated bindings
//!
//! Run with: cargo run --example basic

use jbindgen::Builder;
use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("jbindgen Basic Example");
    println!("======================\n");

    // Set up output directory for compiled class
    let out_dir = PathBuf::from(env::var("CARGO_TARGET_TMPDIR").unwrap_or_else(|_| {
        env::temp_dir()
            .join("jbindgen_examples")
            .display()
            .to_string()
    }))
    .join("basic_example");

    // Clean and create output directory
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).expect("Failed to create output directory");

    // Compile the example Java class
    println!("Compiling examples/java/HelloWorld.java...");
    let class_files = javac::Build::new()
        .file("examples/java/HelloWorld.java")
        .output_dir(&out_dir)
        .compile();

    if class_files.is_empty() {
        eprintln!("Error: No class files were generated");
        std::process::exit(1);
    }

    let class_file = &class_files[0];
    println!("Compiled to: {}\n", class_file.display());

    // Example 1: Generate bindings with default options
    println!("=== Example 1: Default Options ===\n");
    match Builder::new().input_class(class_file.clone()).generate() {
        Ok(bindings) => println!("{}\n", bindings.to_string()),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 2: Custom Rust type name
    println!("=== Example 2: Custom Rust Type Name ===\n");
    match Builder::new()
        .input_class(class_file.clone())
        .rust_type_name("MyHelloWorld".to_string())
        .generate()
    {
        Ok(bindings) => println!("{}\n", bindings.to_string()),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    // Example 3: Private type
    println!("=== Example 3: Private Type ===\n");
    match Builder::new()
        .input_class(class_file.clone())
        .public_type(false)
        .generate()
    {
        Ok(bindings) => println!("{}\n", bindings.to_string()),
        Err(e) => eprintln!("Error: {}\n", e),
    }

    println!("\nYou can now use these bindings in your Rust code with the jni crate!");
}
