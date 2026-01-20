//! Code generator for Rust bindings

use crate::error::Result;
use crate::parser_types::{ClassInfo, MethodInfo};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

/// Event emitted during module traversal
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleEvent {
    /// Beginning of a module
    BeginModule,
    /// End of a module
    EndModule,
}

/// A binding for a single Java type within a module
#[derive(Debug, Clone)]
pub struct ModuleBinding {
    /// The Java package name (e.g., "com.example.stuff")
    pub package: String,
    /// The Rust module path (e.g., ["com", "example", "stuff"])
    pub module_path: Vec<String>,
    /// Fully-qualified use statements (canonical paths, one item per use)
    pub use_statements: Vec<String>,
    /// The generated binding code for a single Java type
    pub binding_code: String,
    /// The Rust type name (e.g., "MyClass")
    pub rust_type_name: String,
    /// The Rust API type name (e.g., "MyClassAPI")
    pub rust_api_type_name: String,
    /// The Java type name (e.g., "MyClass" or "MyClass$Inner")
    pub java_type_name: String,
}

/// Builtin JNI type information
#[allow(dead_code)]
struct BuiltinType {
    /// Simple Rust type name (e.g., "JString")
    rust_name: &'static str,
    /// Java fully-qualified type name (e.g., "java.lang.String")
    java_name: &'static str,
    /// Qualified Rust path relative to jni:: (e.g., "objects::JString")
    rust_path: &'static str,
    /// Whether this is a core type that cannot be remapped
    is_core: bool,
}

/// Get the list of builtin JNI types from the jni crate
fn builtin_jni_types() -> &'static [BuiltinType] {
    &[
        BuiltinType {
            rust_name: "JByteBuffer",
            java_name: "java.nio.ByteBuffer",
            rust_path: "objects::JByteBuffer",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JClassLoader",
            java_name: "java.lang.ClassLoader",
            rust_path: "objects::JClassLoader",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JClass",
            java_name: "java.lang.Class",
            rust_path: "objects::JClass",
            is_core: true,
        },
        BuiltinType {
            rust_name: "JCollection",
            java_name: "java.util.Collection",
            rust_path: "objects::JCollection",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JIterator",
            java_name: "java.util.Iterator",
            rust_path: "objects::JIterator",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JList",
            java_name: "java.util.List",
            rust_path: "objects::JList",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JMap",
            java_name: "java.util.Map",
            rust_path: "objects::JMap",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JMapEntry",
            java_name: "java.util.Map$Entry",
            rust_path: "objects::JMapEntry",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JObject",
            java_name: "java.lang.Object",
            rust_path: "objects::JObject",
            is_core: true,
        },
        BuiltinType {
            rust_name: "JSet",
            java_name: "java.util.Set",
            rust_path: "objects::JSet",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JStackTraceElement",
            java_name: "java.lang.StackTraceElement",
            rust_path: "objects::JStackTraceElement",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JString",
            java_name: "java.lang.String",
            rust_path: "objects::JString",
            is_core: true,
        },
        BuiltinType {
            rust_name: "JThread",
            java_name: "java.lang.Thread",
            rust_path: "objects::JThread",
            is_core: false,
        },
        BuiltinType {
            rust_name: "JThrowable",
            java_name: "java.lang.Throwable",
            rust_path: "objects::JThrowable",
            is_core: true,
        },
    ]
}

/// Derive a Rust type name from a Java class simple name, applying transformations
/// to handle inner classes and applying optional prefix.
///
/// - Strips '$' characters used for inner classes (e.g., "ColorSpace$Rgb" -> "ColorSpaceRgb")
/// - Applies optional name prefix
fn derive_rust_type_name(
    class_name: &str,
    rust_type_override: Option<&str>,
    name_prefix: Option<&str>,
) -> String {
    // Extract the class portion from class_name (strip package)
    // For "android/os/Build$Partition" -> "Build$Partition"
    let class_part = class_name.split('/').next_back().unwrap_or(class_name);

    let base_name = rust_type_override.unwrap_or(class_part);

    // Strip '$' characters used for inner classes (e.g., "Build$Partition" -> "BuildPartition")
    let base_name_cleaned = base_name.replace('$', "");

    // Apply name prefix if specified
    if let Some(prefix) = name_prefix {
        format!("{}{}", prefix, base_name_cleaned)
    } else {
        base_name_cleaned
    }
}

/// Options for binding generation
#[derive(Debug, Clone)]
pub struct BindgenOptions {
    /// Whether to make the generated type public
    pub public_type: bool,
    /// Name override for the Rust type (if None, uses the Java simple class name)
    pub rust_type_name: Option<String>,
    /// Prefix to add to all generated Rust type names
    pub name_prefix: Option<String>,
    /// Whether to generate native method interfaces (default: true)
    pub generate_native_interfaces: bool,
    /// Whether to generate jni_init methods for modules (default: true)
    pub generate_jni_init: bool,
    /// Root module path for generated bindings (e.g., "crate::bindings::sdk")
    pub root_path: String,
    /// DEX signatures of methods/fields to skip during generation
    pub skip_signatures: Vec<String>,
    /// Map of DEX signatures to override Rust names
    pub name_overrides: HashMap<String, String>,
}

impl Default for BindgenOptions {
    fn default() -> Self {
        Self {
            public_type: true,
            rust_type_name: None,
            name_prefix: None,
            generate_native_interfaces: true,
            generate_jni_init: true,
            root_path: String::from("crate"),
            skip_signatures: Vec::new(),
            name_overrides: HashMap::new(),
        }
    }
}

/// Type mapping from Java class names to Rust type names
#[derive(Debug, Clone)]
pub struct TypeMap {
    /// Maps Java fully-qualified class name (e.g., "android.os.Bundle") to Rust type name
    map: HashMap<String, String>,
}

impl TypeMap {
    /// Create a new TypeMap from a collection of ClassInfo
    pub(crate) fn from_classes<'a>(
        classes: impl IntoIterator<Item = &'a ClassInfo>,
        options: &BindgenOptions,
    ) -> Self {
        let mut map = HashMap::new();

        for class_info in classes {
            // Build the Java type name using class_name which has the full binary name with $ for inner classes
            let java_type = if class_info.package.is_empty() {
                format!(".{}", class_info.class_name.replace('/', "."))
            } else {
                class_info.class_name.replace('/', ".")
            };

            // Get the simple Rust type name
            let rust_name = derive_rust_type_name(
                &class_info.class_name,
                options.rust_type_name.as_deref(),
                options.name_prefix.as_deref(),
            );

            // Build fully-qualified Rust path
            // e.g., "crate::bindings::sdk::com::example::MyClass"
            let mut rust_path = options.root_path.clone();
            for segment in &class_info.package {
                rust_path.push_str("::");
                rust_path.push_str(segment);
            }
            rust_path.push_str("::");
            rust_path.push_str(&rust_name);

            map.insert(java_type, rust_path);
        }

        TypeMap { map }
    }

    /// Add extra type mappings (Java type -> Rust type)
    /// Silently ignores attempts to remap core types (java.lang.String, java.lang.Object, etc.)
    pub fn merge(&mut self, extra_mappings: Vec<(String, String)>) {
        for (rust_type, java_type) in extra_mappings {
            // Check if this is a core builtin type that cannot be remapped
            let is_core = builtin_jni_types()
                .iter()
                .any(|b| b.java_name == java_type && b.is_core);

            if is_core {
                log::warn!(
                    "Ignoring attempt to remap core type '{}' to '{}'",
                    java_type,
                    rust_type
                );
                continue;
            }

            self.map.insert(java_type, rust_type);
        }
    }

    /// Look up the Rust type name for a Java class
    pub fn get_rust_type(&self, java_type: &str) -> Option<&str> {
        self.map.get(java_type).map(|s| s.as_str())
    }

    /// Get all mappings (for generating type_map blocks)
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.map.iter()
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Create a new empty TypeMap
    pub fn new() -> Self {
        TypeMap {
            map: HashMap::new(),
        }
    }

    /// Insert a single type mapping
    pub fn insert(&mut self, java_type: String, rust_type: String) {
        // Check if this is a core builtin type that cannot be remapped
        let is_core = builtin_jni_types()
            .iter()
            .any(|b| b.java_name == java_type && b.is_core);

        if !is_core {
            self.map.insert(java_type, rust_type);
        }
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

/// Convert Javadoc comment to Rustdoc format
///
/// This takes a Javadoc comment string and converts it to Rustdoc format.
/// - Converts `@param` tags to parameter documentation
/// - Converts `@return` tags to Returns section
/// - Strips HTML tags
/// - Formats as `///` style comments
fn format_javadoc_as_rustdoc(javadoc: &str) -> String {
    let mut output = String::new();
    let mut in_param_section = false;
    let mut in_return_section = false;

    for line in javadoc.lines() {
        let trimmed = line.trim();

        // Skip empty lines in special sections
        if trimmed.is_empty() {
            if !in_param_section && !in_return_section {
                output.push_str("///\n");
            }
            continue;
        }

        // Handle @param tags
        if trimmed.starts_with("@param") {
            if !in_param_section {
                output.push_str("///\n");
                output.push_str("/// # Parameters\n");
                in_param_section = true;
            }
            // Extract parameter name and description
            if let Some(rest) = trimmed.strip_prefix("@param") {
                let rest = rest.trim();
                if let Some((param_name, desc)) = rest.split_once(char::is_whitespace) {
                    output.push_str(&format!(
                        "/// * `{}` - {}\n",
                        param_name.trim(),
                        desc.trim()
                    ));
                }
            }
            continue;
        }

        // Handle @return tags
        if trimmed.starts_with("@return") {
            in_param_section = false;
            in_return_section = true;
            if let Some(desc) = trimmed.strip_prefix("@return") {
                output.push_str("///\n");
                output.push_str("/// # Returns\n");
                output.push_str(&format!("/// {}\n", desc.trim()));
            }
            continue;
        }

        // Handle @throws/@exception tags
        if trimmed.starts_with("@throws") || trimmed.starts_with("@exception") {
            in_param_section = false;
            in_return_section = false;
            continue; // Skip for now, could add Errors section
        }

        // Handle other @ tags by skipping them
        if trimmed.starts_with('@') {
            in_param_section = false;
            in_return_section = false;
            continue;
        }

        // Regular documentation line
        in_param_section = false;
        in_return_section = false;

        // Basic HTML tag stripping (simple version)
        let cleaned = trimmed
            .replace("<p>", "")
            .replace("</p>", "")
            .replace("<br>", "")
            .replace("<br/>", "")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&");

        output.push_str(&format!("/// {}\n", cleaned));
    }

    output
}

/// Generate Rust bindings code from ClassInfo with type mappings
pub fn generate_with_type_map(
    class_info: &ClassInfo,
    options: &BindgenOptions,
    type_map: &TypeMap,
) -> Result<ModuleBinding> {
    // Track which types from type_map are actually used
    let mut used_types = HashSet::new();

    // Extract package name and module path from class_info
    let module_path = class_info.package.clone();
    let package = module_path.join(".");

    // Collect use statements - always need bind_java_type
    let use_statements = vec!["use jni::bind_java_type;".to_string()];

    // Determine the Rust type name
    // Priority: class_info.rust_name_override > options.rust_type_name > derived name
    let rust_type_override = class_info
        .rust_name_override
        .as_deref()
        .or(options.rust_type_name.as_deref());

    let rust_name = derive_rust_type_name(
        &class_info.class_name,
        rust_type_override,
        options.name_prefix.as_deref(),
    );

    // Use class_name which has the full binary name with $ for inner classes
    // Just convert / to . for the Java type name
    let java_type = if class_info.package.is_empty() {
        // Default package - add leading dot
        format!(".{}", class_info.class_name.replace('/', "."))
    } else {
        class_info.class_name.replace('/', ".")
    };

    // Buffer for type header and documentation
    let mut header_buffer = String::new();

    // Generate the type binding header
    let vis = if options.public_type { "pub " } else { "" };

    // Add class documentation if available (must be right before the type name)
    if let Some(doc) = &class_info.documentation {
        if !doc.is_empty() {
            for line in format_javadoc_as_rustdoc(doc).lines() {
                header_buffer.push_str("    ");
                header_buffer.push_str(line);
                header_buffer.push('\n');
            }
        }
    }

    header_buffer.push_str(&format!("    {}{} => \"{}\",\n", vis, rust_name, java_type));

    // Buffer for constructors, methods, fields, etc.
    let mut body_buffer = String::new();

    // Generate constructors block if any
    if !class_info.constructors.is_empty() {
        // Filter out constructors based on skip_signatures
        let mut constructors_to_emit = class_info.constructors.clone();
        if !options.skip_signatures.is_empty() {
            constructors_to_emit.retain(|ctor| {
                let dex_sig = generate_dex_signature(&class_info.class_name, ctor);
                !options.skip_signatures.contains(&dex_sig)
            });
        }

        if !constructors_to_emit.is_empty() {
            body_buffer.push_str("    constructors {\n");

            // Sort constructors by argument count, then by JNI signature for deterministic output
            let mut sorted_constructors = constructors_to_emit;
            sorted_constructors.sort_by(|a, b| {
                let a_arg_count = a.signature.arguments.len();
                let b_arg_count = b.signature.arguments.len();

                // First, sort by argument count
                match a_arg_count.cmp(&b_arg_count) {
                    std::cmp::Ordering::Equal => {
                        // If argument counts are equal, sort by JNI signature
                        let a_sig = generate_jni_signature_for_sorting(a);
                        let b_sig = generate_jni_signature_for_sorting(b);
                        a_sig.cmp(&b_sig)
                    }
                    other => other,
                }
            });

            let mut used_names = std::collections::HashSet::new();
            let ctor_refs: Vec<&MethodInfo> = sorted_constructors.iter().collect();
            let ctor_names = generate_unique_overload_names("new", &ctor_refs, &mut used_names);

            for (idx, ctor) in sorted_constructors.iter().enumerate() {
                // Check for name override with priority:
                // 1. rust_name_override from annotation
                // 2. options.name_overrides (manual overrides)
                // 3. generated name
                let dex_sig = generate_dex_signature(&class_info.class_name, ctor);

                let ctor_name = if let Some(override_name) = &ctor.rust_name_override {
                    override_name.as_str()
                } else if let Some(override_name) = options.name_overrides.get(&dex_sig) {
                    override_name
                } else {
                    &ctor_names[idx]
                };

                let sig = generate_method_signature_with_deps(ctor, type_map, &mut used_types)?;

                // Add constructor documentation if available
                if let Some(doc) = &ctor.documentation {
                    if !doc.is_empty() {
                        for line in format_javadoc_as_rustdoc(doc).lines() {
                            body_buffer.push_str("        ");
                            body_buffer.push_str(line);
                            body_buffer.push('\n');
                        }
                    }
                }

                // Add #[deprecated] attribute if needed
                if ctor.is_deprecated {
                    body_buffer.push_str("        #[deprecated]\n");
                }

                body_buffer.push_str(&format!("        fn {}{}", ctor_name, sig));
                body_buffer.push_str(",\n");
            }
            body_buffer.push_str("    },\n");
        }
    }

    // Generate fields block if any
    if !class_info.fields.is_empty() {
        // Filter out fields based on skip_signatures
        let mut fields_to_emit = class_info.fields.clone();
        if !options.skip_signatures.is_empty() {
            fields_to_emit.retain(|field| {
                let dex_sig = generate_field_dex_signature(&class_info.class_name, field);
                !options.skip_signatures.contains(&dex_sig)
            });
        }

        if !fields_to_emit.is_empty() {
            body_buffer.push_str("    fields {\n");

            // Track used Rust names to detect collisions
            let mut field_names: HashSet<(String, String)> = HashSet::new();

            for field in &fields_to_emit {
                // Check if there's a name override for this field
                let dex_sig = generate_field_dex_signature(&class_info.class_name, field);
                let overridden_name = options.name_overrides.get(&dex_sig);

                let (mut rust_name, mut is_reversible) =
                    if let Some(override_name) = field.rust_name_override.as_ref() {
                        // Priority 1: Use @RustName annotation from source
                        (override_name.clone(), false)
                    } else if let Some(override_name) = overridden_name {
                        // Priority 2: Use the CLI/config overridden name
                        (override_name.clone(), false)
                    } else {
                        // Priority 3: Derive from Java name
                        java_name_to_rust(&field.name)
                    };

                // Check for name collisions and resolve them
                let (final_rust_name, had_collision, conflicting_java_name) =
                    resolve_name_collision(rust_name.clone(), &field.name, &mut field_names);

                if had_collision {
                    log::warn!(
                        "Field name collision detected in class '{}':\n  \
                         Java field '{}' maps to Rust name '{}' which conflicts with field '{}'.\n  \
                         Using '{}' instead.\n  \
                         To customize this name, use:\n  \
                         - skip_signatures option with DEX signature: {}\n  \
                         - name_overrides option: map[{:?}] = \"your_name\"\n  \
                         - CLI: --skip '{}' or --name '{}=your_name'",
                        class_info.class_name,
                        field.name,
                        rust_name,
                        conflicting_java_name.unwrap_or_default(),
                        final_rust_name,
                        dex_sig,
                        dex_sig,
                        dex_sig,
                        dex_sig
                    );
                    rust_name = final_rust_name;
                    is_reversible = false; // Collisions are never reversible
                } else {
                    rust_name = final_rust_name;
                }

                let rust_type =
                    resolve_type_with_deps(&field.type_info, type_map, &mut used_types)?;
                let needs_explicit_name = !is_reversible;

                // Add field documentation if available
                if let Some(doc) = &field.documentation {
                    if !doc.is_empty() {
                        for line in format_javadoc_as_rustdoc(doc).lines() {
                            body_buffer.push_str("        ");
                            body_buffer.push_str(line);
                            body_buffer.push('\n');
                        }
                    }
                }

                // Add #[deprecated] attribute if needed
                if field.is_deprecated {
                    body_buffer.push_str("        #[deprecated]\n");
                }

                let modifier = if field.is_static { "static " } else { "" };

                let use_props = needs_explicit_name || field.is_final;

                if use_props {
                    // Use property syntax when name isn't reversible or field is final
                    body_buffer.push_str(&format!("        {}{} {{\n", modifier, rust_name));
                    if needs_explicit_name {
                        body_buffer.push_str(&format!("            name = \"{}\",\n", field.name));
                    }
                    body_buffer.push_str(&format!("            sig = {},\n", rust_type));
                    // By explicitly specifying the getter name, we avoid generating a setter for final fields
                    if field.is_final {
                        body_buffer.push_str(&format!("            get = {},\n", rust_name));
                    }

                    body_buffer.push_str("        },\n");
                } else {
                    // Use shorthand syntax when name is reversible
                    body_buffer
                        .push_str(&format!("        {}{}: {}", modifier, rust_name, rust_type));
                    body_buffer.push_str(",\n");
                }
            }

            body_buffer.push_str("    },\n");
        }
    }

    // Generate methods block if any
    // When generate_native_interfaces is false, public native methods also go here
    let mut methods_to_emit = class_info.methods.clone();

    if !options.generate_native_interfaces {
        // Add public native methods to methods block when interfaces are disabled
        for native_method in &class_info.native_methods {
            if native_method.is_public {
                methods_to_emit.push(native_method.clone());
            }
        }
    }

    // Filter out methods based on skip_signatures
    if !options.skip_signatures.is_empty() {
        methods_to_emit.retain(|method| {
            let dex_sig = generate_dex_signature(&class_info.class_name, method);
            !options.skip_signatures.contains(&dex_sig)
        });
    }

    if !methods_to_emit.is_empty() {
        body_buffer.push_str("    methods {\n");

        let method_groups = group_methods_by_name(&methods_to_emit);

        // Track used Rust names across all method groups to detect collisions
        let mut all_method_names: HashSet<(String, String)> = HashSet::new();

        for (java_name, methods) in method_groups {
            let is_overloaded = methods.len() > 1;
            let (rust_base_name, base_is_reversible) = java_name_to_rust(java_name);

            let mut used_names = std::collections::HashSet::new();
            let method_names = if is_overloaded {
                generate_unique_overload_names(&rust_base_name, &methods, &mut used_names)
            } else {
                vec![rust_base_name.clone()]
            };

            for (idx, method) in methods.iter().enumerate() {
                // Add method documentation if available
                if let Some(doc) = &method.documentation {
                    if !doc.is_empty() {
                        for line in format_javadoc_as_rustdoc(doc).lines() {
                            body_buffer.push_str("        ");
                            body_buffer.push_str(line);
                            body_buffer.push('\n');
                        }
                    }
                }

                // Add #[deprecated] attribute if needed
                if method.is_deprecated {
                    body_buffer.push_str("        #[deprecated]\n");
                }

                let modifier = if method.is_static { "static " } else { "" };
                let sig = generate_method_signature_with_deps(method, type_map, &mut used_types)?;

                // Check if there's a name override for this method
                let dex_sig = generate_dex_signature(&class_info.class_name, method);
                let overridden_name = options.name_overrides.get(&dex_sig);

                // Determine rust name and reversibility
                let (mut rust_name, mut is_reversible) =
                    if let Some(override_name) = method.rust_name_override.as_ref() {
                        // Priority 1: Use @RustName annotation from source
                        (override_name.clone(), false)
                    } else if let Some(override_name) = overridden_name {
                        // Priority 2: Use the CLI/config overridden name
                        (override_name.clone(), false)
                    } else if is_overloaded {
                        // Priority 3: Overloaded methods use generated unique names
                        (method_names[idx].clone(), false)
                    } else {
                        // Priority 4: Derive from Java name
                        (rust_base_name.clone(), base_is_reversible)
                    };

                // Check for name collisions across different Java method names
                let (final_rust_name, had_collision, conflicting_java_name) =
                    resolve_name_collision(rust_name.clone(), java_name, &mut all_method_names);

                if had_collision {
                    log::warn!(
                        "Method name collision detected in class '{}':\n  \
                         Java method '{}' maps to Rust name '{}' which conflicts with method '{}'.\n  \
                         Using '{}' instead.\n  \
                         To customize this name, use:\n  \
                         - skip_signatures option with DEX signature: {}\n  \
                         - name_overrides option: map[{:?}] = \"your_name\"\n  \
                         - CLI: --skip '{}' or --name '{}=your_name'",
                        class_info.class_name,
                        java_name,
                        rust_name,
                        conflicting_java_name.unwrap_or_default(),
                        final_rust_name,
                        dex_sig,
                        dex_sig,
                        dex_sig,
                        dex_sig
                    );
                    rust_name = final_rust_name;
                    is_reversible = false; // Collisions are never reversible
                } else {
                    rust_name = final_rust_name;
                }

                if is_reversible {
                    // Use shorthand syntax when name is reversible
                    body_buffer.push_str(&format!("        {}fn {}{}", modifier, rust_name, sig));
                    body_buffer.push_str(",\n");
                } else {
                    // Use property syntax when name isn't reversible (including overloads)
                    body_buffer.push_str(&format!("        {}fn {} {{\n", modifier, rust_name));
                    body_buffer.push_str(&format!("            name = \"{}\",\n", java_name));
                    body_buffer.push_str(&format!("            sig = {},\n", sig));
                    body_buffer.push_str("        },\n");
                }
            }
        }

        body_buffer.push_str("    },\n");
    }

    // Generate native methods block if any and if native interfaces are enabled
    // When generate_native_interfaces is true:
    // - Public native methods go to native_methods block with "pub" visibility
    // - Private native methods go to native_methods block with no visibility qualifier
    //   (omitting visibility means no call method is generated, only trait implementation)
    if options.generate_native_interfaces && !class_info.native_methods.is_empty() {
        // Filter out native methods based on skip_signatures
        let mut native_methods_to_emit = class_info.native_methods.clone();
        if !options.skip_signatures.is_empty() {
            native_methods_to_emit.retain(|method| {
                let dex_sig = generate_dex_signature(&class_info.class_name, method);
                !options.skip_signatures.contains(&dex_sig)
            });
        }

        if !native_methods_to_emit.is_empty() {
            body_buffer.push_str("    native_methods {\n");

            let method_groups = group_methods_by_name(&native_methods_to_emit);

            // Track used Rust names across all native method groups to detect collisions
            let mut all_native_method_names: HashSet<(String, String)> = HashSet::new();

            for (java_name, methods) in method_groups {
                let is_overloaded = methods.len() > 1;
                let (rust_base_name, base_is_reversible) = java_name_to_rust(java_name);

                let mut used_names = std::collections::HashSet::new();
                let method_names = if is_overloaded {
                    generate_unique_overload_names(&rust_base_name, &methods, &mut used_names)
                } else {
                    vec![rust_base_name.clone()]
                };

                for (idx, method) in methods.iter().enumerate() {
                    // Add native method documentation if available
                    if let Some(doc) = &method.documentation {
                        if !doc.is_empty() {
                            for line in format_javadoc_as_rustdoc(doc).lines() {
                                body_buffer.push_str("        ");
                                body_buffer.push_str(line);
                                body_buffer.push('\n');
                            }
                        }
                    }

                    // Add #[deprecated] attribute if needed
                    if method.is_deprecated {
                        body_buffer.push_str("        #[deprecated]\n");
                    }

                    let modifier = if method.is_static { "static " } else { "" };
                    let visibility = if method.is_public { "pub " } else { "" };
                    let sig =
                        generate_method_signature_with_deps(method, type_map, &mut used_types)?;

                    // Check if there's a name override for this method
                    let dex_sig = generate_dex_signature(&class_info.class_name, method);
                    let overridden_name = options.name_overrides.get(&dex_sig);

                    // Determine rust name and reversibility
                    let (mut rust_name, mut is_reversible) =
                        if let Some(override_name) = method.rust_name_override.as_ref() {
                            // Priority 1: Use @RustName annotation from source
                            (override_name.clone(), false)
                        } else if let Some(override_name) = overridden_name {
                            // Priority 2: Use the CLI/config overridden name
                            (override_name.clone(), false)
                        } else if is_overloaded {
                            // Priority 3: Overloaded methods use generated unique names
                            (method_names[idx].clone(), false)
                        } else {
                            // Priority 4: Derive from Java name
                            (rust_base_name.clone(), base_is_reversible)
                        };

                    // Check for name collisions across different Java method names
                    let (final_rust_name, had_collision, conflicting_java_name) =
                        resolve_name_collision(
                            rust_name.clone(),
                            java_name,
                            &mut all_native_method_names,
                        );

                    if had_collision {
                        log::warn!(
                            "Native method name collision detected in class '{}':\n  \
                             Java method '{}' maps to Rust name '{}' which conflicts with method '{}'.\n  \
                             Using '{}' instead.\n  \
                             To customize this name, use:\n  \
                             - skip_signatures option with DEX signature: {}\n  \
                             - name_overrides option: map[{:?}] = \"your_name\"\n  \
                             - CLI: --skip '{}' or --name '{}=your_name'",
                            class_info.class_name,
                            java_name,
                            rust_name,
                            conflicting_java_name.unwrap_or_default(),
                            final_rust_name,
                            dex_sig,
                            dex_sig,
                            dex_sig,
                            dex_sig
                        );
                        rust_name = final_rust_name;
                        is_reversible = false; // Collisions are never reversible
                    } else {
                        rust_name = final_rust_name;
                    }

                    if is_reversible {
                        // Use shorthand syntax when name is reversible
                        body_buffer.push_str(&format!(
                            "        {}{}fn {}{}",
                            visibility, modifier, rust_name, sig
                        ));
                        body_buffer.push_str(",\n");
                    } else {
                        // Use property syntax when name isn't reversible (including overloads)
                        body_buffer.push_str(&format!(
                            "        {}{}fn {} {{\n",
                            visibility, modifier, rust_name
                        ));
                        body_buffer.push_str(&format!("            name = \"{}\",\n", java_name));
                        body_buffer.push_str(&format!("            sig = {},\n", sig));
                        body_buffer.push_str("        },\n");
                    }
                }
            }

            body_buffer.push_str("    },\n");
        }
    }

    // Generate is_instance_of block if we have any types (only those with Rust bindings)
    let instance_of_types: Vec<_> = class_info
        .instance_of
        .iter()
        .filter(|info| type_map.get_rust_type(&info.java_type).is_some())
        .collect();

    if !instance_of_types.is_empty() {
        body_buffer.push_str("    is_instance_of = {\n");

        for info in &instance_of_types {
            let rust_type = type_map.get_rust_type(&info.java_type).unwrap();
            // Track this type as used
            used_types.insert(info.java_type.clone());

            if let Some(stem) = &info.stem {
                body_buffer.push_str(&format!("        {}: {},\n", stem, rust_type));
            } else {
                body_buffer.push_str(&format!("        {},\n", rust_type));
            }
        }

        body_buffer.push_str("    },\n");
    }

    // Now assemble the final output
    // Start with bind_java_type! macro invocation
    let mut output = String::new();
    output.push_str("bind_java_type! {\n");

    // Add the header (type declaration and docs)
    output.push_str(&header_buffer);

    // Generate type_map block with only the types that were actually used
    // Note: we exclude the self type (java_type) because bind_java_type adds it automatically
    if !used_types.is_empty() {
        // Filter the type_map to only include used types, excluding self
        let mut filtered_mappings: Vec<_> = type_map
            .iter()
            .filter(|(java, _)| used_types.contains(*java) && *java != &java_type)
            .collect();

        if !filtered_mappings.is_empty() {
            filtered_mappings.sort_by_key(|(java, _)| java.as_str());

            output.push_str("    type_map = {\n");
            for (java_type, rust_type) in filtered_mappings {
                output.push_str(&format!("        {} => \"{}\",\n", rust_type, java_type));
            }
            output.push_str("    },\n");
        }
    }

    // Add the body (constructors, methods, fields, etc.)
    output.push_str(&body_buffer);

    // Close the bind_java_type! macro
    output.push_str("}\n");

    let rust_api_type_name = format!("{}API", rust_name);

    Ok(ModuleBinding {
        package,
        module_path,
        use_statements,
        binding_code: output,
        rust_type_name: rust_name,
        rust_api_type_name,
        java_type_name: class_info
            .class_name
            .split('/')
            .next_back()
            .unwrap_or(&class_info.class_name)
            .to_string(),
    })
}

/// Generate a JNI-style signature string for a method for sorting purposes
/// Format: (arg1_type,arg2_type,...)return_type
fn generate_jni_signature_for_sorting(method: &MethodInfo) -> String {
    let params: Vec<String> = method
        .signature
        .arguments
        .iter()
        .map(|arg| format_type_for_sorting(&arg.type_info))
        .collect();

    format!(
        "({}){}",
        params.join(","),
        format_type_for_sorting(&method.signature.return_type)
    )
}

/// Format a type for sorting purposes
fn format_type_for_sorting(type_info: &crate::parser_types::TypeInfo) -> String {
    let base = type_info.name.clone();
    if type_info.array_dimensions > 0 {
        format!("{}{}", base, "[]".repeat(type_info.array_dimensions))
    } else {
        base
    }
}

/// Convert a TypeInfo to DEX signature format
fn type_to_dex_signature(type_info: &crate::parser_types::TypeInfo) -> String {
    let mut sig = String::new();

    // Add array dimensions
    for _ in 0..type_info.array_dimensions {
        sig.push('[');
    }

    // Add the base type
    if type_info.is_primitive {
        // Primitive types use single character codes
        let code = match type_info.name.as_str() {
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
        };
        sig.push_str(code);
    } else {
        // Object types use L<class_path>;
        sig.push('L');
        sig.push_str(&type_info.name.replace('.', "/"));
        sig.push(';');
    }

    sig
}

/// Generate a DEX signature for a method
/// Format: L<class_path>;-><method_name>(<params>)<return_type>
fn generate_dex_signature(class_name: &str, method: &MethodInfo) -> String {
    let mut sig = String::new();

    // Add class name in DEX format
    sig.push('L');
    sig.push_str(&class_name.replace('.', "/"));
    sig.push_str(";->");

    // Add method name
    sig.push_str(&method.name);

    // Add parameter signatures
    sig.push('(');
    for arg in &method.signature.arguments {
        sig.push_str(&type_to_dex_signature(&arg.type_info));
    }
    sig.push(')');

    // Add return type signature
    sig.push_str(&type_to_dex_signature(&method.signature.return_type));

    sig
}

/// Generate a DEX signature for a field
/// Format: L<class_path>;-><field_name>:<type>
fn generate_field_dex_signature(
    class_name: &str,
    field: &crate::parser_types::FieldInfo,
) -> String {
    let mut sig = String::new();

    // Add class name in DEX format
    sig.push('L');
    sig.push_str(&class_name.replace('.', "/"));
    sig.push_str(";->");

    // Add field name
    sig.push_str(&field.name);

    // Add field type
    sig.push(':');
    sig.push_str(&type_to_dex_signature(&field.type_info));

    sig
}

/// Group methods by their Java name to detect overloads
/// Returns a Vec of (java_name, Vec<&MethodInfo>)
/// Within each group, methods are sorted by:
/// 1. Number of arguments (ascending)
/// 2. JNI signature (lexicographically)
fn group_methods_by_name(methods: &[MethodInfo]) -> Vec<(&str, Vec<&MethodInfo>)> {
    use std::collections::HashMap;

    let mut groups: HashMap<&str, Vec<&MethodInfo>> = HashMap::new();
    for method in methods {
        groups.entry(method.name.as_str()).or_default().push(method);
    }

    let mut result: Vec<_> = groups.into_iter().collect();
    // Sort by name for deterministic output
    result.sort_by_key(|(name, _)| *name);

    // Sort methods within each group by argument count, then by JNI signature
    for (_, methods) in &mut result {
        methods.sort_by(|a, b| {
            let a_arg_count = a.signature.arguments.len();
            let b_arg_count = b.signature.arguments.len();

            // First, sort by argument count
            match a_arg_count.cmp(&b_arg_count) {
                std::cmp::Ordering::Equal => {
                    // If argument counts are equal, sort by JNI signature
                    let a_sig = generate_jni_signature_for_sorting(a);
                    let b_sig = generate_jni_signature_for_sorting(b);
                    a_sig.cmp(&b_sig)
                }
                other => other,
            }
        });
    }

    result
}

/// Detect and resolve Rust name collisions by appending underscores
///
/// Returns (final_name, had_collision, java_name_for_warning)
/// where had_collision indicates if a collision was detected and resolved
fn resolve_name_collision(
    rust_name: String,
    java_name: &str,
    used_names: &mut HashSet<(String, String)>,
) -> (String, bool, Option<String>) {
    let mut candidate = rust_name.clone();
    let mut had_collision = false;
    let mut conflicting_java_name = None;

    // Check if this rust_name is already used by a different Java name
    loop {
        // Find if there's an existing entry with this rust name
        let existing = used_names.iter().find(|(rust, _java)| rust == &candidate);

        if let Some((_existing_rust, existing_java)) = existing {
            // If the Java names are different, we have a collision
            if existing_java != java_name {
                had_collision = true;
                if conflicting_java_name.is_none() {
                    conflicting_java_name = Some(existing_java.clone());
                }
                // Append underscore and try again
                candidate.push('_');
            } else {
                // Same Java name - this is the same entry, no collision
                break;
            }
        } else {
            // No existing entry with this rust name - we're good
            break;
        }
    }

    used_names.insert((candidate.clone(), java_name.to_string()));
    (candidate, had_collision, conflicting_java_name)
}

/// Convert a Java type name to a snake_case suffix for method naming
fn java_type_to_snake_case_suffix(type_info: &crate::parser_types::TypeInfo) -> String {
    // Handle primitive types
    let base_name = if type_info.is_primitive {
        match type_info.name.as_str() {
            "boolean" => "bool",
            _ => &type_info.name,
        }
        .to_string()
    } else {
        // For object types, extract the simple class name and convert to snake_case
        // e.g., "java.lang.String" -> "string"
        // e.g., "com.example.MyType$Inner" -> "my_type_inner"
        let simple_name = type_info
            .name
            .split('.')
            .next_back()
            .unwrap_or(&type_info.name);

        // Remove '$' characters used for inner classes
        let without_dollar = simple_name.replace('$', "");

        // Convert to snake_case
        java_name_to_rust(&without_dollar).0
    };

    // Add array dimension suffix
    if type_info.array_dimensions > 0 {
        format!("{}_{}d", base_name, type_info.array_dimensions)
    } else {
        base_name
    }
}

/// Generate a unique name for an overloaded method or constructor
///
/// This function generates unique names by:
/// 1. Grouping overloads by arity (number of arguments)
/// 2. For each arity group, determining which argument positions vary across overloads
/// 3. Adding "<N>" or "_args<N>" suffix if arity differs from number of varying positions
/// 4. Adding snake_case type suffixes for varying argument positions
///
/// # Arguments
/// * `base_name` - The base name (e.g., "new" for constructors, "get_value" for methods)
/// * `methods` - All methods in this overload group (already sorted)
/// * `used_names` - Set of names already used
///
/// Returns a Vec of unique names, one for each method
fn generate_unique_overload_names(
    base_name: &str,
    methods: &[&MethodInfo],
    used_names: &mut std::collections::HashSet<String>,
) -> Vec<String> {
    let mut result = Vec::new();

    // Group methods by arity
    let mut arity_groups: std::collections::BTreeMap<usize, Vec<(usize, &MethodInfo)>> =
        std::collections::BTreeMap::new();

    for (idx, method) in methods.iter().enumerate() {
        let arity = method.signature.arguments.len();
        arity_groups.entry(arity).or_default().push((idx, method));
    }

    // Process each arity group
    for (_arity, group) in arity_groups {
        let arity = group[0].1.signature.arguments.len();

        // Determine which argument positions vary within this arity group
        let mut varying_positions = Vec::new();
        for arg_idx in 0..arity {
            let mut types_at_position = std::collections::HashSet::new();
            for (_method_idx, method) in &group {
                if arg_idx < method.signature.arguments.len() {
                    let type_key = format!(
                        "{}:{}:{}",
                        method.signature.arguments[arg_idx].type_info.name,
                        method.signature.arguments[arg_idx].type_info.is_primitive,
                        method.signature.arguments[arg_idx]
                            .type_info
                            .array_dimensions
                    );
                    types_at_position.insert(type_key);
                }
            }
            if types_at_position.len() > 1 {
                varying_positions.push(arg_idx);
            }
        }

        // Check if this arity group is alone (single method in the group)
        let is_alone_in_group = group.len() == 1;

        // Generate names for each method in this arity group
        for (method_idx, method) in group {
            let mut candidate = base_name.to_string();

            // Only allow using base name if:
            // 1. It's not already used, AND
            // 2. This method is alone in its arity group (no other overloads with same arity)
            if is_alone_in_group && !used_names.contains(&candidate) {
                used_names.insert(candidate.clone());
                // Ensure we add empty strings for methods we haven't processed yet
                while result.len() <= method_idx {
                    result.push(String::new());
                }
                result[method_idx] = candidate;
                continue;
            }

            // For arity 1, always add the argument type suffix (no arity number)
            // This happens when arity 0 method exists with the base name
            if arity == 1 {
                let type_suffix =
                    java_type_to_snake_case_suffix(&method.signature.arguments[0].type_info);
                candidate.push('_');
                candidate.push_str(&type_suffix);
            } else {
                // Add arity suffix if needed (arity >= 2 and not all positions vary)
                if arity >= 2 && varying_positions.len() != arity {
                    if base_name.ends_with(|c: char| c.is_ascii_digit()) {
                        candidate.push_str(&format!("_args{}", arity));
                    } else {
                        candidate.push_str(&format!("{}", arity));
                    }
                }

                // Add type suffixes for varying positions
                for &pos in &varying_positions {
                    let type_suffix =
                        java_type_to_snake_case_suffix(&method.signature.arguments[pos].type_info);
                    candidate.push('_');
                    candidate.push_str(&type_suffix);
                }
            }

            // If still not unique, add numeric suffix
            if used_names.contains(&candidate) {
                let mut counter = 1;
                loop {
                    let numbered_name = format!("{}_{}", candidate, counter);
                    if !used_names.contains(&numbered_name) {
                        candidate = numbered_name;
                        break;
                    }
                    counter += 1;
                }
            }

            used_names.insert(candidate.clone());
            // Ensure we add empty strings for methods we haven't processed yet
            while result.len() <= method_idx {
                result.push(String::new());
            }
            result[method_idx] = candidate;
        }
    }

    result
}

/// Rust keywords that need to be escaped when used as identifiers
static RUST_KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    HashSet::from([
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
        "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
    ])
});

/// Sanitize parameter names that are Rust keywords by appending an underscore
fn sanitize_rust_keyword(name: &str) -> String {
    if RUST_KEYWORDS.contains(name) {
        format!("{}_", name)
    } else {
        name.to_string()
    }
}

/// Generate the method signature string for bind_java_type! macro, tracking type dependencies
fn generate_method_signature_with_deps(
    method: &MethodInfo,
    type_map: &TypeMap,
    used_types: &mut HashSet<String>,
) -> Result<String> {
    let mut sig = String::from("(");

    for (i, arg) in method.signature.arguments.iter().enumerate() {
        if i > 0 {
            sig.push_str(", ");
        }

        let arg_name = if let Some(arg_name) = arg.name.as_deref() {
            sanitize_rust_keyword(arg_name)
        } else {
            format!("arg{}", i)
        };
        sig.push_str(&format!("{}: ", arg_name));
        sig.push_str(&resolve_type_with_deps(
            &arg.type_info,
            type_map,
            used_types,
        )?);
    }

    sig.push(')');

    // Add return type if not void
    if method.signature.return_type.name != "void" {
        sig.push_str(" -> ");
        sig.push_str(&resolve_type_with_deps(
            &method.signature.return_type,
            type_map,
            used_types,
        )?);
    }

    Ok(sig)
}

/// Generate the method signature string for bind_java_type! macro
fn generate_method_signature(method: &MethodInfo, type_map: &TypeMap) -> Result<String> {
    let mut unused = HashSet::new();
    generate_method_signature_with_deps(method, type_map, &mut unused)
}

/// Converts a snake_case identifier to lowerCamelCase.
///
/// NOTE: This has been directly copied from the jni_macros proc macro crate so
/// we can reliably evaluate whether the java_name_to_rust transformation is
/// reversible. Method or field bindings with non-reversible names will need to
/// explicitly specify the name of the java type (since we know jni_macros
/// can't derive it from the snake_case name).
///
/// This transformation is idempotent - if the input is already in lowerCamelCase, it returns unchanged.
/// If the input contains any uppercase letters, it's returned unchanged to preserve intentional casing.
/// Leading underscores are preserved except for one underscore that is removed.
/// Trailing underscores are preserved.
///
/// When capitalizing segments after underscores, the first non-numeric character is capitalized.
/// This ensures that segments with numeric prefixes are properly capitalized.
///
/// Examples:
/// - "say_hello" -> "sayHello"
/// - "get_user_name" -> "getUserName"
/// - "_private_method" -> "privateMethod" (one leading underscore removed)
/// - "__dunder__" -> "_dunder__" (one leading underscore removed)
/// - "___priv" -> "__priv" (one leading underscore removed)
/// - "trailing_" -> "trailing_"
/// - "sayHello" -> "sayHello" (unchanged)
/// - "getUserName" -> "getUserName" (unchanged)
/// - "Foo_Bar" -> "Foo_Bar" (unchanged - contains uppercase)
/// - "XMLParser" -> "XMLParser" (unchanged - contains uppercase)
/// - "init" -> "init" (unchanged - no underscores)
/// - "test_αλφα" -> "testΑλφα" (Unicode-aware)
/// - "array_2d_foo" -> "array2DFoo" (capitalizes first char after digits)
/// - "test_3d" -> "test3D" (capitalizes first char after digits)
fn jni_macros_snake_case_to_lower_camel_case(s: &str) -> String {
    // If the string contains any uppercase letters, assume it's intentionally cased
    // and return it unchanged
    if s.chars().any(|c| c.is_uppercase()) {
        return s.to_string();
    }

    // Find leading underscores
    let leading_underscores = s.chars().take_while(|&c| c == '_').count();

    // Find trailing underscores
    let trailing_underscores = s.chars().rev().take_while(|&c| c == '_').count();

    // If the entire string is underscores, return as-is
    if leading_underscores + trailing_underscores >= s.len() {
        return s.to_string();
    }

    // Extract the middle part (without leading/trailing underscores)
    let middle = &s[leading_underscores..s.len() - trailing_underscores];

    // Convert the middle part
    let mut result = String::new();
    let mut capitalize_next = false;

    for c in middle.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            // If this is a digit, just add it and keep looking for the first non-digit to capitalize
            if c.is_ascii_digit() {
                result.push(c);
            } else {
                // Use Unicode-aware uppercase conversion on the first non-digit character
                for upper_c in c.to_uppercase() {
                    result.push(upper_c);
                }
                capitalize_next = false;
            }
        } else {
            result.push(c);
        }
    }

    // Reconstruct with leading and trailing underscores
    // Remove one leading underscore (if any exist)
    let adjusted_leading_underscores = if leading_underscores > 0 {
        leading_underscores - 1
    } else {
        0
    };

    let mut final_result = String::with_capacity(s.len());
    for _ in 0..adjusted_leading_underscores {
        final_result.push('_');
    }
    final_result.push_str(&result);
    for _ in 0..trailing_underscores {
        final_result.push('_');
    }

    final_result
}

/// Convert a Java method name to Rust snake_case
///
/// Returns (rust_name, is_reversible) where is_reversible indicates whether
/// jni_macros can automatically derive the Java name from the Rust name
fn java_name_to_rust(java_name: &str) -> (String, bool) {
    // Check if the name is already in UPPER_SNAKE_CASE (constant style)
    // UPPER_SNAKE_CASE is detected when:
    // - All letters are uppercase
    // - May contain underscores and digits
    // - Has at least one letter
    let has_letter = java_name.chars().any(|c| c.is_alphabetic());
    let is_upper_snake_case = has_letter
        && java_name
            .chars()
            .all(|c| c.is_uppercase() || c == '_' || c.is_numeric());

    if is_upper_snake_case {
        // Keep UPPER_SNAKE_CASE names as-is (e.g., DEFAULT_KEYS_DIALER, RESULT_OK)
        // These are reversible since jni_macros leaves them unchanged
        return (java_name.to_string(), true);
    }

    // Simple conversion: insert underscores before uppercase letters
    // and convert to lowercase
    let mut result = String::new();
    let mut prev_was_lower = false;

    for (i, ch) in java_name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && prev_was_lower {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_was_lower = false;
        } else {
            result.push(ch);
            prev_was_lower = ch.is_lowercase();
        }
    }

    // Check if the transformation is reversible by applying jni_macros algorithm
    let back_converted = jni_macros_snake_case_to_lower_camel_case(&result);
    let is_reversible = back_converted == java_name;

    (result, is_reversible)
}

/// Resolve a type using TypeInfo, preferring type_map lookups, and track dependencies
fn resolve_type_with_deps(
    type_info: &crate::parser_types::TypeInfo,
    type_map: &TypeMap,
    used_types: &mut HashSet<String>,
) -> Result<String> {
    // Get the base/element type first
    let base_type = if let Some(rust_type) = type_map.get_rust_type(&type_info.name) {
        // Type is in the type_map (user-defined class) - track it
        used_types.insert(type_info.name.clone());
        rust_type.to_string()
    } else if let Some(jni_type) = qualified_name_to_jni_type(&type_info.name) {
        // Check for built-in JNI types - no need to track
        jni_type
    } else {
        // Fall back to using the qualified Java type as a string literal - no need to track
        format!("\"{}\"", type_info.name)
    };

    // Add array brackets if needed
    if type_info.array_dimensions > 0 {
        // For builtin types like JString, format as JString[]
        // For quoted types like "android.os.Build", format as "android.os.Build"[]
        Ok(format!(
            "{}{}",
            base_type,
            "[]".repeat(type_info.array_dimensions)
        ))
    } else {
        Ok(base_type)
    }
}

/// Resolve a type using TypeInfo, preferring type_map lookups
fn resolve_type(type_info: &crate::parser_types::TypeInfo, type_map: &TypeMap) -> Result<String> {
    let mut unused = HashSet::new();
    resolve_type_with_deps(type_info, type_map, &mut unused)
}

/// Convert a Java qualified name to a JNI Rust type if it's a known built-in
fn qualified_name_to_jni_type(qualified_name: &str) -> Option<String> {
    // Check builtin JNI types
    for builtin in builtin_jni_types() {
        if builtin.java_name == qualified_name {
            return Some(builtin.rust_name.to_string());
        }
    }

    // Java primitive types (for qualified names from source)
    match qualified_name {
        "boolean" => Some("jboolean".to_string()),
        "byte" => Some("jbyte".to_string()),
        "char" => Some("jchar".to_string()),
        "short" => Some("jshort".to_string()),
        "int" => Some("jint".to_string()),
        "long" => Some("jlong".to_string()),
        "float" => Some("jfloat".to_string()),
        "double" => Some("jdouble".to_string()),
        "void" => Some("void".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser_types::{ArgInfo, MethodSignature, TypeInfo};

    #[test]
    fn test_java_name_to_rust() {
        assert_eq!(
            java_name_to_rust("getValue"),
            ("get_value".to_string(), true)
        );
        assert_eq!(
            java_name_to_rust("getHTTPResponse"),
            ("get_httpresponse".to_string(), false)
        ); // not reversible
        assert_eq!(java_name_to_rust("add"), ("add".to_string(), true));
        assert_eq!(
            java_name_to_rust("toString"),
            ("to_string".to_string(), true)
        );

        // UPPER_SNAKE_CASE should be preserved and reversible
        assert_eq!(
            java_name_to_rust("DEFAULT_KEYS_DIALER"),
            ("DEFAULT_KEYS_DIALER".to_string(), true)
        );
        assert_eq!(
            java_name_to_rust("RESULT_OK"),
            ("RESULT_OK".to_string(), true)
        );
        assert_eq!(
            java_name_to_rust("MAX_VALUE"),
            ("MAX_VALUE".to_string(), true)
        );
        assert_eq!(
            java_name_to_rust("API_LEVEL_35"),
            ("API_LEVEL_35".to_string(), true)
        );
    }

    #[test]
    fn test_overload_sorting() {
        // Create test methods with the same name but different signatures
        let void_type = TypeInfo {
            name: "void".to_string(),
            array_dimensions: 0,
            is_primitive: true,
        };
        let string_type = TypeInfo {
            name: "java.lang.String".to_string(),
            array_dimensions: 0,
            is_primitive: false,
        };
        let int_type = TypeInfo {
            name: "int".to_string(),
            array_dimensions: 0,
            is_primitive: true,
        };

        // Create methods in intentionally mixed order
        let methods = vec![
            MethodInfo {
                name: "test".to_string(),
                documentation: None,
                rust_name_override: None,
                signature: MethodSignature {
                    arguments: vec![
                        ArgInfo {
                            name: Some("arg0".to_string()),
                            type_info: string_type.clone(),
                        },
                        ArgInfo {
                            name: Some("arg1".to_string()),
                            type_info: string_type.clone(),
                        },
                        ArgInfo {
                            name: Some("arg2".to_string()),
                            type_info: int_type.clone(),
                        },
                    ],
                    return_type: void_type.clone(),
                },
                is_static: false,
                is_constructor: false,
                is_native: false,
                is_deprecated: false,
                is_public: true,
            },
            MethodInfo {
                name: "test".to_string(),
                documentation: None,
                rust_name_override: None,
                signature: MethodSignature {
                    arguments: vec![ArgInfo {
                        name: Some("arg0".to_string()),
                        type_info: string_type.clone(),
                    }],
                    return_type: void_type.clone(),
                },
                is_static: false,
                is_constructor: false,
                is_native: false,
                is_deprecated: false,
                is_public: true,
            },
            MethodInfo {
                name: "test".to_string(),
                documentation: None,
                rust_name_override: None,
                signature: MethodSignature {
                    arguments: vec![
                        ArgInfo {
                            name: Some("arg0".to_string()),
                            type_info: int_type.clone(),
                        },
                        ArgInfo {
                            name: Some("arg1".to_string()),
                            type_info: string_type.clone(),
                        },
                    ],
                    return_type: void_type.clone(),
                },
                is_static: false,
                is_constructor: false,
                is_native: false,
                is_deprecated: false,
                is_public: true,
            },
            MethodInfo {
                name: "test".to_string(),
                documentation: None,
                rust_name_override: None,
                signature: MethodSignature {
                    arguments: vec![
                        ArgInfo {
                            name: Some("arg0".to_string()),
                            type_info: string_type.clone(),
                        },
                        ArgInfo {
                            name: Some("arg1".to_string()),
                            type_info: string_type.clone(),
                        },
                    ],
                    return_type: void_type.clone(),
                },
                is_static: false,
                is_constructor: false,
                is_native: false,
                is_deprecated: false,
                is_public: true,
            },
        ];

        let grouped = group_methods_by_name(&methods);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].0, "test");
        assert_eq!(grouped[0].1.len(), 4);

        // Verify they are sorted by argument count first
        assert_eq!(grouped[0].1[0].signature.arguments.len(), 1);
        assert_eq!(grouped[0].1[1].signature.arguments.len(), 2);
        assert_eq!(grouped[0].1[2].signature.arguments.len(), 2);
        assert_eq!(grouped[0].1[3].signature.arguments.len(), 3);

        // Verify that methods with the same argument count are sorted by signature
        // The 2-arg methods should be sorted: (int,String) comes before (String,String)
        assert_eq!(grouped[0].1[1].signature.arguments[0].type_info.name, "int");
        assert_eq!(
            grouped[0].1[2].signature.arguments[0].type_info.name,
            "java.lang.String"
        );
    }

    #[test]
    fn test_fallback_argument_names() {
        // Test that arguments without names get "arg0", "arg1", etc.
        let void_type = TypeInfo {
            name: "void".to_string(),
            array_dimensions: 0,
            is_primitive: true,
        };
        let int_type = TypeInfo {
            name: "int".to_string(),
            array_dimensions: 0,
            is_primitive: true,
        };

        let method = MethodInfo {
            name: "test".to_string(),
            documentation: None,
            rust_name_override: None,
            signature: MethodSignature {
                arguments: vec![
                    ArgInfo {
                        name: None, // No name provided
                        type_info: int_type.clone(),
                    },
                    ArgInfo {
                        name: Some("namedArg".to_string()), // Name provided
                        type_info: int_type.clone(),
                    },
                    ArgInfo {
                        name: None, // No name provided
                        type_info: int_type.clone(),
                    },
                ],
                return_type: void_type,
            },
            is_static: false,
            is_constructor: false,
            is_native: false,
            is_deprecated: false,
            is_public: true,
        };

        let type_map = TypeMap {
            map: std::collections::HashMap::new(),
        };

        let sig = generate_method_signature(&method, &type_map).unwrap();

        // Should generate: (arg0: jint, namedArg: jint, arg2: jint)
        assert!(
            sig.contains("arg0: jint"),
            "Expected 'arg0: jint' in signature, got: {}",
            sig
        );
        assert!(
            sig.contains("namedArg: jint"),
            "Expected 'namedArg: jint' in signature, got: {}",
            sig
        );
        assert!(
            sig.contains("arg2: jint"),
            "Expected 'arg2: jint' in signature, got: {}",
            sig
        );
    }

    #[test]
    fn test_constructor_sorting() {
        use crate::parser_types::ClassInfo;

        // Create test constructors in intentionally mixed order
        let void_type = TypeInfo {
            name: "void".to_string(),
            array_dimensions: 0,
            is_primitive: true,
        };
        let string_type = TypeInfo {
            name: "java.lang.String".to_string(),
            array_dimensions: 0,
            is_primitive: false,
        };
        let int_type = TypeInfo {
            name: "int".to_string(),
            array_dimensions: 0,
            is_primitive: true,
        };

        let class_info = ClassInfo {
            class_name: "com/example/TestClass".to_string(),
            package: vec!["com".to_string(), "example".to_string()],
            simple_name: "TestClass".to_string(),
            documentation: None,
            rust_name_override: None,
            constructors: vec![
                // Constructor with 2 args (String, String)
                MethodInfo {
                    name: "<init>".to_string(),
                    documentation: None,
                    rust_name_override: None,
                    signature: MethodSignature {
                        arguments: vec![
                            ArgInfo {
                                name: None,
                                type_info: string_type.clone(),
                            },
                            ArgInfo {
                                name: None,
                                type_info: string_type.clone(),
                            },
                        ],
                        return_type: void_type.clone(),
                    },
                    is_static: false,
                    is_constructor: true,
                    is_native: false,
                    is_deprecated: false,
                    is_public: true,
                },
                // No-arg constructor
                MethodInfo {
                    name: "<init>".to_string(),
                    documentation: None,
                    rust_name_override: None,
                    signature: MethodSignature {
                        arguments: vec![],
                        return_type: void_type.clone(),
                    },
                    is_static: false,
                    is_constructor: true,
                    is_native: false,
                    is_deprecated: false,
                    is_public: true,
                },
                // Constructor with 2 args (int, String)
                MethodInfo {
                    name: "<init>".to_string(),
                    documentation: None,
                    rust_name_override: None,
                    signature: MethodSignature {
                        arguments: vec![
                            ArgInfo {
                                name: None,
                                type_info: int_type.clone(),
                            },
                            ArgInfo {
                                name: None,
                                type_info: string_type.clone(),
                            },
                        ],
                        return_type: void_type.clone(),
                    },
                    is_static: false,
                    is_constructor: true,
                    is_native: false,
                    is_deprecated: false,
                    is_public: true,
                },
                // Constructor with 1 arg
                MethodInfo {
                    name: "<init>".to_string(),
                    documentation: None,
                    rust_name_override: None,
                    signature: MethodSignature {
                        arguments: vec![ArgInfo {
                            name: None,
                            type_info: int_type.clone(),
                        }],
                        return_type: void_type.clone(),
                    },
                    is_static: false,
                    is_constructor: true,
                    is_native: false,
                    is_deprecated: false,
                    is_public: true,
                },
            ],
            methods: vec![],
            fields: vec![],
            native_methods: vec![],
            instance_of: vec![],
        };

        let options = BindgenOptions::default();
        let type_map = TypeMap::from_classes(&[class_info.clone()], &options);

        let binding = generate_with_type_map(&class_info, &options, &type_map).unwrap();
        let code = binding.binding_code;

        // Verify constructors are sorted by argument count, then by signature
        // With arity-based naming (arity suffix is just N when base doesn't end in number):
        // 1. fn new() - 0 args (base)
        // 2. fn new_int(arg0: jint) - 1 arg, always add type suffix when arity 0 exists -> _int
        // 3. fn new2_int(arg0: jint, arg1: JString) - arity 2, position 0 varies (int vs String), position 1 doesn't -> 2_int
        // 4. fn new2_string(arg0: JString, arg1: JString) - arity 2, position 0 varies -> 2_string

        // Find the positions of each constructor in the generated code
        let new_pos = code.find("fn new()").expect("Should find 'fn new()'");
        let new_int_pos = code
            .find("fn new_int(arg0: jint)")
            .expect("Should find 'fn new_int(arg0: jint)'");
        let new2_int_pos = code
            .find("fn new2_int(arg0: jint, arg1: JString)")
            .expect("Should find 'fn new2_int(arg0: jint, arg1: JString)'");
        let new2_string_pos = code
            .find("fn new2_string(arg0: JString, arg1: JString)")
            .expect("Should find 'fn new2_string(arg0: JString, arg1: JString)'");
        // Verify they appear in the correct order
        assert!(
            new_pos < new_int_pos,
            "fn new() should appear before fn new_int()"
        );
        assert!(
            new_int_pos < new2_int_pos,
            "fn new_int() should appear before fn new2_int()"
        );
        assert!(
            new2_int_pos < new2_string_pos,
            "fn new2_int() should appear before fn new2_string()"
        );
    }

    #[test]
    fn test_name_collision_detection() {
        use crate::parser_types::{ClassInfo, FieldInfo};

        // Test case: toURI and toUri both map to to_uri
        let string_type = TypeInfo {
            name: "java.lang.String".to_string(),
            array_dimensions: 0,
            is_primitive: false,
        };

        let class_info = ClassInfo {
            class_name: "com/example/TestClass".to_string(),
            package: vec!["com".to_string(), "example".to_string()],
            simple_name: "TestClass".to_string(),
            documentation: None,
            rust_name_override: None,
            constructors: vec![],
            methods: vec![
                // First method: toURI
                MethodInfo {
                    name: "toURI".to_string(),
                    documentation: None,
                    rust_name_override: None,
                    signature: MethodSignature {
                        arguments: vec![],
                        return_type: string_type.clone(),
                    },
                    is_static: false,
                    is_constructor: false,
                    is_native: false,
                    is_deprecated: false,
                    is_public: true,
                },
                // Second method: toUri - should get underscore appended
                MethodInfo {
                    name: "toUri".to_string(),
                    documentation: None,
                    rust_name_override: None,
                    signature: MethodSignature {
                        arguments: vec![],
                        return_type: string_type.clone(),
                    },
                    is_static: false,
                    is_constructor: false,
                    is_native: false,
                    is_deprecated: false,
                    is_public: true,
                },
            ],
            fields: vec![
                // Field collision test: myValue and myVALUE both map to my_value
                FieldInfo {
                    name: "myValue".to_string(),
                    documentation: None,
                    rust_name_override: None,
                    type_info: string_type.clone(),
                    is_static: false,
                    is_final: false,
                    is_deprecated: false,
                },
                FieldInfo {
                    name: "myVALUE".to_string(),
                    documentation: None,
                    rust_name_override: None,
                    type_info: string_type.clone(),
                    is_static: false,
                    is_final: false,
                    is_deprecated: false,
                },
            ],
            native_methods: vec![],
            instance_of: vec![],
        };

        let options = BindgenOptions::default();
        let type_map = TypeMap::from_classes(&[class_info.clone()], &options);

        let binding = generate_with_type_map(&class_info, &options, &type_map).unwrap();
        let code = binding.binding_code;

        // Verify that toURI gets to_uri and toUri gets to_uri_ (with underscore)
        // Both should use property syntax since the second is non-reversible
        assert!(
            code.contains("fn to_uri {"),
            "Should find 'fn to_uri {{' for toURI method\nCode:\n{}",
            code
        );
        assert!(
            code.contains("name = \"toURI\""),
            "Should find 'name = \"toURI\"' in generated code\nCode:\n{}",
            code
        );
        assert!(
            code.contains("fn to_uri_ {"),
            "Should find 'fn to_uri_ {{' for toUri method (with underscore)\nCode:\n{}",
            code
        );
        assert!(
            code.contains("name = \"toUri\""),
            "Should find 'name = \"toUri\"' in generated code\nCode:\n{}",
            code
        );

        // Verify field collision handling
        assert!(
            code.contains("my_value {") || code.contains("my_value:"),
            "Should find field 'my_value'\nCode:\n{}",
            code
        );
        assert!(
            code.contains("my_value_ {"),
            "Should find field 'my_value_' (with underscore) for collision\nCode:\n{}",
            code
        );
    }
}
