mod inner {
    use jni_macros::bind_java_type;

    // Test visibility specifiers on methods using shorthand syntax
    bind_java_type! {
        pub TestVisibility => "com.example.TestVisibility",
        methods {
            // Public method (default)
            fn public_method() -> void,

            // Private method using pub(self)
            pub(self) fn private_method() -> void,

            // Private method using priv keyword
            priv fn another_private() -> void,

            // pub(crate) method
            pub(crate) fn crate_method() -> void,
        }
    }

    // Test visibility specifiers on methods using block syntax
    bind_java_type! {
        rust_type = TestVisibilityBlock,
        java_type = "com.example.TestVisibilityBlock",
        methods {
            fn block_public {
                sig = () -> void,
            },

            pub(self) fn block_private {
                sig = () -> void,
            },

            priv fn another_block_private {
                sig = () -> void,
            },
        }
    }

    // Test visibility specifiers on fields
    bind_java_type! {
        rust_type = TestFieldVisibility,
        java_type = "com.example.TestFieldVisibility",
        fields {
            // Public field (default)
            public_field: int,

            // Private field
            pub(self) private_field: int,

            // Field with priv keyword
            priv another_private_field: int,

            // pub(crate) field
            pub(crate) crate_field: int,
        }
    }

    // Test visibility specifiers on getter/setter independently
    bind_java_type! {
        rust_type = TestFieldVisibilityIndependent,
        java_type = "com.example.TestFieldVisibilityIndependent",
        fields {
            // Public getter, private setter
            mixed_visibility {
                sig = int,
                pub get = get_mixed,
                pub(self) set = set_mixed,
            },

            // Both private using block syntax
            both_private {
                sig = int,
                priv get = get_private,
                priv set = set_private,
            },
        }
    }

    // Test that fields inherit visibility from stem
    bind_java_type! {
        rust_type = TestFieldVisibilityInherited,
        java_type = "com.example.TestFieldVisibilityInherited",
        fields {
            // Both getter and setter will be pub(self)
            pub(self) inherited_field: int,

            // Override for setter only
            pub(self) override_setter {
                sig = int,
                pub set = set_override,
            },
        }
    }

    #[test]
    fn test_private_methods_exist() {
        let _ = TestVisibility::private_method;
    }
}

#[test]
fn test_public_methods_exist() {
    let _ = inner::TestVisibility::public_method;
}
