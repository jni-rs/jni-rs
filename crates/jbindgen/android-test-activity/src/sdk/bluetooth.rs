use tracing::info;

include!(concat!(env!("OUT_DIR"), "/bluetooth_bindings.rs"));

use crate::com::github::jni::jbindgen::testactivity::TestActivity;

pub fn test_bluetooth<'local>(
    env: &mut jni::Env<'local>,
    _activity: TestActivity<'local>,
) -> Result<String, jni::errors::Error> {
    info!("Testing android.bluetooth bindings");

    // Just verify that the JNI bindings are able to cache the class, methods, and fields..
    jni_init(env, &Default::default())?;

    Ok("Bluetooth bindings generated successfully".to_string())
}
