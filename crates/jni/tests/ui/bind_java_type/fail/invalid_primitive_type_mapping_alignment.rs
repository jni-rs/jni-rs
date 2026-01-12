// Test that primitive type mappings with incorrect alignment fail at compile time

use jni_macros::bind_java_type;

#[repr(transparent)]
struct MyInvalidHandle([u8; 8]); // Incorrect alignment for jlong (align 1 vs 8)

bind_java_type! {
    rust_type = JInvalidHandleType,
    java_type = "com.example.HandleType",
    type_map = {
        unsafe MyInvalidHandle => long,
    }
}

fn main() {}
