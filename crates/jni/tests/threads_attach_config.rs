#![cfg(feature = "invocation")]

use std::thread::spawn;

use jni::{AttachConfig, jni_sig, jni_str, objects::JString};

mod util;
use util::jvm;

// Tests that an `AttachConfig` can be used to configure the name of the JVM `Thread`
#[test]
fn attach_config() {
    let jvm = jvm();

    let thread = spawn({
        move || {
            jvm.attach_current_thread_with_config(
                || AttachConfig::new().name("test-thread"),
                None,
                |env| -> jni::errors::Result<_> {
                    // Get the current Thread and query the name
                    let thread = env
                        .call_static_method(
                            jni_str!("java/lang/Thread"),
                            jni_str!("currentThread"),
                            jni_sig!("()Ljava/lang/Thread;"),
                            &[],
                        )
                        .unwrap()
                        .l()
                        .unwrap();
                    let name = env
                        .call_method(
                            thread,
                            jni_str!("getName"),
                            jni_sig!("()Ljava/lang/String;"),
                            &[],
                        )?
                        .l()?;
                    let name = env.cast_local::<JString>(name).unwrap();
                    let name = name.mutf8_chars(env)?;
                    assert_eq!(name.as_cstr(), c"test-thread");
                    Ok(())
                },
            )
        }
    });

    let _ = thread.join().unwrap();
}
