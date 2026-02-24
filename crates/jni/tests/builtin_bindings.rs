#![cfg(feature = "invocation")]

mod util;

#[test]
fn test_builtin_bindings() {
    let jvm = util::jvm();

    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        jni::objects::_test_jni_init(env, &Default::default());
        Ok(())
    })
    .unwrap();
}
