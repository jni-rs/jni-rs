// Test that private methods (pub(self)) cannot be accessed from outside the module

mod inner {
    use jni::bind_java_type;

    bind_java_type! {
        rust_type = TestClass,
        rust_type_vis = pub,
        java_type = "com.example.TestClass",
        methods {
            // Private method using pub(self)
            pub(self) fn private_method() -> void,
        }
    }
}

fn main() {
    // This should fail - private_method is pub(self) and cannot be accessed from outside the module
    let _ = inner::TestClass::private_method;
}
