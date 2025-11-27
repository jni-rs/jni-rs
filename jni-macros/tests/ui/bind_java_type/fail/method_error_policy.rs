// Test that error_policy cannot be used with regular methods

use jni::bind_java_type;

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    methods = {
        fn test_method {
            sig = () -> void,
            error_policy = ThrowRuntimeExAndDefault
        }
    }
}

fn main() {}
