// Test that multiple type_map blocks can be specified

use jni_macros::bind_java_type;

// Forward declare custom types using minimal bind_java_type syntax
bind_java_type! { CustomType1 => com.example.CustomType1 }
bind_java_type! { CustomType2 => com.example.CustomType2 }
bind_java_type! { CustomType3 => com.example.CustomType3 }
bind_java_type! { CustomType4 => com.example.CustomType4 }
bind_java_type! { CustomType5 => com.example.CustomType5 }

bind_java_type! {
    rust_type = TestClass,
    java_type = com.example.TestClass,

    // First type_map block
    type_map {
        CustomType1 => com.example.CustomType1,
        CustomType2 => com.example.CustomType2,
    },

    // Second type_map block - should merge with first
    type_map {
        CustomType3 => com.example.CustomType3,
        CustomType4 => com.example.CustomType4,
    },

    // Third type_map block - should merge with first two
    type_map {
        CustomType5 => com.example.CustomType5,
    },

    // Use the custom types in methods to verify they're properly mapped
    methods {
        fn test_method1(arg1: CustomType1) -> CustomType2,
        fn test_method2(arg2: CustomType2, arg3: CustomType3) -> CustomType4,
        fn test_method3(arg4: CustomType4) -> CustomType5,
    }
}

#[test]
fn test_bind_with_multiple_type_maps_compiles() {
    // This test just verifies compilation succeeds
}
