// Test that JObject cannot be explicitly specified in is_instance_of block
// since all types are already instances of JObject

jni_macros::bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    is_instance_of = {
        JObject,
    }
}

fn main() {}
