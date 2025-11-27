// Test that a private setter (pub(self)) cannot be accessed from outside the module

mod inner {
    use jni::bind_java_type;

    bind_java_type! {
        rust_type = TestClass,
        java_type = "com.example.TestClass",
        fields {
            // Public getter, private setter
            mixed_field {
                sig = int,
                pub get = get_mixed,
                pub(self) set = set_mixed,
            },
        }
    }
}

fn main() {
    // This should fail - set_mixed is pub(self) and cannot be accessed from outside the module
    let _ = inner::TestClass::set_mixed;
}
