// Test that primitive type mappings with incorrect size fail at compile time

use jni_macros::bind_java_type;

#[repr(transparent)]
struct MyInvalidHandle(u32); // Incorrect size for jlong (4 bytes vs 8)

bind_java_type! {
    rust_type = JInvalidHandleType,
    java_type = "com.example.HandleType",
    type_map = {
        unsafe MyInvalidHandle => long,
    }
}

fn main() {}
