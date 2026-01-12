//! Tests for generating bindings from Android SDK classes (android.jar)
//!
//! These tests require the ANDROID_HOME environment variable to be set
//! and the Android SDK to be installed with API level 35.
//!
//! Note: These are based on the .class files in android.jar, not the
//! source stubs and therefore won't include documentation comments or
//! meaningful method parameter names.

use std::env;
use std::path::PathBuf;

use jbindgen::Builder;

/// Get the path to the Android SDK JAR file
fn get_android_jar() -> Option<PathBuf> {
    let android_home = env::var("ANDROID_HOME").ok()?;
    let jar_path = PathBuf::from(android_home)
        .join("platforms")
        .join("android-35")
        .join("android.jar");

    if jar_path.exists() {
        Some(jar_path)
    } else {
        None
    }
}

/// Helper macro to skip tests if Android SDK is not available
macro_rules! require_android_sdk {
    () => {
        match get_android_jar() {
            Some(jar) => jar,
            None => {
                eprintln!("Skipping test: ANDROID_HOME not set or android-35 SDK not found");
                return;
            }
        }
    };
}

#[test]
fn test_bind_os_build_from_android_jar() {
    let jar_path = require_android_sdk!();

    let bindings_obj = Builder::new()
        .input_jar(jar_path)
        .patterns(vec!["android.os.Build"])
        .generate()
        .expect("Failed to generate bindings from Android SDK");

    assert!(
        !bindings_obj.is_empty(),
        "Should generate some bindings from Android SDK"
    );

    let content = bindings_obj.to_string();

    // Should have the class binding
    assert!(
        content.contains("pub Build =>"),
        "Should define Build binding"
    );

    // Should have bind_java_type macro
    assert!(
        content.contains("bind_java_type!"),
        "Should use bind_java_type macro"
    );

    // Should have the correct class path
    assert!(
        content.contains("android.os.Build") || content.contains("android/os/Build"),
        "Should have correct class path"
    );
}

#[test]
fn test_bind_util_log_from_android_jar() {
    let jar_path = require_android_sdk!();

    let bindings_obj = Builder::new()
        .input_jar(jar_path)
        .patterns(vec!["android.util.Log"])
        .generate()
        .expect("Failed to generate bindings from Android SDK");

    assert!(
        !bindings_obj.is_empty(),
        "Should generate some bindings from Android SDK"
    );

    let content = bindings_obj.to_string();

    // android.util.Log has static logging methods
    assert!(content.contains("pub Log =>"), "Should define Log binding");

    // Check for some common logging methods (they should be present as static methods)
    // Note: With arity-based naming:
    // - Methods with 2 args use base name (e.g., d, i, e)
    // - Methods with 3 args get 3 suffix (e.g., d3, i3, e3)
    // The first overload may use shorthand syntax (no braces) if the name is reversible
    assert!(content.contains("fn d(") || content.contains("fn d {"));
    assert!(content.contains("fn d3 {"));
    assert!(content.contains("fn i(") || content.contains("fn i {"));
    assert!(content.contains("fn i3 {"));
    assert!(content.contains("fn e(") || content.contains("fn e {"));
    assert!(content.contains("fn e3 {"));
}

#[test]
fn test_bind_graphics_point_from_android_jar() {
    let jar_path = require_android_sdk!();

    let bindings_obj = Builder::new()
        .input_jar(jar_path)
        .patterns(vec!["android.graphics.Point"])
        .generate()
        .expect("Failed to generate bindings from Android SDK");

    assert!(
        !bindings_obj.is_empty(),
        "Should generate some bindings from Android SDK"
    );

    let content = bindings_obj.to_string();

    if !content.contains("Point") {
        println!("Did not find Point class in generated bindings");
        println!("Generated {} total bindings", bindings_obj.len());
    }

    assert!(
        content.contains("Point"),
        "Should have Point class bindings"
    );

    assert!(
        content.contains("pub Point =>"),
        "Should define Point binding"
    );

    assert!(
        content.contains("android.graphics.Point") || content.contains("android/graphics/Point"),
        "Should have correct class path"
    );

    // Point has constructors and methods like equals, set, etc.
    assert!(
        content.contains("fn new(") || content.contains("constructors"),
        "Should have constructor or methods"
    );

    println!("Generated bindings for android.graphics.Point:");
    println!("{}", content);
}

#[test]
fn test_bind_graphics_from_android_jar() {
    let jar_path = require_android_sdk!();

    let all_bindings_obj = Builder::new()
        .input_jar(jar_path)
        .patterns(vec!["android.graphics.*"])
        .generate()
        .expect("Failed to generate bindings from Android SDK");

    // Should generate bindings for multiple classes in android.graphics
    assert!(
        all_bindings_obj.len() > 1,
        "Should generate bindings for multiple classes in android.graphics package"
    );

    println!(
        "Generated {} class bindings from android.graphics package",
        all_bindings_obj.len()
    );

    let content = all_bindings_obj.to_string();

    // Verify some expected classes are present
    let has_point = content.contains("Point");
    let has_rect = content.contains("Rect");

    assert!(
        has_point || has_rect,
        "Should include at least Point or Rect from android.graphics"
    );
}
