use jbindgen::Builder;
use std::fs;
use std::path::PathBuf;

#[test]
fn test_bindings_type_map() {
    // Build bindings from Java source with some extra type mappings
    let bindings = Builder::new()
        .root_path("my_crate::bindings")
        .input_sources(
            vec![
                PathBuf::from("tests/java/SimpleClass.java"),
                PathBuf::from("tests/java/Calculator.java"),
                PathBuf::from("tests/java/WithDependencies.java"),
            ],
            vec![], // no classpath
            vec!["**".to_string()],
        )
        .type_mapping("external::crate::SomeType".to_string(), "com.external.SomeType".to_string())
        .generate()
        .unwrap();

    // Get the type map
    let type_map = bindings.type_map(None);
    
    // Debug: print the type map
    println!("Type map entries ({}):", type_map.len());
    for (java_type, rust_type) in type_map.iter() {
        println!("  {} => {}", java_type, rust_type);
    }

    // Verify it includes generated bindings
    assert!(type_map.get_rust_type("com.example.SimpleClass").is_some());
    assert!(type_map.get_rust_type("com.example.Calculator").is_some());
    assert!(type_map.get_rust_type("com.example.WithDependencies").is_some());

    // Verify it includes input mappings
    assert_eq!(
        type_map.get_rust_type("com.external.SomeType"),
        Some("external::crate::SomeType")
    );

    // Test with public root path
    let pub_type_map = bindings.type_map(Some("my_crate"));
    
    // Check a generated mapping uses the public root
    let simple_class_type = pub_type_map.get_rust_type("com.example.SimpleClass").unwrap();
    assert!(simple_class_type.starts_with("my_crate::"), 
        "Expected public root 'my_crate::' but got: {}", simple_class_type);

    // Write type map to file
    let out_dir = std::env::temp_dir().join("jbindgen_type_map_test");
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir).unwrap();
    
    let type_map_file = out_dir.join("type_map.txt");
    bindings.write_type_map(&type_map_file).unwrap();
    
    // Verify file was created and contains expected content
    let content = fs::read_to_string(&type_map_file).unwrap();
    assert!(content.contains("SimpleClass"));
    assert!(content.contains("Calculator"));
    assert!(content.contains("WithDependencies"));
    assert!(content.contains("com.example"));
    
    // Write public type map
    let pub_type_map_file = out_dir.join("pub_type_map.txt");
    bindings.write_pub_type_map(&pub_type_map_file, "my_crate").unwrap();
    
    let pub_content = fs::read_to_string(&pub_type_map_file).unwrap();
    assert!(pub_content.contains("my_crate::"));
    assert!(!pub_content.contains("my_crate::bindings::"));

    println!("Type map content:\n{}", content);
    println!("\nPublic type map content:\n{}", pub_content);
}
