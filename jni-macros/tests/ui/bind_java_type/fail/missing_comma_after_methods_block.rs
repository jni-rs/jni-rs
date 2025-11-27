// Test that a comma is required after a methods block

use jni::bind_java_type;

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    methods = {
        fn test_method() -> void
    }
    fields = {
        value: jint
    }
}

fn main() {}
