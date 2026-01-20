//! Java source parser using JavacTask for extracting API information
//!
//! This module provides integration with the Java compiler API to parse
//! Java source files and extract API information including fully-qualified
//! types and parameter names.

use crate::error::{Error, Result};
use crate::parser_types::{
    ArgInfo, ClassInfo, FieldInfo, InstanceOfInfo, MethodInfo, MethodSignature, TypeInfo,
};
use jni::objects::{JObjectArray, JString};
use jni::{Env, InitArgsBuilder, JNIVersion, JavaVM};
use std::path::PathBuf;
use std::sync::{Arc, Once};

use std::io::Read;

// Define JNI bindings for the Java parser and its nested classes

// First define all the nested type bindings
jni::bind_java_type! {
    rust_type = TypeDescription,
    java_type = "com.jbindgen.Parser$TypeDescription",
    fields {
        type_name: JString,
        descriptor: JString,
        array_dimensions: jint,
        is_primitive: jboolean,
    },
}

jni::bind_java_type! {
    rust_type = ParameterDescription,
    java_type = "com.jbindgen.Parser$ParameterDescription",
    type_map = {
        TypeDescription => com.jbindgen.Parser::TypeDescription,
    },
    fields {
        name: JString,
        type_name: JString,
        descriptor: JString,
        array_dimensions: jint,
        is_primitive: jboolean,
        rust_primitive: JString,
    },
}

jni::bind_java_type! {
    rust_type = MethodDescription,
    java_type = "com.jbindgen.Parser$MethodDescription",
    type_map = {
        ParameterDescription => com.jbindgen.Parser::ParameterDescription,
        TypeDescription => com.jbindgen.Parser::TypeDescription,
    },
    fields {
        name: JString,
        documentation: JString,
        rust_name: JString,
        is_static: jboolean,
        is_constructor: jboolean,
        is_native: jboolean,
        is_deprecated: jboolean,
        is_public: jboolean,
        parameters: ParameterDescription[],
        return_type: TypeDescription,
    },
}

jni::bind_java_type! {
    rust_type = FieldDescription,
    java_type = "com.jbindgen.Parser$FieldDescription",
    fields {
        name: JString,
        documentation: JString,
        rust_name: JString,
        type_name: JString,
        descriptor: JString,
        array_dimensions: jint,
        is_primitive: jboolean,
        is_static: jboolean,
        is_final: jboolean,
        is_deprecated: jboolean,
    },
}

jni::bind_java_type! {
    rust_type = ClassDescription,
    java_type = "com.jbindgen.Parser$ClassDescription",
    type_map = {
        MethodDescription => com.jbindgen.Parser::MethodDescription,
        FieldDescription => com.jbindgen.Parser::FieldDescription,
    },
    fields {
        class_name: JString,
        package_name: JString,
        simple_name: JString,
        documentation: JString,
        rust_name: JString,
        constructors: MethodDescription[],
        methods: MethodDescription[],
        fields: FieldDescription[],
        native_methods: MethodDescription[],
        super_class: JString,
        interfaces: JString[],
    },
}

// Now define the wrapper with the static method
jni::bind_java_type! {
    rust_type = ParserWrapper,
    java_type = "com.jbindgen.ParserWrapper",
    type_map = {
        ClassDescription => com.jbindgen.Parser::ClassDescription,
    },
    methods {
        static fn parse(source_paths: JString[], class_path_entries: JString[], class_pattern: JString) -> ClassDescription[],
    },
}

const PARSER_JAR_BYTES: &[u8] = include_bytes!(env!("JBINDGEN_PARSER_JAR"));

fn load_classes_from_jar(
    env: &mut Env,
    jar_bytes: &[u8],
) -> std::result::Result<(), jni::errors::Error> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let cursor = Cursor::new(jar_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(|e| {
        jni::errors::Error::ParseFailed(format!("Failed to read embedded JAR: {}", e))
    })?;

    // Get the system class loader
    let class_loader = jni::objects::JClassLoader::get_system_class_loader(env)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            jni::errors::Error::ParseFailed(format!("Failed to read JAR entry: {}", e))
        })?;

        let name = file.name().to_string();

        // Only load .class files, skip META-INF and other resources
        if name.ends_with(".class") && !name.starts_with("META-INF/") {
            let mut class_bytes = Vec::new();
            file.read_to_end(&mut class_bytes).map_err(|e| {
                jni::errors::Error::ParseFailed(format!("Failed to read class bytes: {}", e))
            })?;

            // Convert class file path to JNI class name (e.g., "com/example/Foo.class" -> "com/example/Foo")
            let class_name_str = name.trim_end_matches(".class");
            let class_name = jni::strings::JNIString::new(class_name_str);

            // Use define_class to load the class into the JVM
            let _ = env.define_class(Some(&class_name), &class_loader, &class_bytes)?;
        }
    }

    Ok(())
}

/// Get or initialize the JVM for the Java parser
fn get_parser_jvm() -> &'static Arc<JavaVM> {
    static mut JVM: Option<Arc<JavaVM>> = None;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V1_8)
            .build()
            .expect("Failed to build JVM args for parser");

        let jvm = JavaVM::new(jvm_args).expect("Failed to create JVM for parser");
        unsafe { JVM = Some(Arc::new(jvm)) };
    });

    #[allow(static_mut_refs)]
    unsafe {
        JVM.as_ref().unwrap()
    }
}

/// Ensure parser classes are loaded (call once per JVM)
fn ensure_parser_classes_loaded() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let jvm = get_parser_jvm();
        let _ = jvm.attach_current_thread(|env| {
            // Load classes from embedded JAR file using define_class
            load_classes_from_jar(env, PARSER_JAR_BYTES)?;
            Ok::<(), jni::errors::Error>(())
        });
    });
}

/// Parse Java source files using the Java parser
pub fn parse_java_sources(
    source_paths: &[PathBuf],
    class_path_entries: &[PathBuf],
    class_pattern: &str,
) -> Result<Vec<ClassInfo>> {
    let jvm = get_parser_jvm();

    // Ensure classes are loaded once
    ensure_parser_classes_loaded();

    let result: std::result::Result<Vec<ClassInfo>, jni::errors::Error> = jvm
        .attach_current_thread(|env| {
            // Convert source paths to JString array
            let source_paths_array = create_string_array(env, source_paths)?;

            // Convert classpath entries to JString array
            let classpath_array = create_string_array(env, class_path_entries)?;

            // Create pattern JString
            let pattern_jstring = JString::from_str(env, class_pattern)?;

            // Call the parser using the bind_java_type! generated binding
            let result_array =
                ParserWrapper::parse(env, &source_paths_array, &classpath_array, &pattern_jstring)?;

            // Convert Java ClassDescription array to Rust ClassInfo
            let mut classes = Vec::new();
            let len = result_array.len(env)?;
            for i in 0..len {
                let class_desc = result_array.get_element(env, i)?;
                let class_info = java_to_class_info(env, &class_desc)?;
                classes.push(class_info);
            }

            Ok(classes)
        });

    result.map_err(|e| Error::Parse(format!("JNI error: {}", e)))
}

/// Create a JString array from PathBuf slice
fn create_string_array<'a>(
    env: &mut Env<'a>,
    paths: &[PathBuf],
) -> std::result::Result<JObjectArray<'a, JString<'a>>, jni::errors::Error> {
    let array = JObjectArray::<JString>::new(env, paths.len(), JString::null())?;
    for (i, path) in paths.iter().enumerate() {
        let path_str = path
            .to_str()
            .ok_or_else(|| jni::errors::Error::ParseFailed("Invalid path encoding".to_string()))?;
        let jstring = JString::from_str(env, path_str)?;
        array.set_element(env, i, &jstring)?;
    }

    Ok(array)
}

/// Convert Java ClassDescription to ClassInfo
fn java_to_class_info(
    env: &mut Env,
    class_desc: &ClassDescription,
) -> std::result::Result<ClassInfo, jni::errors::Error> {
    let class_name = class_desc.class_name(env)?.to_string();
    let package_name = class_desc.package_name(env)?.to_string();
    let simple_name = class_desc.simple_name(env)?.to_string();

    let package = if package_name.is_empty() {
        Vec::new()
    } else {
        package_name.split('.').map(|s| s.to_string()).collect()
    };

    // Convert constructors
    let constructors_array = class_desc.constructors(env)?;
    let constructors = java_array_to_methods(env, &constructors_array)?;

    // Convert methods
    let methods_array = class_desc.methods(env)?;
    let methods = java_array_to_methods(env, &methods_array)?;

    // Convert fields
    let fields_array = class_desc.fields(env)?;
    let fields = java_array_to_fields(env, &fields_array)?;

    // Convert native methods
    let native_methods_array = class_desc.native_methods(env)?;
    let native_methods = java_array_to_methods(env, &native_methods_array)?;

    // Build instance_of list from superclass and interfaces
    let mut instance_of = Vec::new();

    // Add superclass if it exists and is not java.lang.Object
    let super_class = class_desc.super_class(env)?.to_string();
    if !super_class.is_empty() && super_class != "java.lang.Object" {
        instance_of.push(InstanceOfInfo {
            java_type: super_class,
            stem: None,
        });
    }

    // Add interfaces
    let interfaces_array = class_desc.interfaces(env)?;
    let len = interfaces_array.len(env)?;
    for i in 0..len {
        let interface_jstring = interfaces_array.get_element(env, i)?;
        let interface = interface_jstring.to_string();
        instance_of.push(InstanceOfInfo {
            java_type: interface,
            stem: None,
        });
    }

    Ok(ClassInfo {
        class_name,
        package,
        simple_name,
        documentation: Some(class_desc.documentation(env)?.to_string()),
        rust_name_override: {
            let rust_name_jstring = class_desc.rust_name(env)?;
            if rust_name_jstring.is_null() {
                None
            } else {
                Some(rust_name_jstring.to_string())
            }
        },
        constructors,
        methods,
        fields,
        native_methods,
        instance_of,
    })
}

/// Convert Java MethodDescription array to Vec<MethodInfo>
fn java_array_to_methods(
    env: &mut Env,
    array: &JObjectArray<MethodDescription>,
) -> std::result::Result<Vec<MethodInfo>, jni::errors::Error> {
    let len = array.len(env)?;
    let mut methods = Vec::with_capacity(len);

    for i in 0..len {
        let method_desc = array.get_element(env, i)?;
        let method_info = java_to_method_info(env, &method_desc)?;
        methods.push(method_info);
    }

    Ok(methods)
}

/// Convert Java MethodDescription to MethodInfo
fn java_to_method_info(
    env: &mut Env,
    method_desc: &MethodDescription,
) -> std::result::Result<MethodInfo, jni::errors::Error> {
    let name = method_desc.name(env)?.to_string();
    let documentation = method_desc.documentation(env)?.to_string();
    let is_static = method_desc.is_static(env)?;
    let is_constructor = method_desc.is_constructor(env)?;
    let is_native = method_desc.is_native(env)?;
    let is_deprecated = method_desc.is_deprecated(env)?;
    let is_public = method_desc.is_public(env)?;

    // Convert parameters
    let params_array = method_desc.parameters(env)?;
    let len = params_array.len(env)?;
    let mut arguments = Vec::with_capacity(len);

    for i in 0..len {
        let param_desc = params_array.get_element(env, i)?;
        let arg_info = java_to_arg_info(env, &param_desc)?;
        arguments.push(arg_info);
    }

    // Convert return type
    let return_type_desc = method_desc.return_type(env)?;
    let return_type = java_to_type_info(env, &return_type_desc)?;

    Ok(MethodInfo {
        name,
        documentation: Some(documentation),
        rust_name_override: {
            let rust_name_jstring = method_desc.rust_name(env)?;
            if rust_name_jstring.is_null() {
                None
            } else {
                Some(rust_name_jstring.to_string())
            }
        },
        signature: MethodSignature {
            arguments,
            return_type,
        },
        is_static,
        is_constructor,
        is_native,
        is_deprecated,
        is_public,
    })
}

/// Convert Java ParameterDescription to ArgInfo
fn java_to_arg_info(
    env: &mut Env,
    param_desc: &ParameterDescription,
) -> std::result::Result<ArgInfo, jni::errors::Error> {
    let name = param_desc.name(env)?.to_string();
    let type_name = param_desc.type_name(env)?.to_string();
    let array_dimensions = param_desc.array_dimensions(env)? as usize;
    let is_primitive = param_desc.is_primitive(env)?;

    // Extract rust_primitive if present (may be null)
    let rust_primitive_jstring = param_desc.rust_primitive(env)?;
    let rust_primitive = if rust_primitive_jstring.is_null() {
        None
    } else {
        Some(rust_primitive_jstring.to_string())
    };

    Ok(ArgInfo {
        name: Some(name),
        type_info: TypeInfo {
            name: type_name,
            array_dimensions,
            is_primitive,
        },
        rust_primitive,
    })
}

/// Convert Java TypeDescription to TypeInfo
fn java_to_type_info(
    env: &mut Env,
    type_desc: &TypeDescription,
) -> std::result::Result<TypeInfo, jni::errors::Error> {
    let type_name = type_desc.type_name(env)?.to_string();
    let array_dimensions = type_desc.array_dimensions(env)? as usize;
    let is_primitive = type_desc.is_primitive(env)?;

    Ok(TypeInfo {
        name: type_name,
        array_dimensions,
        is_primitive,
    })
}

/// Convert Java FieldDescription array to Vec<FieldInfo>
fn java_array_to_fields(
    env: &mut Env,
    array: &JObjectArray<FieldDescription>,
) -> std::result::Result<Vec<FieldInfo>, jni::errors::Error> {
    let len = array.len(env)?;
    let mut fields = Vec::with_capacity(len);

    for i in 0..len {
        let field_desc = array.get_element(env, i)?;
        let field_info = java_to_field_info(env, &field_desc)?;
        fields.push(field_info);
    }

    Ok(fields)
}

/// Convert Java FieldDescription to FieldInfo
fn java_to_field_info(
    env: &mut Env,
    field_desc: &FieldDescription,
) -> std::result::Result<FieldInfo, jni::errors::Error> {
    let name = field_desc.name(env)?.to_string();
    let documentation = field_desc.documentation(env)?.to_string();
    let type_name = field_desc.type_name(env)?.to_string();
    let array_dimensions = field_desc.array_dimensions(env)? as usize;
    let is_primitive = field_desc.is_primitive(env)?;
    let is_static = field_desc.is_static(env)?;
    let is_final = field_desc.is_final(env)?;
    let is_deprecated = field_desc.is_deprecated(env)?;

    Ok(FieldInfo {
        name,
        documentation: Some(documentation),
        rust_name_override: {
            let rust_name_jstring = field_desc.rust_name(env)?;
            if rust_name_jstring.is_null() {
                None
            } else {
                Some(rust_name_jstring.to_string())
            }
        },
        type_info: TypeInfo {
            name: type_name,
            array_dimensions,
            is_primitive,
        },
        is_static,
        is_final,
        is_deprecated,
    })
}
