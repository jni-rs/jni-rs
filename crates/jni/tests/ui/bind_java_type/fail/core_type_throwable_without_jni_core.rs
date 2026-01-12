// Test that java.lang.Throwable cannot be bound without __jni_core = true

jni_macros::bind_java_type! {
    rust_type = MyThrowable,
    java_type = "java.lang.Throwable",
}

fn main() {}
