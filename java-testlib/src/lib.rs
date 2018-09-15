extern crate jni;

use jni::objects::JClass;
use jni::sys::jint;
use jni::JNIEnv;


#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn Java_jni_it_StaticJniCalls_abs(_env: JNIEnv,
                                                      _class: JClass,
                                                      x: jint) -> jint {
    x.abs()
}

