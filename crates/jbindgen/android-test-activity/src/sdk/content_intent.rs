use tracing::info;

include!(concat!(env!("OUT_DIR"), "/content_intent_bindings.rs"));

use crate::com::github::jni::jbindgen::testactivity::TestActivity;

pub fn test_content_intent<'local>(
    env: &mut jni::Env<'local>,
    activity: TestActivity<'local>,
) -> Result<String, jni::errors::Error> {
    info!("Testing android.content.Intent bindings");

    // Just verify the bindings were generated successfully by accessing a constant
    let action_view_str = android::content::AndroidIntent::ACTION_VIEW(env)?;
    let action_str = action_view_str.to_string();

    // Create result info string
    let result_info = format!("Intent bindings generated\nACTION_VIEW: {}", action_str);

    // Update the activity with test info
    let info_jstring = jni::objects::JString::from_str(env, &result_info)?;
    activity.update_device_info(env, info_jstring)?;

    Ok(format!("Intent bindings: ACTION_VIEW = {}", action_str))
}
