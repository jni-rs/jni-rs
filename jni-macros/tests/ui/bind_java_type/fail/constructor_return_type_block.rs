// Test that constructors cannot have a non-void return type (block syntax)

use jni::bind_java_type;

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    constructors = {
        fn new {
            sig = () -> jint
        }
    }
}

fn main() {}
