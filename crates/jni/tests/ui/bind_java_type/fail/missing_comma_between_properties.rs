// Test that a comma is required between simple properties

use jni::bind_java_type;

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass"
    api = TestAPI
}

fn main() {}
