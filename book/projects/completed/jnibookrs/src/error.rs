// ANCHOR: try_java
use anyhow::anyhow;
use jni::JNIEnv;
use std::panic::{catch_unwind, AssertUnwindSafe};

pub fn try_java<F, T>(env: JNIEnv, error_value: T, f: F) -> T
where
    F: FnOnce() -> Result<T, anyhow::Error>,
{
    let result = catch_unwind(AssertUnwindSafe(f));
    let result = match result {
        Ok(r) => r,
        Err(_panic_info) => Err(anyhow!("Exception caused by Panic")),
    };

    match result {
        Ok(s) => s,
        Err(e) => {
            // Only throw an exception if one isn't already pending.
            if !env.exception_check().unwrap() {
                env.throw_new("java/lang/RuntimeException", e.to_string())
                    .expect("Failed to throw exception");
            }
            error_value
        }
    }
}
// ANCHOR_END: try_java
