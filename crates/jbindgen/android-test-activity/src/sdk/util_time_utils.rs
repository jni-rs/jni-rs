use tracing::info;

include!(concat!(env!("OUT_DIR"), "/time_utils_bindings.rs"));

use crate::com::github::jni::jbindgen::testactivity::TestActivity;

pub fn test_time_utils<'local>(
    env: &mut jni::Env<'local>,
    activity: TestActivity<'local>,
) -> Result<String, jni::errors::Error> {
    info!("Testing android.util.TimeUtils and android.icu.util.TimeZone bindings");

    // Just verify that the JNI bindings are able to cache the class, methods, and fields..
    jni_init(env, &Default::default())?;

    // Get timezone database version using the generated bindings
    let tz_version = android::util::TimeUtils::get_time_zone_database_version(env)?;
    let tz_version_str = tz_version.to_string();

    // Get default timezone using android.icu.util.TimeZone bindings
    let default_tz = android::icu::util::TimeZone::get_default(env)?;

    // Get timezone ID
    let tz_id = default_tz.get_id(env)?;
    let tz_id_str = tz_id.to_string();

    // Get timezone display name
    let tz_display_name = default_tz.get_display_name(env)?;
    let tz_display_name_str = tz_display_name.to_string();

    // Create timezone info string
    let timezone_info = format!(
        "Timezone DB: {}\nTimezone: {}\nDisplay: {}",
        tz_version_str, tz_id_str, tz_display_name_str
    );

    // Update the activity with timezone info
    let info_jstring = jni::objects::JString::from_str(env, &timezone_info)?;
    activity.update_device_info(env, info_jstring)?;

    Ok(format!("TimeZone: {} ({})", tz_id_str, tz_version_str))
}
