#[path = "../../util/mod.rs"]
mod util;
use util::attach_current_thread;

use jni::objects::ReleaseMode;

pub fn main() {
    attach_current_thread(|env0| {
        let smuggle = env0
            .with_local_frame(10, |env1| -> jni::errors::Result<_> {
                let java_array = env1
                    .new_int_array(3)
                    .expect("JNIEnv#new_int_array must create a java array with given size");

                // It should be OK to get AutoElements from a new `env2` frame because
                // it is only constrained by the `env1` lifetime of it's array reference
                let elems1 = env1.with_local_frame(10, |env2| -> jni::errors::Result<_> {
                    let elems2 = unsafe {
                        env2.get_array_elements(java_array, ReleaseMode::CopyBack)
                            .unwrap()
                    };
                    Ok(elems2)
                })?;

                // But the borrow checker should prevent this...
                Ok(elems1)
            })
            .unwrap();

        eprintln!("BUG: AutoElements has out-lived JNI frame of array reference!");
        drop(smuggle);
        Ok(())
    })
    .unwrap();
}
