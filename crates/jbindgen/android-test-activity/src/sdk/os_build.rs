use tracing::info;

include!(concat!(env!("OUT_DIR"), "/os_build_bindings.rs"));

use android::os::AndroidBuild;
use crate::com::github::jni::jbindgen::testactivity::TestActivity;

pub fn test_os_build<'local>(
    env: &mut jni::Env<'local>,
    activity: TestActivity<'local>,
) -> Result<String, jni::errors::Error> {
    info!("Testing android.os.Build bindings");

    // Use the generated AndroidBuild bindings to access static fields
    let brand = AndroidBuild::BRAND(env)?;
    let brand_str = brand.to_string();

    let model = AndroidBuild::MODEL(env)?;
    let model_str = model.to_string();

    let device = AndroidBuild::DEVICE(env)?;
    let device_str = device.to_string();

    let manufacturer = AndroidBuild::MANUFACTURER(env)?;
    let manufacturer_str = manufacturer.to_string();

    let version_release = android::os::AndroidBuildVERSION::RELEASE(env)?;
    let version_str = version_release.to_string();

    let sdk_int = android::os::AndroidBuildVERSION::SDK_INT(env)?;

    // Create detailed build info string
    let build_info = format!(
        "Device: {} {}\nManufacturer: {}\nModel: {}\nAndroid: {} (SDK {})",
        brand_str, device_str, manufacturer_str, model_str, version_str, sdk_int
    );

    // Call back to the activity to update device info on screen
    let info_jstring = jni::objects::JString::from_str(env, &build_info)?;
    activity.update_device_info(env, info_jstring)?;

    let result = format!(
        "Brand: {}, Model: {}, Device: {}",
        brand_str, model_str, device_str
    );

    info!("{}", &result);
    Ok(result)
}
