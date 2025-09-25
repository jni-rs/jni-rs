use crate::objects::{Global, JClass};

struct JByteBufferAPI {
    class: Global<JClass<'static>>,
}

crate::define_reference_type!(
    type = JByteBuffer,
    class = "java.nio.ByteBuffer",
    init = |env, class| {
        Ok(Self { class: env.new_global_ref(&class)? })
    }
);
