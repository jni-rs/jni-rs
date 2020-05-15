use jni::objects::JClass;
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_com_github_jni_1rs_jnibook_NativeAPI_verify_1link(
    _env: JNIEnv,
    _class: JClass,
) {
}
