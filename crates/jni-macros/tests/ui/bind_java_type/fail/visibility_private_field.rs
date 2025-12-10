// Test that private fields (pub(self)) cannot be accessed from outside the module

mod inner {
    use jni::bind_java_type;

    bind_java_type! {
        rust_type = TestClass,
        java_type = "com.example.TestClass",
        fields {
            // Private field using priv keyword (both getter and setter are private)
            priv private_field: int,
        }
    }
}

fn main() {
    // This should fail - private_field getter is pub(self) and cannot be accessed from outside the module
    let _ = inner::TestClass::private_field;
}
