//! Build script to generate TestActivity bindings using jbindgen

use std::env;
use std::path::{Path, PathBuf};

fn main() {
    // Get the Android SDK location
    let android_home = env::var("ANDROID_HOME")
        .or_else(|_| env::var("ANDROID_SDK_ROOT"))
        .expect("ANDROID_HOME or ANDROID_SDK_ROOT must be set");

    let android_jar = PathBuf::from(&android_home).join("platforms/android-35/android.jar");

    if !android_jar.exists() {
        panic!(
            "Android SDK jar not found at {}. Please install Android SDK platform-35.",
            android_jar.display()
        );
    }

    // Download hiddenapi-flags.csv if needed
    let hiddenapi_flags_path = download_hiddenapi_flags();

    // Get the output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let java_src =
        PathBuf::from("app/src/main/java/com/github/jni/jbindgen/testactivity/TestActivity.java");

    // Compile TestActivity.java with Android SDK
    println!("cargo:rerun-if-changed={}", java_src.display());
    println!("cargo:rerun-if-env-changed=ANDROID_HOME");
    println!("cargo:rerun-if-env-changed=ANDROID_SDK_ROOT");

    /*
    let class_files = javac::Build::new()
        .file(&java_src)
        .classpath(&android_jar)
        .output_dir(&out_dir)
        .compile();

    if class_files.is_empty() {
        panic!("No class files generated from TestActivity.java");
    }
    */

    // Generate Rust bindings using jbindgen
    // Note: We generate from source instead of class file to avoid issues with inner classes
    let bindings = jbindgen::Builder::new()
        .input_sources(
            vec![java_src.clone()],
            vec![android_jar.clone()],
            vec!["com.github.jni.jbindgen.testactivity.TestActivity".to_string()],
        )
        .generate()
        .expect("Failed to generate bindings");

    // Write bindings to output directory
    let bindings_path = out_dir.join("test_activity_bindings.rs");
    bindings
        .write_to_file(&bindings_path)
        .expect("Failed to write TestActivity bindings");

    println!(
        "cargo:warning=Generated TestActivity bindings at {}",
        bindings_path.display()
    );

    // Generate Android SDK bindings for each enabled feature
    #[cfg(feature = "sdk_util_time_utils")]
    generate_sdk_binding(
        &out_dir,
        &hiddenapi_flags_path,
        "util_time_utils",
        &["android.util.TimeUtils", "android.icu.util.TimeZone"],
        "time_utils_bindings.rs",
        None,
    );

    #[cfg(feature = "sdk_os_build")]
    generate_sdk_binding(
        &out_dir,
        &hiddenapi_flags_path,
        "os_build",
        &["android.os.Build"],
        "os_build_bindings.rs",
        Some("Android"),
    );

    #[cfg(feature = "sdk_os_binder")]
    generate_sdk_binding(
        &out_dir,
        &hiddenapi_flags_path,
        "os_binder",
        &["android.os.Binder"],
        "os_binder_bindings.rs",
        Some("Android"),
    );

    #[cfg(feature = "sdk_bluetooth")]
    generate_sdk_binding(
        &out_dir,
        &hiddenapi_flags_path,
        "bluetooth",
        &["android.bluetooth.le.*"],
        "bluetooth_bindings.rs",
        Some("A"),
    );

    #[cfg(feature = "sdk_content_intent")]
    generate_sdk_binding_with_skip(
        &out_dir,
        &hiddenapi_flags_path,
        "content_intent",
        &["android.content.Intent"],
        "content_intent_bindings.rs",
        Some("Android"),
        &["Landroid/content/Intent;->toURI()Ljava/lang/String;"],
    );

    #[cfg(feature = "sdk_net_uri")]
    generate_sdk_binding(
        &out_dir,
        &hiddenapi_flags_path,
        "net_uri",
        &["android.net.Uri"],
        "net_uri_bindings.rs",
        Some("Android"),
    );
}

fn download_hiddenapi_flags() -> PathBuf {
    // Use CARGO_TARGET_TMPDIR if available, otherwise fall back to OUT_DIR
    let tmp_dir = env::var("CARGO_TARGET_TMPDIR")
        .or_else(|_| env::var("OUT_DIR"))
        .expect("Neither CARGO_TARGET_TMPDIR nor OUT_DIR is set");
    
    let hiddenapi_flags_path = PathBuf::from(&tmp_dir).join("hiddenapi-flags.csv");
    
    // Download only if not already present
    if !hiddenapi_flags_path.exists() {
        println!("cargo:warning=Downloading hiddenapi-flags.csv...");
        
        let url = "https://dl.google.com/developers/android/baklava/non-sdk/hiddenapi-flags.csv";
        let response = ureq::get(url)
            .call()
            .expect("Failed to download hiddenapi-flags.csv");
        
        let mut reader = response.into_reader();
        let mut file = std::fs::File::create(&hiddenapi_flags_path)
            .expect("Failed to create hiddenapi-flags.csv");
        
        std::io::copy(&mut reader, &mut file)
            .expect("Failed to write hiddenapi-flags.csv");
        
        println!("cargo:warning=Downloaded hiddenapi-flags.csv to {}", hiddenapi_flags_path.display());
    } else {
        println!("cargo:warning=Using cached hiddenapi-flags.csv from {}", hiddenapi_flags_path.display());
    }
    
    hiddenapi_flags_path
}

fn generate_sdk_binding(
    out_dir: &Path,
    hiddenapi_flags_path: &Path,
    sub_module: &str,
    patterns: &[&str],
    output_file: &str,
    name_prefix: Option<&str>,
) {
    generate_sdk_binding_with_skip(out_dir, hiddenapi_flags_path, sub_module, patterns, output_file, name_prefix, &[]);
}

fn generate_sdk_binding_with_skip(
    out_dir: &Path,
    hiddenapi_flags_path: &Path,
    sub_module: &str,
    patterns: &[&str],
    output_file: &str,
    name_prefix: Option<&str>,
    skip_signatures: &[&str],
) {
    use jbindgen::Builder;

    println!(
        "cargo:warning=Generating bindings for: {}",
        patterns.join(", ")
    );

    let mut builder = Builder::new()
        .root_path(format!("crate::sdk::{}", sub_module))
        .input_android_sdk(35, patterns.iter().map(|s| s.to_string()).collect())
        .hiddenapi_flags(hiddenapi_flags_path.to_str().expect("Invalid hiddenapi_flags_path"));

    if let Some(name_prefix) = name_prefix {
        builder = builder.name_prefix(name_prefix.to_string());
    }

    for sig in skip_signatures {
        builder = builder.skip_signature(sig.to_string());
    }

    let result = builder.generate();

    match result {
        Ok(bindings_obj) => {
            let bindings_path = out_dir.join(output_file);
            bindings_obj
                .write_to_file(&bindings_path)
                .expect("Failed to write SDK bindings");

            println!(
                "cargo:warning=Generated {} bindings at {}",
                output_file,
                bindings_path.display()
            );
        }
        Err(e) => {
            println!(
                "cargo:warning=Failed to generate {} bindings: {}",
                output_file, e
            );
            println!("cargo:warning=Make sure Android SDK sources are installed for API 35");
        }
    }
}
