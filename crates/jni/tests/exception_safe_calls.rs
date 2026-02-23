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

#[test]
fn test_global_drop_no_exception_side_effects() {
    let jvm = util::jvm();

    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let s = env.new_string("Test").unwrap();
        let global = env.new_global_ref(s).unwrap();

        // Don't use '?' here because we want to continue with the JavaException error
        // indicating a pending exception.
        let _ = env.throw("Test Exception");

        assert!(env.exception_check());

        drop(global);

        assert!(
            env.exception_check(),
            "Expected exception to still be pending after dropping global reference"
        );

        env.exception_clear();

        Ok(())
    })
    .unwrap();
}

#[test]
fn test_weak_drop_no_exception_side_effects() {
    let jvm = util::jvm();

    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let s = env.new_string("Test").unwrap();
        let weak = env.new_weak_ref(s).unwrap();

        // Don't use '?' here because we want to continue with the JavaException error
        // indicating a pending exception.
        let _ = env.throw("Test Exception");

        assert!(env.exception_check());

        drop(weak);
        assert!(
            env.exception_check(),
            "Expected exception to still be pending after dropping weak reference"
        );

        env.exception_clear();

        Ok(())
    })
    .unwrap();
}

#[test]
fn test_env_throw_apis_return_java_exception_err() {
    let jvm = util::jvm();

    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let res = env.throw("Test Exception");
        assert!(matches!(res, Err(jni::errors::Error::JavaException)));
        assert!(env.exception_check());

        let catch = env.exception_catch();
        println!("Caught exception: {:?}", catch);
        assert!(
            matches!(catch, Err(jni::errors::Error::CaughtJavaException {
                ref name,
                ref msg,
                ..
            }) if name == "java.lang.RuntimeException" && msg == "Test Exception")
        );
        assert!(!env.exception_check());

        let res = env.throw_new(
            jni::jni_str!("java/lang/Exception"),
            jni::jni_str!("something bad happened"),
        );
        assert!(matches!(res, Err(jni::errors::Error::JavaException)));
        assert!(env.exception_check());

        let catch = env.exception_catch();
        println!("Caught exception: {:?}", catch);
        assert!(
            matches!(catch, Err(jni::errors::Error::CaughtJavaException {
                ref name,
                ref msg,
                ..
            }) if name == "java.lang.Exception" && msg == "something bad happened")
        );
        assert!(!env.exception_check());

        let res = env.throw_new_void(jni::jni_str!("java/lang/NullPointerException"));
        assert!(matches!(res, Err(jni::errors::Error::JavaException)));
        assert!(env.exception_check());

        let catch = env.exception_catch();
        println!("Caught exception: {:?}", catch);
        assert!(
            matches!(catch, Err(jni::errors::Error::CaughtJavaException {
                ref name,
                ref msg,
                ..
            }) if name == "java.lang.NullPointerException")
        );
        assert!(!env.exception_check());

        Ok(())
    })
    .unwrap();
}
