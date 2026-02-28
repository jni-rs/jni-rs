#![cfg(feature = "invocation")]
// Test that `bind_java_type` is emitting `env.assert_top` runtime checks

mod util;

jni::bind_java_type! {
    pub JBoolean => "java.lang.Boolean",
    constructors {
        fn new(arg0: jboolean),
    },
    methods {
        fn value {
            name = "booleanValue",
            sig = () -> jboolean,
        },
    },
}

// Covers https://github.com/jni-rs/jni-rs/issues/774 bug
#[test]
#[should_panic = "jni runtime check failure"]
fn test_bind_java_type_constructor_emits_assert_top() {
    let jvm = util::jvm();
    jvm.attach_current_thread(|env| {
        let _bad_obj = env.get_java_vm()?.with_local_frame(16, |_| {
            //let s = env.new_string("s")?; // triggers the runtime error
            JBoolean::new(env, true)
        })?;
        eprintln!("ERROR: Should not have reached here without a runtime check!");
        Ok::<_, jni::errors::Error>(())
    })
    .unwrap();
}
