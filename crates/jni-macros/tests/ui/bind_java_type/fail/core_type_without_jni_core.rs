// Test that core Java types cannot be bound without __jni_core = true

jni_macros::bind_java_type! {
    rust_type = MyObject,
    java_type = "java.lang.Object",
}

fn main() {}
