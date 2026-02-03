//! jbindgen CLI - Generate Rust bindings from Java class files

use clap::{Parser, Subcommand};
use jbindgen::Builder;
use std::fs;
use std::path::PathBuf;
use std::process;

/// Generate Rust bindings from Java class files, JAR archives, or Android SDK
#[derive(Parser, Debug)]
#[command(name = "jbindgen")]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate bindings from .class or .jar files
    Classfile {
        /// Path to .class file or .jar archive
        #[arg(value_name = "INPUT")]
        input: PathBuf,

        /// Root module path for generated bindings (e.g., "crate::bindings::sdk")
        #[arg(long, default_value = "crate", value_name = "PATH")]
        root: String,

        /// Write output to FILE instead of stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Override the generated Rust type name (only for single .class files)
        #[arg(long, value_name = "NAME")]
        rust_name: Option<String>,

        /// Prefix to add to all generated Rust type names
        #[arg(long, value_name = "PREFIX")]
        name_prefix: Option<String>,

        /// Generate a private type (default: public)
        #[arg(long)]
        private: bool,

        /// Class patterns for filtering (e.g., com.example.*, android.app.Activity)
        /// Can be specified multiple times. Applies to JAR files.
        #[arg(long = "pattern", value_name = "PATTERN")]
        patterns: Vec<String>,

        /// For JAR files: output directory for generated bindings (one file per class)
        #[arg(long, value_name = "DIR")]
        output_dir: Option<PathBuf>,

        /// Additional type mappings in format "RustType=>java.foo.Type" (can be specified multiple times)
        #[arg(long = "type-map", value_name = "MAPPING")]
        type_maps: Vec<String>,

        /// Disable generation of native method interfaces (native methods will be emitted as callable methods)
        #[arg(long)]
        no_native_interfaces: bool,

        /// Disable generation of jni_init functions
        #[arg(long)]
        no_jni_init: bool,

        /// Skip methods/fields with the specified DEX signature (can be specified multiple times)
        /// Format: Lcom/example/Class;->methodName(Ljava/lang/String;)V
        #[arg(long = "skip", value_name = "SIGNATURE")]
        skip_signatures: Vec<String>,

        /// Override method/field name for the specified DEX signature (can be specified multiple times)
        /// Format: SIGNATURE=new_name (e.g., Landroid/content/Intent;->toURI(I)Ljava/lang/String;=to_uri_deprecated)
        #[arg(long = "name", value_name = "SIGNATURE=NAME")]
        name_overrides: Vec<String>,

        /// Write the final type map to a file
        #[arg(long, value_name = "FILE")]
        output_type_map: Option<PathBuf>,

        /// Public root path for type map output (e.g., "crate" instead of "crate::bindings")
        #[arg(long, value_name = "PATH")]
        public_root: Option<String>,
    },

    /// Generate bindings from Java source files
    Java {
        /// Path(s) to .java source file(s) or directory(ies) containing source files
        #[arg(value_name = "INPUT", required = true)]
        inputs: Vec<PathBuf>,

        /// Root module path for generated bindings (e.g., "crate::bindings::sdk")
        #[arg(long, default_value = "crate", value_name = "PATH")]
        root: String,

        /// Write output to FILE instead of stdout
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Override the generated Rust type name (only for single .java files)
        #[arg(long, value_name = "NAME")]
        rust_name: Option<String>,

        /// Prefix to add to all generated Rust type names
        #[arg(long, value_name = "PREFIX")]
        name_prefix: Option<String>,

        /// Generate a private type (default: public)
        #[arg(long)]
        private: bool,

        /// Class patterns for filtering (e.g., com.example.*, android.app.Activity)
        /// Can be specified multiple times.
        #[arg(long = "pattern", value_name = "PATTERN")]
        patterns: Vec<String>,

        /// Classpath entries for dependencies (can be specified multiple times)
        #[arg(long = "classpath", value_name = "PATH")]
        classpaths: Vec<PathBuf>,

        /// Output directory for generated bindings (one file per class)
        #[arg(long, value_name = "DIR")]
        output_dir: Option<PathBuf>,

        /// Additional type mappings in format "RustType=>java.foo.Type" (can be specified multiple times)
        #[arg(long = "type-map", value_name = "MAPPING")]
        type_maps: Vec<String>,

        /// Disable generation of native method interfaces (native methods will be emitted as callable methods)
        #[arg(long)]
        no_native_interfaces: bool,

        /// Disable generation of jni_init functions
        #[arg(long)]
        no_jni_init: bool,

        /// Skip methods/fields with the specified DEX signature (can be specified multiple times)
        /// Format: Lcom/example/Class;->methodName(Ljava/lang/String;)V
        #[arg(long = "skip", value_name = "SIGNATURE")]
        skip_signatures: Vec<String>,

        /// Override method/field name for the specified DEX signature (can be specified multiple times)
        /// Format: SIGNATURE=new_name (e.g., Landroid/content/Intent;->toURI(I)Ljava/lang/String;=to_uri_deprecated)
        #[arg(long = "name", value_name = "SIGNATURE=NAME")]
        name_overrides: Vec<String>,

        /// Write the final type map to a file
        #[arg(long, value_name = "FILE")]
        output_type_map: Option<PathBuf>,

        /// Public root path for type map output (e.g., "crate" instead of "crate::bindings")
        #[arg(long, value_name = "PATH")]
        public_root: Option<String>,
    },

    /// Output annotation source files for vendoring
    Annotations {
        /// Output directory for annotation files (default: current directory)
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,
    },

    /// Generate bindings from Android SDK
    Android {
        /// Android API level (e.g., 33, 34, 35)
        #[arg(short, long, value_name = "LEVEL")]
        api_level: u32,

        /// Root module path for generated bindings (e.g., "crate::bindings::sdk")
        #[arg(long, default_value = "crate", value_name = "PATH")]
        root: String,

        /// Class patterns to match (e.g., android.app.*, android.os.Build)
        /// Can be specified multiple times.
        #[arg(long = "pattern", value_name = "PATTERN", required = true)]
        patterns: Vec<String>,

        /// Output directory for generated bindings (one file per class)
        #[arg(short, long, value_name = "DIR")]
        output_dir: Option<PathBuf>,

        /// Write output to FILE instead of stdout (for single class)
        #[arg(short = 'f', long, value_name = "FILE")]
        output_file: Option<PathBuf>,

        /// Prefix to add to all generated Rust type names
        #[arg(long, value_name = "PREFIX")]
        name_prefix: Option<String>,

        /// Generate a private type (default: public)
        #[arg(long)]
        private: bool,

        /// Additional type mappings in format "RustType=>java.foo.Type" (can be specified multiple times)
        #[arg(long = "type-map", value_name = "MAPPING")]
        type_maps: Vec<String>,

        /// Path to hiddenapi-flags.csv for filtering out hidden/non-public APIs
        #[arg(long, value_name = "FILE")]
        hiddenapi_flags: Option<PathBuf>,

        /// Allow "unsupported" APIs when using --hiddenapi-flags (default: only "public-api" and "sdk" APIs)
        #[arg(long)]
        allow_unsupported: bool,

        /// Maximum target level for conditional API support (e.g., "o", "p", "q").
        /// When using --hiddenapi-flags, include APIs with max-target-<level> if level >= this value.
        /// Example: --max-target=o includes APIs marked max-target-o, max-target-p, max-target-q, etc.
        #[arg(long, value_name = "LEVEL")]
        max_target: Option<String>,

        /// Disable generation of jni_init functions
        #[arg(long)]
        no_jni_init: bool,

        /// Skip methods/fields with the specified DEX signature (can be specified multiple times)
        /// Format: Lcom/example/Class;->methodName(Ljava/lang/String;)V
        #[arg(long = "skip", value_name = "SIGNATURE")]
        skip_signatures: Vec<String>,

        /// Override method/field name for the specified DEX signature (can be specified multiple times)
        /// Format: SIGNATURE=new_name (e.g., Landroid/content/Intent;->toURI(I)Ljava/lang/String;=to_uri_deprecated)
        #[arg(long = "name", value_name = "SIGNATURE=NAME")]
        name_overrides: Vec<String>,

        /// Write the final type map to a file
        #[arg(long, value_name = "FILE")]
        output_type_map: Option<PathBuf>,

        /// Public root path for type map output (e.g., "crate" instead of "crate::bindings")
        #[arg(long, value_name = "PATH")]
        public_root: Option<String>,
    },
}

fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter_module("jbindgen", log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let args = Args::parse();

    match args.command {
        Command::Classfile {
            input,
            root,
            output,
            rust_name,
            name_prefix,
            private,
            patterns,
            output_dir,
            type_maps,
            no_native_interfaces,
            no_jni_init,
            skip_signatures,
            name_overrides,
            output_type_map,
            public_root,
        } => {
            handle_classfile(
                root,
                input,
                output,
                rust_name,
                name_prefix,
                private,
                patterns,
                output_dir,
                type_maps,
                no_native_interfaces,
                no_jni_init,
                skip_signatures,
                name_overrides,
                output_type_map,
                public_root,
            );
        }
        Command::Java {
            inputs,
            root,
            output,
            rust_name,
            name_prefix,
            private,
            patterns,
            classpaths,
            output_dir,
            type_maps,
            no_native_interfaces,
            no_jni_init,
            skip_signatures,
            name_overrides,
            output_type_map,
            public_root,
        } => {
            handle_java(
                root,
                inputs,
                output,
                rust_name,
                name_prefix,
                private,
                patterns,
                classpaths,
                output_dir,
                type_maps,
                no_native_interfaces,
                no_jni_init,
                skip_signatures,
                name_overrides,
                output_type_map,
                public_root,
            );
        }
        Command::Annotations { output } => {
            handle_annotations(output);
        }
        Command::Android {
            api_level,
            root,
            patterns,
            output_dir,
            output_file,
            name_prefix,
            private,
            type_maps,
            hiddenapi_flags,
            allow_unsupported,
            max_target,
            no_jni_init,
            skip_signatures,
            name_overrides,
            output_type_map,
            public_root,
        } => {
            handle_android(
                root,
                api_level,
                patterns,
                output_dir,
                output_file,
                name_prefix,
                private,
                type_maps,
                hiddenapi_flags,
                allow_unsupported,
                max_target,
                no_jni_init,
                skip_signatures,
                name_overrides,
                output_type_map,
                public_root,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_classfile(
    root_path: String,
    input: PathBuf,
    output: Option<PathBuf>,
    rust_name: Option<String>,
    name_prefix: Option<String>,
    private: bool,
    patterns: Vec<String>,
    output_dir: Option<PathBuf>,
    type_maps: Vec<String>,
    no_native_interfaces: bool,
    no_jni_init: bool,
    skip_signatures: Vec<String>,
    name_overrides: Vec<String>,
    output_type_map: Option<PathBuf>,
    public_root: Option<String>,
) {
    if !input.exists() {
        eprintln!("Error: Input file not found: {}", input.display());
        process::exit(1);
    }

    let extension = input.extension().and_then(|s| s.to_str()).unwrap_or("");

    match extension {
        "class" => {
            // Build configuration using Builder API
            let mut builder = Builder::new()
                .root_path(root_path)
                .input_class(input.clone())
                .public_type(!private)
                .generate_native_interfaces(!no_native_interfaces)
                .generate_jni_init(!no_jni_init);

            if let Some(name) = rust_name {
                builder = builder.rust_type_name(name);
            }

            if let Some(prefix) = name_prefix {
                builder = builder.name_prefix(prefix);
            }

            // Parse and add type map arguments
            let extra_type_map = match parse_type_map_args(&type_maps) {
                Ok(map) => map,
                Err(e) => {
                    eprintln!("Error parsing --type-map: {}", e);
                    process::exit(1);
                }
            };

            for (rust_type, java_type) in extra_type_map {
                builder = builder.type_mapping(rust_type, java_type);
            }

            // Parse and add skip signatures
            for signature in skip_signatures {
                builder = builder.skip_signature(signature);
            }

            // Parse and add name overrides from SIGNATURE=NAME format
            for override_spec in name_overrides {
                if let Some((sig, name)) = override_spec.split_once('=') {
                    builder = builder.name_override(sig, name);
                } else {
                    eprintln!(
                        "Error: --name argument must be in format SIGNATURE=NAME, got: {}",
                        override_spec
                    );
                    process::exit(1);
                }
            }

            let bindings = match builder.generate() {
                Ok(bindings) => bindings,
                Err(e) => {
                    eprintln!("Error generating bindings: {}", e);
                    process::exit(1);
                }
            };

            // Write type map if requested
            if let Some(ref type_map_path) = output_type_map {
                let write_result = if let Some(ref pub_root) = public_root {
                    bindings.write_pub_type_map(type_map_path, pub_root)
                } else {
                    bindings.write_type_map(type_map_path)
                };

                if let Err(e) = write_result {
                    eprintln!("Error writing type map: {}", e);
                    process::exit(1);
                }
                eprintln!("Type map written to {}", type_map_path.display());
            }

            // Write output
            if let Some(ref output_path) = output {
                if let Err(e) = bindings.write_to_file(output_path) {
                    eprintln!("Error writing output file: {}", e);
                    process::exit(1);
                }
                eprintln!("Bindings written to {}", output_path.display());
            } else {
                print!("{}", bindings.to_string());
            }
        }
        "jar" => {
            if rust_name.is_some() {
                eprintln!("Warning: --rust-name is ignored for JAR files");
            }

            // Build configuration using Builder API
            let mut builder = Builder::new()
                .root_path(root_path)
                .input_jar(input.clone())
                .public_type(!private)
                .generate_native_interfaces(!no_native_interfaces)
                .generate_jni_init(!no_jni_init);

            if let Some(prefix) = name_prefix {
                builder = builder.name_prefix(prefix);
            }

            // Add patterns for filtering if specified
            if !patterns.is_empty() {
                builder = builder.patterns(patterns);
            }

            // Parse and add type map arguments
            let extra_type_map = match parse_type_map_args(&type_maps) {
                Ok(map) => map,
                Err(e) => {
                    eprintln!("Error parsing --type-map: {}", e);
                    process::exit(1);
                }
            };

            for (rust_type, java_type) in extra_type_map {
                builder = builder.type_mapping(rust_type, java_type);
            }

            // Parse and add skip signatures
            for signature in skip_signatures {
                builder = builder.skip_signature(signature);
            }

            // Parse and add name overrides from SIGNATURE=NAME format
            for override_spec in name_overrides {
                if let Some((sig, name)) = override_spec.split_once('=') {
                    builder = builder.name_override(sig, name);
                } else {
                    eprintln!(
                        "Error: --name argument must be in format SIGNATURE=NAME, got: {}",
                        override_spec
                    );
                    process::exit(1);
                }
            }

            // Generate bindings for all classes in JAR
            let bindings = match builder.generate() {
                Ok(bindings) => bindings,
                Err(e) => {
                    eprintln!("Error reading JAR file: {}", e);
                    process::exit(1);
                }
            };

            if bindings.is_empty() {
                eprintln!("Warning: No class files found in JAR");
                return;
            }

            // Write type map if requested
            if let Some(ref type_map_path) = output_type_map {
                let write_result = if let Some(ref pub_root) = public_root {
                    bindings.write_pub_type_map(type_map_path, pub_root)
                } else {
                    bindings.write_type_map(type_map_path)
                };

                if let Err(e) = write_result {
                    eprintln!("Error writing type map: {}", e);
                    process::exit(1);
                }
                eprintln!("Type map written to {}", type_map_path.display());
            }

            // For JAR files, bindings are now organized by module
            // If output_dir is specified, create a module hierarchy
            if let Some(ref output_dir) = output_dir {
                if let Err(e) = bindings.write_to_files(output_dir) {
                    eprintln!("Error writing bindings to directory: {}", e);
                    process::exit(1);
                }

                println!("Generated module hierarchy in: {}", output_dir.display());
            } else {
                let bindings_str = bindings.to_string();
                write_output(&bindings_str, output.as_ref());
            }
        }
        _ => {
            eprintln!("Error: Unsupported file type. Expected .class or .jar file");
            process::exit(1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_java(
    root_path: String,
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    rust_name: Option<String>,
    name_prefix: Option<String>,
    private: bool,
    patterns: Vec<String>,
    classpaths: Vec<PathBuf>,
    output_dir: Option<PathBuf>,
    type_maps: Vec<String>,
    no_native_interfaces: bool,
    no_jni_init: bool,
    skip_signatures: Vec<String>,
    name_overrides: Vec<String>,
    output_type_map: Option<PathBuf>,
    public_root: Option<String>,
) {
    // Verify all inputs exist and collect source paths
    let mut source_paths = Vec::new();

    for input in &inputs {
        if !input.exists() {
            eprintln!("Error: Input path not found: {}", input.display());
            process::exit(1);
        }

        if input.is_file() {
            // Check it's a .java file
            if input.extension().and_then(|s| s.to_str()) != Some("java") {
                eprintln!(
                    "Error: Input file must be a .java file: {}",
                    input.display()
                );
                process::exit(1);
            }
            source_paths.push(input.clone());
        } else if input.is_dir() {
            source_paths.push(input.clone());
        } else {
            eprintln!(
                "Error: Input must be a .java file or directory: {}",
                input.display()
            );
            process::exit(1);
        }
    }

    // Build configuration using Builder API
    let pattern_vec = if patterns.is_empty() {
        vec!["**".to_string()]
    } else {
        patterns
    };

    let mut builder = Builder::new()
        .root_path(root_path)
        .input_sources(source_paths, classpaths, pattern_vec)
        .public_type(!private)
        .generate_native_interfaces(!no_native_interfaces)
        .generate_jni_init(!no_jni_init);

    if let Some(name) = rust_name {
        builder = builder.rust_type_name(name);
    }

    if let Some(prefix) = name_prefix {
        builder = builder.name_prefix(prefix);
    }

    // Parse and add type map arguments
    let extra_type_map = match parse_type_map_args(&type_maps) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("Error parsing --type-map: {}", e);
            process::exit(1);
        }
    };

    for (rust_type, java_type) in extra_type_map {
        builder = builder.type_mapping(rust_type, java_type);
    }

    // Parse and add skip signatures
    for signature in skip_signatures {
        builder = builder.skip_signature(signature);
    }

    // Parse and add name overrides from SIGNATURE=NAME format
    for override_spec in name_overrides {
        if let Some((sig, name)) = override_spec.split_once('=') {
            builder = builder.name_override(sig, name);
        } else {
            eprintln!(
                "Error: --name argument must be in format SIGNATURE=NAME, got: {}",
                override_spec
            );
            process::exit(1);
        }
    }

    // Generate bindings
    let bindings = match builder.generate() {
        Ok(bindings) => bindings,
        Err(e) => {
            eprintln!("Error generating bindings from Java source: {}", e);
            process::exit(1);
        }
    };

    if bindings.is_empty() {
        eprintln!("Warning: No classes found in Java source");
        return;
    }

    // Write type map if requested
    if let Some(ref type_map_path) = output_type_map {
        let write_result = if let Some(ref pub_root) = public_root {
            bindings.write_pub_type_map(type_map_path, pub_root)
        } else {
            bindings.write_type_map(type_map_path)
        };

        if let Err(e) = write_result {
            eprintln!("Error writing type map: {}", e);
            process::exit(1);
        }
        eprintln!("Type map written to {}", type_map_path.display());
    }

    // For Java source files, bindings are now organized by module
    // If output_dir is specified, create a module hierarchy
    if let Some(ref output_dir) = output_dir {
        if let Err(e) = bindings.write_to_files(output_dir) {
            eprintln!("Error writing bindings to directory: {}", e);
            process::exit(1);
        }

        println!("Generated module hierarchy in: {}", output_dir.display());
    } else {
        let bindings_str = bindings.to_string();
        write_output(&bindings_str, output.as_ref());
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_android(
    root_path: String,
    api_level: u32,
    patterns: Vec<String>,
    output_dir: Option<PathBuf>,
    output_file: Option<PathBuf>,
    name_prefix: Option<String>,
    private: bool,
    type_maps: Vec<String>,
    hiddenapi_flags: Option<PathBuf>,
    allow_unsupported: bool,
    max_target: Option<String>,
    no_jni_init: bool,
    skip_signatures: Vec<String>,
    name_overrides: Vec<String>,
    output_type_map: Option<PathBuf>,
    public_root: Option<String>,
) {
    // Parse name overrides from SIGNATURE:NAME format
    // Note: Field signatures contain ':' (e.g., Lclass;->field:Ltype;), so split at last ':'
    let mut name_override_map = std::collections::HashMap::new();
    for override_spec in name_overrides {
        if let Some((sig, name)) = override_spec.rsplit_once(':') {
            name_override_map.insert(sig.to_string(), name.to_string());
        } else {
            eprintln!(
                "Warning: Invalid --name format (expected SIGNATURE:NAME): {}",
                override_spec
            );
        }
    }

    // Build configuration using Builder API
    let pattern_count = patterns.len();
    let mut builder = Builder::new()
        .root_path(root_path)
        .input_android_sdk(api_level, patterns)
        .public_type(!private)
        .generate_jni_init(!no_jni_init);

    if let Some(prefix) = name_prefix {
        builder = builder.name_prefix(prefix);
    }

    if let Some(flags_path) = hiddenapi_flags {
        builder = builder.hiddenapi_flags(flags_path);
    }

    if allow_unsupported {
        builder = builder.allow_unsupported(true);
    }

    if let Some(target) = max_target {
        builder = builder.max_target(target);
    }

    // Add skip signatures
    for sig in skip_signatures {
        builder = builder.skip_signature(sig);
    }

    // Add name overrides
    for (sig, name) in name_override_map {
        builder = builder.name_override(sig, name);
    }

    // Parse and add type map arguments
    let extra_type_map = match parse_type_map_args(&type_maps) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("Error parsing --type-map: {}", e);
            process::exit(1);
        }
    };

    for (rust_type, java_type) in extra_type_map {
        builder = builder.type_mapping(rust_type, java_type);
    }

    eprintln!(
        "Generating Android bindings for API level {} with {} pattern(s)...",
        api_level, pattern_count
    );

    // Generate Android bindings
    let bindings = match builder.generate() {
        Ok(bindings) => bindings,
        Err(e) => {
            eprintln!("Error generating Android bindings: {}", e);
            eprintln!("\nMake sure:");
            eprintln!("  - ANDROID_HOME or ANDROID_SDK_ROOT is set");
            eprintln!("  - Android SDK API level {} is installed", api_level);
            process::exit(1);
        }
    };

    if bindings.is_empty() {
        eprintln!("Warning: No classes matched the specified patterns");
        return;
    }

    eprintln!("Generated {} binding(s)", bindings.len());

    // Write type map if requested
    if let Some(ref type_map_path) = output_type_map {
        let write_result = if let Some(ref pub_root) = public_root {
            bindings.write_pub_type_map(type_map_path, pub_root)
        } else {
            bindings.write_type_map(type_map_path)
        };

        if let Err(e) = write_result {
            eprintln!("Error writing type map: {}", e);
            process::exit(1);
        }
        eprintln!("Type map written to {}", type_map_path.display());
    }

    // For Android bindings, bindings are now organized by module
    // If output_dir is specified, create a module hierarchy
    if let Some(ref output_dir) = output_dir {
        if let Err(e) = bindings.write_to_files(output_dir) {
            eprintln!("Error writing bindings to directory: {}", e);
            process::exit(1);
        }

        println!("Generated module hierarchy in: {}", output_dir.display());
    } else {
        let bindings_str = bindings.to_string();
        write_output(&bindings_str, output_file.as_ref());
    }
}

fn write_output(content: &str, output_file: Option<&PathBuf>) {
    if let Some(output_path) = output_file {
        if let Err(e) = fs::write(output_path, content) {
            eprintln!("Error writing output file: {}", e);
            process::exit(1);
        }
        eprintln!("Bindings written to {}", output_path.display());
    } else {
        print!("{}", content);
    }
}

fn parse_type_map_args(type_map_args: &[String]) -> Result<Vec<(String, String)>, String> {
    let mut mappings = Vec::new();

    for arg in type_map_args {
        // Check if this is a file path
        if let Ok(content) = fs::read_to_string(arg) {
            // Read type mappings from file
            for (line_num, line) in content.lines().enumerate() {
                let line = line.trim();
                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                    continue;
                }

                // Parse "path::to::RustType => \"java.foo.Type\"" or "unsafe RustType => javaPrimitive" format
                let parts: Vec<&str> = line.split("=>").collect();
                if parts.len() != 2 {
                    return Err(format!(
                        "Invalid type-map format in file '{}' at line {}: '{}'. Expected format: RustType => \"java.foo.Type\" or unsafe RustType => javaPrimitive",
                        arg,
                        line_num + 1,
                        line
                    ));
                }

                let rust_type = parts[0].trim().to_string();
                let java_type = parts[1].trim().to_string();

                if rust_type.is_empty() || java_type.is_empty() {
                    return Err(format!(
                        "Empty type name in type-map file '{}' at line {}",
                        arg,
                        line_num + 1
                    ));
                }

                mappings.push((rust_type, java_type));
            }
        } else {
            // Parse as direct mapping: "RustType=>java.foo.Type" or "unsafe RustType=>javaPrimitive" format
            let parts: Vec<&str> = arg.split("=>").collect();
            if parts.len() != 2 {
                return Err(format!(
                    "Invalid type-map format '{}'. Expected format: RustType=>java.foo.Type or unsafe RustType=>javaPrimitive or path to file",
                    arg
                ));
            }

            let rust_type = parts[0].trim().to_string();
            let java_type = parts[1].trim().to_string();

            if rust_type.is_empty() || java_type.is_empty() {
                return Err(format!("Empty type name in type-map '{}'", arg));
            }

            mappings.push((rust_type, java_type));
        }
    }

    Ok(mappings)
}

/// Embedded annotation source files
const ANNOTATION_FILES: &[(&str, &str)] = &[
    (
        "RustName.java",
        include_str!("../../annotations/src/main/java/io/github/jni_rs/jbindgen/RustName.java"),
    ),
    (
        "RustPrimitive.java",
        include_str!(
            "../../annotations/src/main/java/io/github/jni_rs/jbindgen/RustPrimitive.java"
        ),
    ),
    (
        "RustSkip.java",
        include_str!("../../annotations/src/main/java/io/github/jni_rs/jbindgen/RustSkip.java"),
    ),
    (
        "package-info.java",
        include_str!("../../annotations/src/main/java/io/github/jni_rs/jbindgen/package-info.java"),
    ),
];

fn handle_annotations(output: Option<PathBuf>) {
    let base_dir = output.unwrap_or_else(|| PathBuf::from("."));
    let package_dir = base_dir.join("io/github/jni_rs/jbindgen");

    // Create the directory structure
    if let Err(e) = fs::create_dir_all(&package_dir) {
        eprintln!("Error creating directory {}: {}", package_dir.display(), e);
        process::exit(1);
    }

    // Write all annotation files
    for (filename, content) in ANNOTATION_FILES {
        let file_path = package_dir.join(filename);
        if let Err(e) = fs::write(&file_path, content) {
            eprintln!("Error writing {}: {}", file_path.display(), e);
            process::exit(1);
        }
        println!("Created: {}", file_path.display());
    }

    println!(
        "\nAnnotation files have been written to: {}",
        package_dir.display()
    );
    println!("\nYou can now use these annotations in your Java code:");
    println!("  import io.github.jni_rs.jbindgen.RustName;");
    println!("  import io.github.jni_rs.jbindgen.RustPrimitive;");
    println!("  import io.github.jni_rs.jbindgen.RustSkip;");
    println!("\nExample usage:");
    println!("  @RustName(\"CustomName\")");
    println!("  public class MyClass {{ }}");
    println!();
    println!("  public native String myMethod(@RustPrimitive(\"ThingHandle\") long handle);");
}
