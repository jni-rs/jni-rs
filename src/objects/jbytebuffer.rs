use std::{borrow::Cow, ops::Deref};

use crate::{
    objects::{Global, JClass, LoaderContext},
    Env,
};

struct JByteBufferAPI {
    class: Global<JClass<'static>>,
}

crate::define_reference_type!(
    JByteBuffer,
    "java.nio.ByteBuffer",
    |env: &mut Env, loader_context: &LoaderContext| {
        let class = loader_context.load_class_for_type::<JByteBuffer>(false, env)?;
        let class = env.new_global_ref(&class).unwrap();
        Ok(Self { class })
    }
);
