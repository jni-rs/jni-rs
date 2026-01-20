//! Shared intermediate representation types for Java class parsing
//!
//! These types are used by both the cafebabe (bytecode) parser and the
//! Java source parser to represent parsed Java API information.

/// Intermediate representation of a Java class for binding generation
#[derive(Debug, Clone)]
pub struct ClassInfo {
    /// Fully qualified class name (e.g., "com/example/MyClass")
    pub class_name: String,
    /// Package name parts (e.g., ["com", "example"])
    pub package: Vec<String>,
    /// Simple class name (e.g., "MyClass")
    pub simple_name: String,
    /// Optional Javadoc documentation for the class
    pub documentation: Option<String>,
    /// Optional Rust type name override from @RustName annotation
    pub rust_name_override: Option<String>,
    /// List of constructors
    pub constructors: Vec<MethodInfo>,
    /// List of methods (both instance and static)
    pub methods: Vec<MethodInfo>,
    /// List of fields (both instance and static)
    pub fields: Vec<FieldInfo>,
    /// List of native methods
    pub native_methods: Vec<MethodInfo>,
    /// List of types this class is an instance of (superclasses and interfaces)
    pub instance_of: Vec<InstanceOfInfo>,
}

/// Information about a type relationship for is_instance_of
#[derive(Debug, Clone)]
pub struct InstanceOfInfo {
    /// Fully qualified Java type name (e.g., "android.app.Activity")
    pub java_type: String,
    /// Optional stem name for generating as_*() method (e.g., "activity" generates "as_activity()")
    /// If None, only From trait implementations are generated
    pub stem: Option<String>,
}

/// Intermediate representation of a Java method
#[derive(Debug, Clone)]
pub struct MethodInfo {
    /// Java method name
    pub name: String,
    /// Optional Javadoc documentation for the method
    pub documentation: Option<String>,
    /// Optional Rust method name override from @RustName annotation
    pub rust_name_override: Option<String>,
    /// Method signature
    pub signature: MethodSignature,
    /// Whether this is a static method
    pub is_static: bool,
    /// Whether this is a constructor
    pub is_constructor: bool,
    /// Whether this is a native method (requires JNI implementation)
    pub is_native: bool,
    /// Whether this method is marked as deprecated
    pub is_deprecated: bool,
    /// Whether this method is public (default: true for backwards compatibility)
    pub is_public: bool,
}

/// Method signature representation with structured arguments
#[derive(Debug, Clone)]
pub struct MethodSignature {
    /// Method arguments with type information
    pub arguments: Vec<ArgInfo>,
    /// Return type information
    pub return_type: TypeInfo,
}

/// Information about a method argument
#[derive(Debug, Clone)]
pub struct ArgInfo {
    /// Argument name (None if not available from source - generator will use "arg0", "arg1", etc.)
    pub name: Option<String>,
    /// Type information
    pub type_info: TypeInfo,
}

/// Java type information
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Fully qualified type name (e.g., "java.lang.String", "int", "boolean")
    /// For primitives, uses Java primitive names: boolean, byte, char, short, int, long, float, double, void
    /// For arrays, this is the element type WITHOUT [] suffix
    pub name: String,
    /// Number of array dimensions (0 for non-arrays, 1 for T[], 2 for T[][], etc.)
    pub array_dimensions: usize,
    /// Whether the element type is a primitive (boolean, int, etc.)
    pub is_primitive: bool,
}

/// Intermediate representation of a Java field
#[derive(Debug, Clone)]
pub struct FieldInfo {
    /// Java field name
    pub name: String,
    /// Optional Javadoc documentation for the field
    pub documentation: Option<String>,
    /// Optional Rust field name override from @RustName annotation
    pub rust_name_override: Option<String>,
    /// Field type information
    pub type_info: TypeInfo,
    /// Whether this is a static field
    pub is_static: bool,
    /// Whether this is a final field
    pub is_final: bool,
    /// Whether this field is marked as deprecated
    pub is_deprecated: bool,
}
