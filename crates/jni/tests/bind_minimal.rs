#![cfg(feature = "invocation")]
use jni_macros::bind_java_type;

bind_java_type! { JMinimal0 => "min.Minimal" }

bind_java_type! { JMinimal1 => "min.Minimal$Inner" }

bind_java_type! { JMinimal2 => min.Minimal }

bind_java_type! { JMinimal3 => min.Minimal::Inner }

bind_java_type! { JMinimal4 => .Minimal }

bind_java_type! { JMinimal5 => .Minimal::Inner }

bind_java_type! {
    rust_type = JMinimal6,
    java_type = min.Minimal
}

bind_java_type! {
    rust_type = JMinimal7,
    java_type = min.Minimal7, // trailing comma is ok
}

bind_java_type! {
    rust_type = JMinimal8,
    java_type = min.Minimal8,
    api = JMinimalAPIType,
}

// The shorthand syntax and property / block syntax can be mixed
// Additionally, multiple type_map blocks are allowed and merged (to help support
// wrapper macros)
bind_java_type! {
    JMinimal9 => min.Minimal,
    type_map = {
        JMinimal7 => min.Minimal7,
    },
    type_map = {
        JMinimal8 => min.Minimal8,
    },
    constructors = { fn new() },
}

#[test]
fn test_minimal_bindings_compile() {
    println!("Compiled successfully!");
}
