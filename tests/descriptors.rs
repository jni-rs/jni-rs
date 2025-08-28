#![cfg(feature = "invocation")]

mod util;
use util::attach_current_thread;

use jni::{
    descriptors::Desc,
    objects::{AutoLocal, JClass},
};

#[test]
fn test_descriptors() {
    attach_current_thread(|env| {
        let class_local = env.find_class("java/lang/String").unwrap();
        let class_as_ref = Desc::<JClass>::lookup(&class_local, env).unwrap();
        let class_global = env.new_global_ref(class_as_ref).unwrap();
        let _class_as_ref = Desc::<JClass>::lookup(&class_global, env).unwrap();
        let class_auto: AutoLocal<_> = Desc::<JClass>::lookup("java/lang/String", env).unwrap();
        let _class_as_ref = Desc::<JClass>::lookup(&class_auto, env).unwrap();

        Ok(())
    })
    .unwrap();
}
