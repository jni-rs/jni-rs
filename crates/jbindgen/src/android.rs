//! Android-specific binding generation with API filtering
//!
//! This module provides functionality for generating bindings for Android SDK APIs
//! by intersecting class definitions from android.jar (compiled bytecode) with
//! android-stubs-src.jar (source stubs), and optionally filtering based on
//! hiddenapi-flags.csv to exclude non-public/hidden APIs.
//!
//! ## Overview
//!
//! Android SDK bindings are generated through a multi-stage filtering process:
//!
//! 1. **Parse android.jar**: Extract class information from compiled bytecode (.class files)
//!    in the platform's android.jar. This contains the complete implementation.
//!
//! 2. **Parse android-stubs-src.jar**: Extract class information from Java source stubs.
//!    This represents the public API surface that Android officially exposes.
//!
//! 3. **Intersect**: Only keep constructors, methods, fields, and native methods that
//!    exist in BOTH sources. This ensures we only generate bindings for APIs that are
//!    both implemented and publicly documented.
//!
//! 4. **Filter hidden APIs** (optional): If a hiddenapi-flags.csv file is provided,
//!    further filter the APIs to exclude those marked as hidden or non-public by Android.
//!
//! ## Hidden API Filtering
//!
//! The hiddenapi-flags.csv file uses DEX-style signatures to identify each API member:
//!
//! - Constructor: `Landroid/util/TimeUtils;-><init>()V`
//! - Method: `Landroid/util/TimeUtils;->getTimeZone(IZJLjava/lang/String;)Ljava/util/TimeZone;`
//! - Field: `Landroid/os/Build;->BOARD:Ljava/lang/String;`
//!
//! Each line in the CSV contains a signature followed by comma-separated flags.
//!
//! ### Default Filtering (SDK APIs only)
//!
//! By default, only APIs marked with "public-api" or "sdk" flags are kept.
//!
//! ### Allow Unsupported APIs
//!
//! With `allow_unsupported` enabled, APIs marked as "unsupported" are also included.
//!
//! ### Maximum Target Level
//!
//! With `max_target` set to a target level (e.g., "o" for Android O), APIs with
//! `max-target-<level>` flags are included if their target level is >= the specified level.
//! For example, with `max_target = Some("o")`, APIs with `max-target-o`, `max-target-p`,
//! `max-target-q`, etc. will be included.
//!
//! ## Example
//!
//! ### Basic Usage (SDK APIs only)
//!
//! ```rust,ignore
//! use jbindgen::Builder;
//!
//! // Generate bindings with default filtering (sdk and public-api only)
//! let bindings = Builder::new()
//!     .input_android_sdk(35, vec!["android.os.Build".to_string()])
//!     .hiddenapi_flags("path/to/hiddenapi-flags.csv")
//!     .generate()?;
//! ```
//!
//! ### Include Unsupported APIs
//!
//! ```rust,ignore
//! use jbindgen::Builder;
//!
//! // Include APIs marked as "unsupported"
//! let bindings = Builder::new()
//!     .input_android_sdk(35, vec!["android.os.Build".to_string()])
//!     .hiddenapi_flags("path/to/hiddenapi-flags.csv")
//!     .allow_unsupported(true)
//!     .generate()?;
//! ```
//!
//! ### Include APIs up to Target Level
//!
//! ```rust,ignore
//! use jbindgen::Builder;
//!
//! // Include APIs with max-target-o and higher (Oreo+)
//! let bindings = Builder::new()
//!     .input_android_sdk(35, vec!["android.os.Build".to_string()])
//!     .hiddenapi_flags("path/to/hiddenapi-flags.csv")
//!     .max_target("o")
//!     .generate()?;
//! ```

use crate::android_sdk::AndroidSdk;
use crate::cafebabe_parser;
use crate::error::Result;
use crate::generator::{self, BindgenOptions, ModuleBinding, TypeMap};
use crate::java_parser;
use crate::parser_types::{ClassInfo, FieldInfo, MethodInfo};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

/// Configuration for filtering hidden APIs
#[derive(Debug, Clone, Default)]
pub struct HiddenApiFilter {
    /// Allow APIs marked as "unsupported"
    pub allow_unsupported: bool,
    /// Maximum target level (e.g., "o", "p", "q") for conditional API support
    /// When set, APIs with max-target-<level> are included if level >= this value
    pub max_target: Option<String>,
}

/// Generate Android SDK bindings with optional hidden API filtering
///
/// # Arguments
///
/// * `api_level` - Android API level (e.g., 33, 35)
/// * `patterns` - Patterns to match classes (e.g., ["android.app.*"])
/// * `hiddenapi_flags_path` - Optional path to hiddenapi-flags.csv for filtering
/// * `hiddenapi_filter` - Filter configuration for hidden API filtering (only used if hiddenapi_flags_path is Some)
/// * `options` - Binding generation options
/// * `extra_type_map` - Additional type mappings
pub fn generate_android_bindings(
    api_level: u32,
    patterns: &[String],
    hiddenapi_flags_path: Option<&Path>,
    hiddenapi_filter: &HiddenApiFilter,
    options: &BindgenOptions,
    extra_type_map: Vec<(String, String)>,
) -> Result<Vec<ModuleBinding>> {
    let sdk = AndroidSdk::from_env(api_level)?;

    // Parse android.jar (bytecode) for actual implementation
    let android_jar = sdk.get_android_jar()?;
    let jar_classes = parse_jar_classes(&android_jar, patterns)?;

    // Parse android-stubs-src.jar (source stubs) for public API surface
    let stubs_jar = sdk.get_stubs_src_jar()?;
    let classpath = sdk.get_classpath()?;
    let stub_classes = parse_source_stubs(&stubs_jar, &classpath, patterns)?;

    // Intersect the two sets
    let mut filtered_classes = intersect_classes(jar_classes, stub_classes)?;

    // Apply hidden API filtering if provided
    if let Some(flags_path) = hiddenapi_flags_path {
        let hidden_apis = parse_hiddenapi_flags(flags_path, hiddenapi_filter)?;
        filtered_classes = filter_hidden_apis(filtered_classes, &hidden_apis)?;
    }

    // Generate bindings from filtered classes
    let mut type_map = TypeMap::from_classes(filtered_classes.iter(), options);
    type_map.merge(extra_type_map);

    let mut results = Vec::new();
    for class_info in filtered_classes {
        let class_name = class_info.class_name.clone();
        match generator::generate_with_type_map(&class_info, options, &type_map) {
            Ok(binding) => {
                results.push(binding);
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to generate bindings for {}: {}",
                    class_name, e
                );
            }
        }
    }

    Ok(results)
}

/// Parse class files from a JAR archive
fn parse_jar_classes(jar_path: &Path, patterns: &[String]) -> Result<HashMap<String, ClassInfo>> {
    let file = fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut classes = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if !name.ends_with(".class") {
            continue;
        }

        // Filter by patterns if provided
        if !patterns.is_empty() && !matches_any_pattern(&name, patterns) {
            continue;
        }

        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        match cafebabe::parse_class(&bytes) {
            Ok(class) => match cafebabe_parser::parse_class(&class) {
                Ok(class_info) => {
                    // Use fully qualified class name as key (e.g., "android.app.Activity")
                    let key = class_info.class_name.replace('/', ".");
                    classes.insert(key, class_info);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse {}: {}", name, e);
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to parse cafebabe for {}: {}", name, e);
            }
        }
    }

    Ok(classes)
}

/// Parse source stubs from android-stubs-src.jar
fn parse_source_stubs(
    stubs_jar: &Path,
    classpath: &[PathBuf],
    patterns: &[String],
) -> Result<HashMap<String, ClassInfo>> {
    let pattern_str = if patterns.is_empty() {
        "*".to_string()
    } else {
        patterns.join(",")
    };

    let classes =
        java_parser::parse_java_sources(&[stubs_jar.to_path_buf()], classpath, &pattern_str)?;

    let mut result = HashMap::new();
    for class_info in classes {
        // Use fully qualified class name as key (e.g., "android.app.Activity")
        let key = class_info.class_name.replace('/', ".");
        result.insert(key, class_info);
    }

    Ok(result)
}

/// Check if a class name matches any of the given patterns
fn matches_any_pattern(name: &str, patterns: &[String]) -> bool {
    let normalized_name = name.trim_end_matches(".class").replace('/', ".");

    for pattern in patterns {
        let normalized_pattern = pattern.replace('/', ".");

        if normalized_pattern.ends_with(".*") {
            let prefix = normalized_pattern.trim_end_matches(".*");
            if normalized_name == prefix || normalized_name.starts_with(&format!("{}.", prefix)) {
                return true;
            }
        } else if normalized_pattern == "*" {
            return true;
        } else if normalized_name == normalized_pattern {
            return true;
        } else if normalized_pattern.contains('$') {
            continue;
        } else {
            if normalized_name.starts_with(&format!("{}$", normalized_pattern)) {
                return true;
            }
            if name.contains(&normalized_pattern.replace('.', "/")) {
                return true;
            }
        }
    }
    false
}

/// Intersect classes from JAR and source stubs, keeping only members present in both
fn intersect_classes(
    jar_classes: HashMap<String, ClassInfo>,
    stub_classes: HashMap<String, ClassInfo>,
) -> Result<Vec<ClassInfo>> {
    let mut result = Vec::new();

    for (class_name, jar_class) in jar_classes {
        // Only keep classes that exist in both sources
        if let Some(stub_class) = stub_classes.get(&class_name) {
            let intersected = intersect_class_members(&jar_class, stub_class);
            result.push(intersected);
        }
    }

    Ok(result)
}

/// Intersect members of two ClassInfo instances
fn intersect_class_members(jar_class: &ClassInfo, stub_class: &ClassInfo) -> ClassInfo {
    // Create signature sets from JAR for fast lookup
    // We'll keep stub members that exist in JAR to preserve documentation
    let jar_constructor_sigs: HashSet<String> = jar_class
        .constructors
        .iter()
        .map(|m| method_signature_key(m))
        .collect();

    let jar_method_sigs: HashSet<String> = jar_class
        .methods
        .iter()
        .map(|m| method_signature_key(m))
        .collect();

    let jar_native_sigs: HashSet<String> = jar_class
        .native_methods
        .iter()
        .map(|m| method_signature_key(m))
        .collect();

    let jar_field_sigs: HashSet<String> = jar_class
        .fields
        .iter()
        .map(|f| field_signature_key(f))
        .collect();

    // Filter stub class members to only those that exist in JAR
    // This preserves documentation and deprecation info from stubs
    let constructors = stub_class
        .constructors
        .iter()
        .filter(|m| jar_constructor_sigs.contains(&method_signature_key(m)))
        .cloned()
        .collect();

    let methods = stub_class
        .methods
        .iter()
        .filter(|m| jar_method_sigs.contains(&method_signature_key(m)))
        .cloned()
        .collect();

    let native_methods = stub_class
        .native_methods
        .iter()
        .filter(|m| jar_native_sigs.contains(&method_signature_key(m)))
        .cloned()
        .collect();

    let fields = stub_class
        .fields
        .iter()
        .filter(|f| jar_field_sigs.contains(&field_signature_key(f)))
        .cloned()
        .collect();

    ClassInfo {
        class_name: stub_class.class_name.clone(),
        package: stub_class.package.clone(),
        simple_name: stub_class.simple_name.clone(),
        documentation: stub_class.documentation.clone(),
        rust_name_override: stub_class.rust_name_override.clone(),
        constructors,
        methods,
        fields,
        native_methods,
        instance_of: stub_class.instance_of.clone(),
    }
}

/// Create a signature key for a method (name + parameter types + return type + static flag)
fn method_signature_key(method: &MethodInfo) -> String {
    let params: Vec<String> = method
        .signature
        .arguments
        .iter()
        .map(|arg| format_type_signature(&arg.type_info))
        .collect();

    format!(
        "{}({}){}:{}",
        method.name,
        params.join(","),
        format_type_signature(&method.signature.return_type),
        method.is_static
    )
}

/// Create a signature key for a field (name + type + static flag)
fn field_signature_key(field: &FieldInfo) -> String {
    format!(
        "{}:{}:{}",
        field.name,
        format_type_signature(&field.type_info),
        field.is_static
    )
}

/// Format a type signature for comparison
fn format_type_signature(type_info: &crate::parser_types::TypeInfo) -> String {
    let base = if type_info.is_primitive {
        type_info.name.clone()
    } else {
        type_info.name.clone()
    };

    if type_info.array_dimensions > 0 {
        format!("{}{}", base, "[]".repeat(type_info.array_dimensions))
    } else {
        base
    }
}

/// Parse hiddenapi-flags.csv and return set of allowed API signatures based on filter config
fn parse_hiddenapi_flags(path: &Path, filter: &HiddenApiFilter) -> Result<HashSet<String>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut allowed_apis = HashSet::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Format: signature,flag1,flag2,...
        let parts: Vec<&str> = line.split(',').collect();
        if parts.is_empty() {
            continue;
        }

        let signature = parts[0];
        let flags = &parts[1..];

        if is_api_allowed(flags, filter) {
            allowed_apis.insert(signature.to_string());
        }
    }

    Ok(allowed_apis)
}

/// Check if an API should be allowed based on its flags and filter configuration
fn is_api_allowed(flags: &[&str], filter: &HiddenApiFilter) -> bool {
    // Always allow public-api and sdk
    if flags.iter().any(|f| *f == "public-api" || *f == "sdk") {
        return true;
    }

    // Check for unsupported flag
    if filter.allow_unsupported && flags.iter().any(|f| *f == "unsupported") {
        return true;
    }

    // Check for max-target-<level> flags
    if let Some(ref max_target) = filter.max_target {
        for flag in flags {
            if let Some(target) = flag.strip_prefix("max-target-") {
                // Compare target levels
                // If the API's target level is >= our max_target, include it
                if compare_target_levels(target, max_target) >= 0 {
                    return true;
                }
            }
        }
    }

    false
}

/// Compare Android target levels (e.g., "o" < "p" < "q")
/// Returns: -1 if a < b, 0 if a == b, 1 if a > b
fn compare_target_levels(a: &str, b: &str) -> i32 {
    // Map common Android version codes to numbers for comparison
    // Note: This is a simplified mapping of Android version codenames
    let level_map = [
        ("l", 21), // Lollipop
        ("m", 23), // Marshmallow
        ("n", 24), // Nougat
        ("o", 26), // Oreo
        ("p", 28), // Pie
        ("q", 29), // Android 10
        ("r", 30), // Android 11
        ("s", 31), // Android 12
        ("t", 33), // Android 13
        ("u", 34), // Android 14
        ("v", 35), // Android 15
    ];

    let a_level = level_map.iter().find(|(k, _)| *k == a).map(|(_, v)| *v);
    let b_level = level_map.iter().find(|(k, _)| *k == b).map(|(_, v)| *v);

    match (a_level, b_level) {
        (Some(a_val), Some(b_val)) => {
            if a_val < b_val {
                -1
            } else if a_val > b_val {
                1
            } else {
                0
            }
        }
        // If we can't parse one of the levels, do string comparison as fallback
        _ => a.cmp(b) as i32,
    }
}

/// Filter classes to remove hidden APIs based on hiddenapi-flags
fn filter_hidden_apis(
    classes: Vec<ClassInfo>,
    hidden_apis: &HashSet<String>,
) -> Result<Vec<ClassInfo>> {
    let mut result = Vec::new();

    for class in classes {
        let filtered = filter_class_hidden_apis(class, hidden_apis);
        result.push(filtered);
    }

    Ok(result)
}

/// Filter a single class's members based on hidden API flags
fn filter_class_hidden_apis(class: ClassInfo, hidden_apis: &HashSet<String>) -> ClassInfo {
    let class_name_dex = class.class_name.replace('.', "/");

    let constructors = class
        .constructors
        .into_iter()
        .filter(|m| {
            let sig = method_to_dex_signature(&class_name_dex, m);
            hidden_apis.contains(&sig)
        })
        .collect();

    let methods = class
        .methods
        .into_iter()
        .filter(|m| {
            let sig = method_to_dex_signature(&class_name_dex, m);
            hidden_apis.contains(&sig)
        })
        .collect();

    let native_methods = class
        .native_methods
        .into_iter()
        .filter(|m| {
            let sig = method_to_dex_signature(&class_name_dex, m);
            hidden_apis.contains(&sig)
        })
        .collect();

    let fields = class
        .fields
        .into_iter()
        .filter(|f| {
            let sig = field_to_dex_signature(&class_name_dex, f);
            hidden_apis.contains(&sig)
        })
        .collect();

    ClassInfo {
        class_name: class.class_name,
        package: class.package,
        simple_name: class.simple_name,
        documentation: class.documentation,
        rust_name_override: class.rust_name_override,
        constructors,
        methods,
        fields,
        native_methods,
        instance_of: class.instance_of,
    }
}

/// Convert a method to DEX-style signature
/// Format: Lclass/name;->methodName(params)returnType
/// Example: Landroid/util/TimeUtils;->getTimeZone(IZJLjava/lang/String;)Ljava/util/TimeZone;
fn method_to_dex_signature(class_name: &str, method: &MethodInfo) -> String {
    let method_name = if method.is_constructor {
        "<init>"
    } else {
        &method.name
    };

    let params: Vec<String> = method
        .signature
        .arguments
        .iter()
        .map(|arg| type_to_dex_descriptor(&arg.type_info))
        .collect();

    let return_type = type_to_dex_descriptor(&method.signature.return_type);

    format!(
        "L{};->{}({}){}",
        class_name,
        method_name,
        params.join(""),
        return_type
    )
}

/// Convert a field to DEX-style signature
/// Format: Lclass/name;->fieldName:Type
/// Example: Landroid/os/Build;->BOARD:Ljava/lang/String;
fn field_to_dex_signature(class_name: &str, field: &FieldInfo) -> String {
    format!(
        "L{};->{}:{}",
        class_name,
        field.name,
        type_to_dex_descriptor(&field.type_info)
    )
}

/// Convert TypeInfo to DEX descriptor
/// Examples:
/// - int -> I
/// - java.lang.String -> Ljava/lang/String;
/// - int[] -> [I
/// - String[][] -> [[Ljava/lang/String;
fn type_to_dex_descriptor(type_info: &crate::parser_types::TypeInfo) -> String {
    let array_prefix = "[".repeat(type_info.array_dimensions);

    let base_descriptor = if type_info.is_primitive {
        match type_info.name.as_str() {
            "boolean" => "Z",
            "byte" => "B",
            "char" => "C",
            "short" => "S",
            "int" => "I",
            "long" => "J",
            "float" => "F",
            "double" => "D",
            "void" => "V",
            _ => panic!("Unknown primitive type: {}", type_info.name),
        }
        .to_string()
    } else {
        // Object type: java.lang.String -> Ljava/lang/String;
        let class_path = type_info.name.replace('.', "/");
        format!("L{};", class_path)
    };

    format!("{}{}", array_prefix, base_descriptor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_types::{MethodSignature, TypeInfo};

    #[test]
    fn test_type_to_dex_descriptor() {
        let int_type = TypeInfo {
            name: "int".to_string(),
            array_dimensions: 0,
            is_primitive: true,
        };
        assert_eq!(type_to_dex_descriptor(&int_type), "I");

        let string_type = TypeInfo {
            name: "java.lang.String".to_string(),
            array_dimensions: 0,
            is_primitive: false,
        };
        assert_eq!(type_to_dex_descriptor(&string_type), "Ljava/lang/String;");

        let int_array = TypeInfo {
            name: "int".to_string(),
            array_dimensions: 1,
            is_primitive: true,
        };
        assert_eq!(type_to_dex_descriptor(&int_array), "[I");

        let string_array_2d = TypeInfo {
            name: "java.lang.String".to_string(),
            array_dimensions: 2,
            is_primitive: false,
        };
        assert_eq!(
            type_to_dex_descriptor(&string_array_2d),
            "[[Ljava/lang/String;"
        );
    }

    #[test]
    fn test_method_to_dex_signature() {
        let method = MethodInfo {
            name: "getTimeZone".to_string(),
            documentation: None,
            rust_name_override: None,
            signature: MethodSignature {
                arguments: vec![
                    crate::parser_types::ArgInfo {
                        name: None,
                        type_info: TypeInfo {
                            name: "int".to_string(),
                            array_dimensions: 0,
                            is_primitive: true,
                        },
                    },
                    crate::parser_types::ArgInfo {
                        name: None,
                        type_info: TypeInfo {
                            name: "boolean".to_string(),
                            array_dimensions: 0,
                            is_primitive: true,
                        },
                    },
                    crate::parser_types::ArgInfo {
                        name: None,
                        type_info: TypeInfo {
                            name: "long".to_string(),
                            array_dimensions: 0,
                            is_primitive: true,
                        },
                    },
                    crate::parser_types::ArgInfo {
                        name: None,
                        type_info: TypeInfo {
                            name: "java.lang.String".to_string(),
                            array_dimensions: 0,
                            is_primitive: false,
                        },
                    },
                ],
                return_type: TypeInfo {
                    name: "java.util.TimeZone".to_string(),
                    array_dimensions: 0,
                    is_primitive: false,
                },
            },
            is_static: false,
            is_constructor: false,
            is_native: false,
            is_deprecated: false,
            is_public: true,
        };

        let sig = method_to_dex_signature("android/util/TimeUtils", &method);
        assert_eq!(
            sig,
            "Landroid/util/TimeUtils;->getTimeZone(IZJLjava/lang/String;)Ljava/util/TimeZone;"
        );
    }

    #[test]
    fn test_constructor_to_dex_signature() {
        let constructor = MethodInfo {
            name: "<init>".to_string(),
            documentation: None,
            rust_name_override: None,
            signature: MethodSignature {
                arguments: vec![],
                return_type: TypeInfo {
                    name: "void".to_string(),
                    array_dimensions: 0,
                    is_primitive: true,
                },
            },
            is_static: false,
            is_constructor: true,
            is_native: false,
            is_deprecated: false,
            is_public: true,
        };

        let sig = method_to_dex_signature("android/util/TimeUtils", &constructor);
        assert_eq!(sig, "Landroid/util/TimeUtils;-><init>()V");
    }

    #[test]
    fn test_field_to_dex_signature() {
        let field = FieldInfo {
            name: "BOARD".to_string(),
            documentation: None,
            rust_name_override: None,
            type_info: TypeInfo {
                name: "java.lang.String".to_string(),
                array_dimensions: 0,
                is_primitive: false,
            },
            is_static: true,
            is_final: true,
            is_deprecated: false,
        };

        let sig = field_to_dex_signature("android/os/Build", &field);
        assert_eq!(sig, "Landroid/os/Build;->BOARD:Ljava/lang/String;");
    }

    #[test]
    fn test_is_api_allowed_default() {
        let filter = HiddenApiFilter::default();

        // public-api should always be allowed
        assert!(is_api_allowed(&["public-api"], &filter));

        // sdk should always be allowed
        assert!(is_api_allowed(&["sdk"], &filter));

        // unsupported should not be allowed by default
        assert!(!is_api_allowed(&["unsupported"], &filter));

        // max-target flags should not be allowed without max_target set
        assert!(!is_api_allowed(&["max-target-o"], &filter));

        // blocked should never be allowed
        assert!(!is_api_allowed(&["blocked"], &filter));
    }

    #[test]
    fn test_is_api_allowed_with_unsupported() {
        let filter = HiddenApiFilter {
            allow_unsupported: true,
            max_target: None,
        };

        // unsupported should now be allowed
        assert!(is_api_allowed(&["unsupported"], &filter));

        // public-api still allowed
        assert!(is_api_allowed(&["public-api"], &filter));

        // blocked still not allowed
        assert!(!is_api_allowed(&["blocked"], &filter));
    }

    #[test]
    fn test_is_api_allowed_with_max_target() {
        let filter = HiddenApiFilter {
            allow_unsupported: false,
            max_target: Some("p".to_string()),
        };

        // max-target-p should be allowed (equal to our max)
        assert!(is_api_allowed(&["max-target-p"], &filter));

        // max-target-q should be allowed (greater than our max)
        assert!(is_api_allowed(&["max-target-q"], &filter));

        // max-target-o should not be allowed (less than our max)
        assert!(!is_api_allowed(&["max-target-o"], &filter));

        // max-target-n should not be allowed (less than our max)
        assert!(!is_api_allowed(&["max-target-n"], &filter));

        // public-api still allowed
        assert!(is_api_allowed(&["public-api"], &filter));
    }

    #[test]
    fn test_compare_target_levels() {
        assert_eq!(compare_target_levels("o", "o"), 0);
        assert_eq!(compare_target_levels("o", "p"), -1);
        assert_eq!(compare_target_levels("p", "o"), 1);
        assert_eq!(compare_target_levels("n", "q"), -1);
        assert_eq!(compare_target_levels("u", "p"), 1);
    }
}
