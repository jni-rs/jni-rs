#![cfg(feature = "invocation")]
extern crate error_chain;
extern crate jni;

use jni::{
    objects::JValue,
    sys::jint,
};

mod util;
use util::attach_current_thread;

#[test]
pub fn nested_attach_guard_should_not_detach_thread() {
    let env = attach_current_thread();
    let val = env.call_static_method("java/lang/Math", "abs", "(I)I", &[JValue::from(-1 as jint)])
        .unwrap().i().unwrap();
    assert_eq!(val, 1);

    // create nested AttachGuard
    {
        let env_nested = attach_current_thread();
        let val = env_nested
            .call_static_method("java/lang/Math", "abs", "(I)I", &[JValue::from(-2 as jint)])
            .unwrap().i().unwrap();
        assert_eq!(val, 2);
    }

    // attach after nested guard has been dropped -> check that thread has not been detached
    let val = env.call_static_method("java/lang/Math", "abs", "(I)I", &[JValue::from(-3 as jint)])
        .unwrap().i().unwrap();
    assert_eq!(val, 3);
}
