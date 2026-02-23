#![cfg(feature = "invocation")]

mod util;

use rusty_fork::rusty_fork_test;

rusty_fork_test! {
#[test]
fn test_exception_unsafe_calls() {
    let jvm = util::jvm();
    let res = jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let _ = env.throw("Test Exception".to_string());

        let res = env.new_string("Test");

        println!("new_string res = {:?}", res);
        // No new string should be allocated because there is a pending exception
        // and until the exception is cleared, the JNI calls would lead to
        // undefined behavior
        assert!(matches!(res, Err(jni::errors::Error::JavaException)));

        let pending = env.with_local_frame(8, |env| -> jni::errors::Result<bool> {
            Ok(env.exception_check())
        }).expect("Push/PopLocalFrame + ExceptionCheck should be exception safe");

        assert!(pending);
        Ok(())
    });

    println!("res = {:?}", res);
    /*
    assert!(matches!(res, Err(jni::errors::Error::CaughtJavaException {
        ref name,
        ref msg,
        ..
    }) if name == "java.lang.RuntimeException" && msg == "Test Exception"));
*/
}
}

#[test]
fn test_get_method_id_exception_side_effects() {
    let jvm = util::jvm();

    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let s = env.new_string("Test").unwrap();

        let str_class = env.get_object_class(&s).unwrap();
        let res = env.get_method_id(
            str_class,
            jni::jni_str!("invalidMethod"),
            jni::jni_sig!("()V"),
        );
        println!("method ID lookup result {:?}", res);

        match res {
            Err(jni::errors::Error::MethodNotFound { .. }) => {
                assert!(
                    !env.exception_check(),
                    "Expected no pending exception with MethodNotFound error"
                );
            }
            Err(jni::errors::Error::JavaException) => {
                assert!(env.exception_check());
                panic!("Expected MethodNotFound error for invalid method lookup");
            }
            _ => {
                assert!(
                    !env.exception_check(),
                    "Spurious pending exception without JavaException error"
                );
                panic!("Expected a MethodNotFound error for invalid method lookup");
            }
        }
        Ok(())
    })
    .unwrap();
}

#[test]
fn test_get_field_id_exception_side_effects() {
    let jvm = util::jvm();

    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let s = env.new_string("Test").unwrap();

        let str_class = env.get_object_class(&s).unwrap();
        let res = env.get_field_id(str_class, jni::jni_str!("invalidField"), jni::jni_sig!("Z"));
        println!("field ID lookup result {:?}", res);

        match res {
            Err(jni::errors::Error::FieldNotFound { .. }) => {
                assert!(
                    !env.exception_check(),
                    "Expected no pending exception with FieldNotFound error"
                );
            }
            Err(jni::errors::Error::JavaException) => {
                assert!(env.exception_check());
                panic!("Expected FieldNotFound error for invalid field lookup");
            }
            _ => {
                assert!(
                    !env.exception_check(),
                    "Spurious pending exception without JavaException error"
                );
                panic!("Expected a FieldNotFound error for invalid field lookup");
            }
        }
        Ok(())
    })
    .unwrap();
}
