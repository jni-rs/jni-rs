#![cfg(feature = "invocation")]
extern crate error_chain;
extern crate jni;

use error_chain::ChainedError;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jni::objects::JObject;
use jni::objects::JValue;

fn print_exception(env: &JNIEnv) {
    let exception_occurred = env.exception_check()
        .unwrap_or_else(|e| panic!(format!("{:?}", e)));
    if exception_occurred {
        env.exception_describe()
            .unwrap_or_else(|e| panic!(format!("{:?}", e)));
    }
}

#[test]
fn test_java_integers() {
    let jvm_args = InitArgsBuilder::new()
        .version(JNIVersion::V8)
        .option("-Xcheck:jni")
        .option("-Xdebug")
        .build()
        .unwrap_or_else(|e| {
            panic!(format!("{}", e.display_chain().to_string()));
        });

    let jvm = JavaVM::new(jvm_args).unwrap_or_else(|e| {
        panic!(format!("{}", e.display_chain().to_string()));
    });


    let env = jvm.attach_current_thread()
        .expect("failed to attach jvm thread");

    let array_length = 50;

    for value in -10..10 {
        env.with_local_frame(16, || {
            let integer_value = JObject::from(env.new_object(
                "java/lang/Integer",
                "(I)V",
                &[JValue::Int(value)],
            )?);

            let values_array = JObject::from(env.new_object_array(
                array_length,
                "java/lang/Integer",
                integer_value,
            )?);

            let result = env.call_static_method(
                "java/util/Arrays",
                "binarySearch",
                "([Ljava/lang/Object;Ljava/lang/Object;)I",
                &[JValue::Object(values_array), JValue::Object(integer_value)],
            )?
                .i()?;

            assert!(0 <= result && result < array_length);

            Ok(JObject::null())
        }).unwrap_or_else(|e| {
                print_exception(&env);
                panic!(format!("{}", e.display_chain().to_string()));
            });
    }
}
