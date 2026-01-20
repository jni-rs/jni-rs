//! Parser for Java class files into intermediate representation

use crate::error::{Error, Result};
use crate::parser_types::{
    ArgInfo, ClassInfo, FieldInfo, InstanceOfInfo, MethodInfo, MethodSignature, TypeInfo,
};
use cafebabe::{ClassFile, FieldAccessFlags, MethodAccessFlags};

/// Parse a JNI type descriptor into TypeInfo
fn parse_type_descriptor(descriptor: &str) -> Result<TypeInfo> {
    let mut array_dimensions = 0;
    let mut chars = descriptor.chars();

    // Count array dimensions
    while let Some('[') = chars.clone().peekable().peek() {
        array_dimensions += 1;
        chars.next();
    }

    // Parse element type
    let element_desc: String = chars.collect();
    if element_desc.is_empty() {
        return Err(Error::Parse(format!(
            "Invalid type descriptor: {}",
            descriptor
        )));
    }

    let first_char = element_desc.chars().next().unwrap();

    let (name, is_primitive) = match first_char {
        'Z' => ("boolean".to_string(), true),
        'B' => ("byte".to_string(), true),
        'C' => ("char".to_string(), true),
        'S' => ("short".to_string(), true),
        'I' => ("int".to_string(), true),
        'J' => ("long".to_string(), true),
        'F' => ("float".to_string(), true),
        'D' => ("double".to_string(), true),
        'V' => ("void".to_string(), true),
        'L' => {
            // Object type: Ljava/lang/String; -> java.lang.String
            if !element_desc.ends_with(';') {
                return Err(Error::Parse(format!(
                    "Invalid object type descriptor: {}",
                    element_desc
                )));
            }
            let class_name = &element_desc[1..element_desc.len() - 1];
            (class_name.replace('/', "."), false)
        }
        _ => {
            return Err(Error::Parse(format!(
                "Unknown type descriptor: {}",
                element_desc
            )));
        }
    };

    Ok(TypeInfo {
        name,
        array_dimensions,
        is_primitive,
    })
}

/// Parse a method descriptor into arguments and return type
fn parse_method_descriptor(descriptor: &str) -> Result<(Vec<ArgInfo>, TypeInfo)> {
    if !descriptor.starts_with('(') {
        return Err(Error::Parse(format!(
            "Invalid method descriptor (missing opening paren): {}",
            descriptor
        )));
    }

    let end_params = descriptor.find(')').ok_or_else(|| {
        Error::Parse(format!(
            "Invalid method descriptor (missing closing paren): {}",
            descriptor
        ))
    })?;

    let params_str = &descriptor[1..end_params];
    let return_str = &descriptor[end_params + 1..];

    // Parse parameters
    let mut arguments = Vec::new();
    let mut chars = params_str.chars().peekable();

    while chars.peek().is_some() {
        // Count array dimensions
        let mut array_dims = 0;
        while let Some('[') = chars.peek() {
            array_dims += 1;
            chars.next();
        }

        // Parse element type
        let type_char = chars.next().ok_or_else(|| {
            Error::Parse(format!(
                "Unexpected end of parameter list in: {}",
                descriptor
            ))
        })?;

        let type_str = match type_char {
            'L' => {
                // Object type - consume until ';'
                let mut obj_type = String::from("L");
                loop {
                    match chars.next() {
                        Some(';') => {
                            obj_type.push(';');
                            break;
                        }
                        Some(c) => obj_type.push(c),
                        None => {
                            return Err(Error::Parse(format!(
                                "Unterminated object type in descriptor: {}",
                                descriptor
                            )));
                        }
                    }
                }
                obj_type
            }
            primitive => {
                // Primitive type - single character
                primitive.to_string()
            }
        };

        // Reconstruct full descriptor with array dimensions
        let full_descriptor = format!("{}{}", "[".repeat(array_dims), type_str);
        let type_info = parse_type_descriptor(&full_descriptor)?;

        arguments.push(ArgInfo {
            name: None, // Bytecode doesn't contain parameter names
            type_info,
        });
    }

    // Parse return type
    let return_type = parse_type_descriptor(return_str)?;

    Ok((arguments, return_type))
}

/// Parse a cafebabe ClassFile into our intermediate representation
pub fn parse_class(class: &ClassFile) -> Result<ClassInfo> {
    // Get the class name from the constant pool (convert to String)
    let class_name_str = class.this_class.to_string();

    // Split into package and simple name
    let parts: Vec<&str> = class_name_str.split('/').collect();
    let simple_name = parts.last().unwrap_or(&"").to_string();
    let package = parts[..parts.len().saturating_sub(1)]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut constructors = Vec::new();
    let mut methods = Vec::new();
    let mut fields = Vec::new();
    let mut native_methods = Vec::new();

    // Parse fields
    for field in &class.fields {
        // Skip private fields
        if field.access_flags.contains(FieldAccessFlags::PRIVATE) {
            continue;
        }

        let name = field.name.to_string();
        let descriptor = field.descriptor.to_string();
        let is_static = field.access_flags.contains(FieldAccessFlags::STATIC);
        let is_final = field.access_flags.contains(FieldAccessFlags::FINAL);

        let type_info = parse_type_descriptor(&descriptor)?;

        let field_info = FieldInfo {
            name,
            documentation: None, // Not available from bytecode
            rust_name_override: None, // Not available from bytecode
            type_info,
            is_static,
            is_final,
            is_deprecated: false, // Not available from bytecode
        };

        fields.push(field_info);
    }

    // Parse methods
    for method in &class.methods {
        let name = method.name.to_string();
        let descriptor = method.descriptor.to_string();
        let is_static = method.access_flags.contains(MethodAccessFlags::STATIC);
        let is_native = method.access_flags.contains(MethodAccessFlags::NATIVE);
        let is_constructor = name == "<init>";
        let is_public = method.access_flags.contains(MethodAccessFlags::PUBLIC);

        // Skip class initializers and private methods
        if name == "<clinit>" {
            continue;
        }

        let (arguments, return_type) = parse_method_descriptor(&descriptor)?;

        let method_info = MethodInfo {
            name,
            documentation: None, // Not available from bytecode
            rust_name_override: None, // Not available from bytecode
            signature: MethodSignature {
                arguments,
                return_type,
            },
            is_static,
            is_constructor,
            is_native,
            is_deprecated: false, // Not available from bytecode
            is_public,
        };

        // Native methods go into a separate list for special handling
        if is_native {
            native_methods.push(method_info);
        } else if is_constructor {
            constructors.push(method_info);
        } else {
            methods.push(method_info);
        }
    }

    // Parse superclass and interfaces for instance_of relationships
    let mut instance_of = Vec::new();

    // Add superclass if it exists and is not java.lang.Object
    if let Some(super_class) = &class.super_class {
        let super_class_name = super_class.to_string().replace('/', ".");
        if super_class_name != "java.lang.Object" {
            instance_of.push(InstanceOfInfo {
                java_type: super_class_name,
                stem: None, // No stem for now
            });
        }
    }

    // Add interfaces
    for interface in &class.interfaces {
        let interface_name = interface.to_string().replace('/', ".");
        instance_of.push(InstanceOfInfo {
            java_type: interface_name,
            stem: None, // No stem for now
        });
    }

    Ok(ClassInfo {
        class_name: class_name_str,
        package,
        simple_name,
        documentation: None, // Not available from bytecode
        rust_name_override: None, // Not available from bytecode
        constructors,
        methods,
        fields,
        native_methods,
        instance_of,
    })
}
