// Test that raw/fn cannot be used with constructors

use jni::EnvUnowned;
use jni::bind_java_type;
use jni::sys::jobject;

extern "system" fn raw_constructor<'local>(_env: EnvUnowned<'local>) -> jobject {
    std::ptr::null_mut()
}

bind_java_type! {
    rust_type = TestClass,
    java_type = "com.example.TestClass",
    constructors = {
        fn new {
            sig = () -> void,
            fn = raw_constructor,
            raw = true,
        }
    }
}

fn main() {}
