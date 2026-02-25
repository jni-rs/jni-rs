#![cfg(feature = "invocation")]

mod util;

#[test]
fn test_builtin_bindings() {
    let jvm = util::jvm();

    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        jni::__test_bindings_init(env, &Default::default());
        Ok(())
    })
    .unwrap();
}
