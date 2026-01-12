use tracing::info;

include!(concat!(env!("OUT_DIR"), "/net_uri_bindings.rs"));

use crate::com::github::jni::jbindgen::testactivity::TestActivity;
use android::net::AndroidUri;

pub fn test_net_uri<'local>(
    env: &mut jni::Env<'local>,
    activity: TestActivity<'local>,
) -> Result<String, jni::errors::Error> {
    info!("Testing android.net.Uri bindings");

    // Parse a URI using the generated bindings
    let uri_str = jni::objects::JString::from_str(env, "https://github.com/jni-rs/jni-rs")?;
    let uri = AndroidUri::parse(env, uri_str)?;

    // Get URI components
    let scheme = uri.get_scheme(env)?;
    let scheme_str = scheme.to_string();

    let host = uri.get_host(env)?;
    let host_str = host.to_string();
    let path = uri.get_path(env)?;
    let path_str = path.to_string();

    // Create result info string
    let result_info = format!(
        "URI Parsing:\nScheme: {}\nHost: {}\nPath: {}",
        scheme_str, host_str, path_str
    );

    // Update the activity with test info
    let info_jstring = jni::objects::JString::from_str(env, &result_info)?;
    activity.update_device_info(env, info_jstring)?;

    Ok(format!("Uri: {}://{}{}", scheme_str, host_str, path_str))
}
