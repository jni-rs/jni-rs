// This is a separate test program because it has to start a JVM with a specific option.

#![cfg(feature = "invocation")]

use std::borrow::Cow;

use jni::{objects::JString, InitArgsBuilder, JavaVM};

#[test]
fn invocation_character_encoding() {
    let jvm_args = InitArgsBuilder::new()
        .version(jni::JNIVersion::V1_8)
        .option("-Xcheck:jni")
        // U+00A0 NO-BREAK SPACE is the only non-ASCII character that's present in all parts of
        // ISO 8859. This minimizes the chance of this test failing as a result of the character
        // not being present in the platform default character encoding. This test will still fail
        // on platforms where the default character encoding cannot represent a no-break space,
        // such as GBK.
        .option("-Dnbsp=\u{00a0}")
        .build()
        .unwrap_or_else(|e| panic!("{:#?}", e));

    let jvm = JavaVM::new(jvm_args).unwrap_or_else(|e| panic!("{:#?}", e));

    jvm.attach_current_thread(|env| -> jni::errors::Result<()> {
        println!("creating new_string, env = {:?}", env.get_raw());
        let prop_name = env.new_string("nbsp").unwrap();

        println!("calling getProperty");
        let prop_value: JString = env
            .call_static_method(
                "java/lang/System",
                "getProperty",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[(&prop_name).into()],
            )
            .unwrap()
            .l()
            .unwrap()
            .into();

        let prop_value_str = env.get_string(&prop_value).unwrap();
        let prop_value_str: Cow<str> = prop_value_str.to_str();

        assert_eq!("\u{00a0}", prop_value_str);
        Ok(())
    })
    .unwrap()
}
