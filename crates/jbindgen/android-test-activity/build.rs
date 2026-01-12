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
        "util_time_utils",
        &["android.util.TimeUtils", "android.icu.util.TimeZone"],
        "time_utils_bindings.rs",
        None,
    );

    #[cfg(feature = "sdk_os_build")]
    generate_sdk_binding(
        &out_dir,
        "os_build",
        &["android.os.Build"],
        "os_build_bindings.rs",
        Some("Android"),
    );

    #[cfg(feature = "sdk_os_binder")]
    generate_sdk_binding(
        &out_dir,
        "os_binder",
        &["android.os.Binder"],
        "os_binder_bindings.rs",
        Some("Android"),
    );

    #[cfg(feature = "sdk_bluetooth")]
    generate_sdk_binding(
        &out_dir,
        "bluetooth",
        &["android.bluetooth.le.*"],
        "bluetooth_bindings.rs",
        Some("A"),
    );

    #[cfg(feature = "sdk_content_intent")]
    generate_sdk_binding_with_skip(
        &out_dir,
        "content_intent",
        &["android.content.Intent"],
        "content_intent_bindings.rs",
        Some("Android"),
        &["Landroid/content/Intent;->toURI()Ljava/lang/String;"],
    );

    #[cfg(feature = "sdk_net_uri")]
    generate_sdk_binding(
        &out_dir,
        "net_uri",
        &["android.net.Uri"],
        "net_uri_bindings.rs",
        Some("Android"),
    );
}

fn generate_sdk_binding(
    out_dir: &Path,
    sub_module: &str,
    patterns: &[&str],
    output_file: &str,
    name_prefix: Option<&str>,
) {
    generate_sdk_binding_with_skip(out_dir, sub_module, patterns, output_file, name_prefix, &[]);
}

fn generate_sdk_binding_with_skip(
    out_dir: &Path,
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
        .hiddenapi_flags("hiddenapi-flags.csv");

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
