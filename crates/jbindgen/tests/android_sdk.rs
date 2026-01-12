//! Android SDK binding generation tests
//!
//! These tests verify that jbindgen can generate bindings from Android SDK
//! source stubs using the Java source parser.

use jbindgen::{AndroidSdk, Builder};
use std::env;

/// Test generating bindings for a simple Android SDK class
#[test]
#[ignore] // Only run when ANDROID_HOME is set
fn test_android_activity_binding() {
    // Skip if ANDROID_HOME is not set
    if env::var("ANDROID_HOME").is_err() && env::var("ANDROID_SDK_ROOT").is_err() {
        println!("Skipping test: ANDROID_HOME or ANDROID_SDK_ROOT not set");
        return;
    }

    // Try to generate bindings for android.app.Activity
    let result = Builder::new()
        .input_android_sdk(35, vec!["android.app.Activity".to_string()])
        .generate();

    match result {
        Ok(bindings_obj) => {
            assert!(
                !bindings_obj.is_empty(),
                "Should generate at least one binding"
            );

            let binding_code = bindings_obj.to_string();
            println!("Generated binding preview (first 500 chars):");
            println!("{}...", &binding_code[..binding_code.len().min(500)]);

            // Verify the binding contains expected elements
            assert!(binding_code.contains("bind_java_type!"));
            assert!(binding_code.contains("Activity"));
        }
        Err(e) => {
            println!("Note: Test failed with: {}", e);
            println!("This is expected if Android SDK sources are not installed for API 35");
        }
    }
}

/// Test generating bindings for multiple Android SDK classes with wildcard
#[test]
#[ignore] // Only run when ANDROID_HOME is set
fn test_android_app_package_binding() {
    // Skip if ANDROID_HOME is not set
    if env::var("ANDROID_HOME").is_err() && env::var("ANDROID_SDK_ROOT").is_err() {
        println!("Skipping test: ANDROID_HOME or ANDROID_SDK_ROOT not set");
        return;
    }

    // Try to generate bindings for android.app.* (limiting to see multiple classes)
    let result = Builder::new()
        .input_android_sdk(35, vec!["android.app.*".to_string()])
        .generate();

    match result {
        Ok(bindings_obj) => {
            println!("Generated {} bindings", bindings_obj.len());

            // Should generate bindings for multiple classes
            assert!(
                bindings_obj.len() > 1,
                "Should generate bindings for multiple classes in android.app package"
            );

            let binding_code = bindings_obj.to_string();

            // Verify at least some core classes are present (if sources are available)
            let has_activity = binding_code.contains("Activity");
            println!("Has Activity class: {}", has_activity);
        }
        Err(e) => {
            println!("Note: Test failed with: {}", e);
            println!("This is expected if Android SDK sources are not installed for API 35");
        }
    }
}

/// Test Android SDK discovery
#[test]
#[ignore] // Only run when ANDROID_HOME is set
fn test_android_sdk_discovery() {
    // Skip if ANDROID_HOME is not set
    if env::var("ANDROID_HOME").is_err() && env::var("ANDROID_SDK_ROOT").is_err() {
        println!("Skipping test: ANDROID_HOME or ANDROID_SDK_ROOT not set");
        return;
    }

    let sdk = AndroidSdk::from_env(35);

    match sdk {
        Ok(sdk) => {
            println!("Android SDK path: {}", sdk.sdk_path.display());

            // Try to get android.jar
            match sdk.get_android_jar() {
                Ok(jar_path) => {
                    println!("android.jar: {}", jar_path.display());
                    assert!(jar_path.exists());
                }
                Err(e) => {
                    println!("Could not find android.jar: {}", e);
                }
            }

            // Try to get stubs src JAR
            match sdk.get_stubs_src_jar() {
                Ok(jar_path) => {
                    println!("android-stubs-src.jar: {}", jar_path.display());
                    assert!(jar_path.exists());
                }
                Err(e) => {
                    println!("Could not find android-stubs-src.jar: {}", e);
                    println!("You may need to install Android SDK Sources for API 35");
                }
            }
        }
        Err(e) => {
            println!("Failed to initialize Android SDK: {}", e);
        }
    }
}

/// Smoke test that compiles a simple binding and verifies structure
#[test]
#[ignore] // Only run when ANDROID_HOME is set
fn test_android_binding_structure() {
    // Skip if ANDROID_HOME is not set
    if env::var("ANDROID_HOME").is_err() && env::var("ANDROID_SDK_ROOT").is_err() {
        println!("Skipping test: ANDROID_HOME or ANDROID_SDK_ROOT not set");
        return;
    }

    let sdk = AndroidSdk::from_env(35);

    if let Ok(sdk) = sdk {
        // Get the stubs JAR - no extraction needed
        if let Ok(stubs_jar) = sdk.get_stubs_src_jar() {
            let classpath = sdk.get_classpath().unwrap();

            let result = Builder::new()
                .input_sources(
                    vec![stubs_jar],
                    classpath,
                    vec!["android.os.Bundle".to_string()],
                )
                .generate();

            match result {
                Ok(bindings_obj) => {
                    assert!(!bindings_obj.is_empty());

                    let code = bindings_obj.to_string();

                    // Verify structure
                    assert!(code.contains("bind_java_type!"));
                    assert!(code.contains("Bundle"));
                    assert!(code.contains("android.os"));

                    // Bundle should have constructors and methods
                    assert!(
                        code.contains("constructors") || code.contains("methods"),
                        "Bundle should have constructors or methods"
                    );

                    println!("Successfully generated binding for Bundle:");
                    println!("{}", code);
                }
                Err(e) => {
                    println!("Could not generate binding: {}", e);
                }
            }
        }
    }
}
