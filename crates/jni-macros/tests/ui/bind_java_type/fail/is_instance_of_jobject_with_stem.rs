// Test that JObject with custom stem cannot be specified in is_instance_of block

jni_macros::bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    is_instance_of = {
        obj = JObject,
    }
}

fn main() {}
