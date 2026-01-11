// Test that error_policy cannot be used with constructors

use jni::bind_java_type;

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    constructors = {
        fn new {
            sig = () -> void,
            error_policy = ThrowRuntimeExAndDefault
        }
    }
}

fn main() {}
