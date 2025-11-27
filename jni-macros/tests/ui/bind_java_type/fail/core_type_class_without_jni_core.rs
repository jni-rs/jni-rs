// Test that java.lang.Class cannot be bound without __jni_core = true

jni_macros::bind_java_type! {
    rust_type = MyClass,
    java_type = "java.lang.Class",
}

fn main() {}
