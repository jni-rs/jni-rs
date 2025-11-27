// Test that java.lang.String cannot be bound without __jni_core = true

jni_macros::bind_java_type! {
    rust_type = MyString,
    java_type = "java.lang.String",
}

fn main() {}
