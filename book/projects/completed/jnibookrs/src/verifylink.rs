// ANCHOR: imports
use jni::objects::JClass;
use jni::JNIEnv;
// ANCHOR_END: imports

#[cfg(feature="link_0")]
// ANCHOR: link_0
// #[no_mangle] - Disables name mangling, so that the compiler doesn't rename the function
// in the shared library.
//
// extern "system" - Specifies the ABI
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_verify_1link(
) {}
// ANCHOR_END: link_0

#[cfg(feature="link_complete")]
// ANCHOR: complete
// Although no arguments are used in this function, the first two arguments must
// always be in the native method's signature.
// ANCHOR: complete_no_info
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_verify_1link(
    _env: JNIEnv,
    _class: JClass
) {}
// ANCHOR_END: complete_no_info
// ANCHOR_END: complete
