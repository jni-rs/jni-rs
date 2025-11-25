#![cfg(feature = "invocation")]

use jni::{errors::Result, jni_sig, jni_str, objects::JValue};

mod util;
use util::{attach_current_thread, print_exception};

#[test]
fn test_java_integers() {
    attach_current_thread(|env| {
        let array_length = 50;

        for value in -10..10 {
            env.with_local_frame(16, |env| -> Result<()> {
                let integer_value = env.new_object(
                    jni_str!("java/lang/Integer"),
                    jni_sig!("(I)V"),
                    &[JValue::Int(value)],
                )?;

                let values_array = env.new_object_array(
                    array_length,
                    jni_str!("java/lang/Integer"),
                    &integer_value,
                )?;

                let result = env
                    .call_static_method(
                        jni_str!("java/util/Arrays"),
                        jni_str!("binarySearch"),
                        jni_sig!("([Ljava/lang/Object;Ljava/lang/Object;)I"),
                        &[
                            JValue::Object(&values_array),
                            JValue::Object(&integer_value),
                        ],
                    )?
                    .i()?;

                assert!(0 <= result && result < array_length);

                Ok(())
            })
            .unwrap_or_else(|e| {
                print_exception(env);
                panic!("{:#?}", e);
            })
        }
        Ok(())
    })
    .unwrap();
}
