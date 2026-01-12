// Test that back-to-back commas are not allowed

use jni::bind_java_type;

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",,
    api = TestAPI
}

fn main() {}
