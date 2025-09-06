#![cfg(feature = "invocation")]

use std::thread::spawn;

use jni::{objects::JString, AttachConfig};

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
                            c"java/lang/Thread",
                            c"currentThread",
                            c"()Ljava/lang/Thread;",
                            &[],
                        )
                        .unwrap()
                        .l()
                        .unwrap();
                    let name = env
                        .call_method(thread, c"getName", c"()Ljava/lang/String;", &[])?
                        .l()?;
                    let name = env.cast_local::<JString>(name).unwrap();
                    let name = env.get_string(&name)?;
                    assert_eq!(name.as_cstr(), c"test-thread");
                    Ok(())
                },
            )
        }
    });

    let _ = thread.join().unwrap();
}
