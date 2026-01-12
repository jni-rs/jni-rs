// Test that constructors cannot have a custom Java name

use jni::bind_java_type;

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    constructors = {
        fn new {
            name = "customConstructor",
            sig = () -> void
        }
    }
}

fn main() {}
