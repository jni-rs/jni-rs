// Smoke test that doc attributes for types, methods and fields are accepted

jni_macros::bind_java_type! {
    /// Bindings for the java.foo.Bar class
    ///
    /// This is a custom documentation comment.
    /// It has multiple lines.
    FooBar => "java.foo.Bar"
}

// Test without doc attribute (should get default docs)
jni_macros::bind_java_type! {
    Baz => "java.baz.Baz"
}

// Test with other attributes (like #[allow(dead_code)])
jni_macros::bind_java_type! {
    #[allow(dead_code)]
    /// Custom docs with other attributes
    Qux => "java.qux.Qux"
}

jni_macros::bind_java_type! {
    /// Bindings for the com.example.WithMethods class
    rust_type = WithMethods,
    java_type = "com.example.WithMethods",
    methods {
        /// This is a test method
        fn test_method() -> void,
    }
}

jni_macros::bind_java_type! {
    /// Bindings for the com.example.WithFields class
    rust_type = WithFields,
    java_type = "com.example.WithFields",
    fields {
        /// This is a test field
        test_field: i32,
        /// Top-level docs applied to getter, with default setter docs
        other_field {
            sig = boolean,
            get = get_other_field,
            set = set_other_field,
        },
        split_field {
            sig = float,
            /// Custom getter docs
            get = get_split_field,
            /// Custom setter docs
            set = set_split_field,
        },
        split_field_with_visibility {
            sig = float,
            /// Custom getter docs
            pub get = get_split2_field,
            /// Custom setter docs
            priv set = set_split2_field,
        }
    }
}

#[test]
fn test_custom_doc_attributes() {
    // This test just verifies compilation succeeds
    // The actual doc output would need cargo doc to verify
}
