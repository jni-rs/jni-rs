// Test that core Java types CAN be bound with __jni_core = true

jni_macros::bind_java_type! {
    rust_type = MyObject,
    java_type = "java.lang.Object",
    __jni_core = true,
}

jni_macros::bind_java_type! {
    rust_type = MyClass,
    java_type = "java.lang.Class",
    __jni_core = true,
}

jni_macros::bind_java_type! {
    rust_type = MyString,
    java_type = "java.lang.String",
    __jni_core = true,
}

jni_macros::bind_java_type! {
    rust_type = MyThrowable,
    java_type = "java.lang.Throwable",
    __jni_core = true,
}

fn main() {}
