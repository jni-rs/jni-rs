use std::sync::Mutex;

use errors::*;

use JNIEnv;

use objects::{GlobalRef, JObject, JClass, JValue};

use strings::JNIString;

use lazy_static::lazy_static;

/// The `loadClass` function name.
const LOAD_CLASS: &str = "loadClass";
/// The `loadClass` signature.
const LOAD_CLASS_SIG: &str = "(Ljava/lang/String;)Ljava/lang/Class;";

lazy_static! {
    /// The global class loader instance.
    static ref CLASS_LOADER: Mutex<Option<GlobalRef>> = Mutex::default();
}

/// Register the given object as the global class loader instance.
pub fn register_class_loader<'a>(env: &JNIEnv<'a>, class_loader: JObject<'a>) -> Result<()> {
    // Check that the `loadClass` function is present.
    env.get_method_id(class_loader, LOAD_CLASS, LOAD_CLASS_SIG)?;

    *CLASS_LOADER.lock().unwrap() = Some(env.new_global_ref(class_loader)?);

    Ok(())
}

/// Unregister the global class loader instance.
pub fn unregister_class_loader() {
    *CLASS_LOADER.lock().unwrap() = None;
}

/// Look up a class by name.
///
/// Either it uses the registered `CLASS_LOADER` or it falls back to use the
/// JNI env function `FindClass`.
pub(crate) fn load_class<'a>(env: &JNIEnv<'a>, name: JNIString) -> Result<JClass<'a>> {
    match *CLASS_LOADER.lock().unwrap() {
        Some(ref class_loader) => {
            let name = env.new_string(name)?;
            let res = env.call_method(
                class_loader.as_obj(),
                LOAD_CLASS,
                LOAD_CLASS_SIG,
                &[JValue::Object(name.into())]
            )?;
            res.l().map(Into::into)
        },
        None => {
            let class = jni_non_null_call!(env.get_native_interface(), FindClass, name.as_ptr());
            Ok(class)
        }
    }
}