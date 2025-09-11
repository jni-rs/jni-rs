use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    errors::Result,
    objects::{Global, JClass, JObject, JObjectRef, LoaderContext},
    strings::JNIStr,
    sys::jobject,
    Env,
};

/// Lifetime'd representation of a `jobject` that is an instance of the
/// ByteBuffer Java class. Just a `JObject` wrapped in a new class.
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JByteBuffer<'local>(JObject<'local>);

impl<'local> AsRef<JByteBuffer<'local>> for JByteBuffer<'local> {
    fn as_ref(&self) -> &JByteBuffer<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JByteBuffer<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JByteBuffer<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JByteBuffer<'local>> for JObject<'local> {
    fn from(other: JByteBuffer) -> JObject {
        other.0
    }
}

struct JByteBufferAPI {
    class: Global<JClass<'static>>,
}

impl JByteBufferAPI {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JBYTEBUFFER_API: OnceCell<JByteBufferAPI> = OnceCell::new();
        JBYTEBUFFER_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JByteBuffer>(false, env)?;
                let class = env.new_global_ref(&class).unwrap();
                Ok(Self { class })
            })
        })
    }
}

impl JByteBuffer<'_> {
    /// Creates a [`JByteBuffer`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    /// No runtime check is made to verify that the given [`jobject`] is an instance of
    /// a `ByteBuffer`.
    pub const unsafe fn from_raw(raw: jobject) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw() as jobject
    }
}

// SAFETY: JByteBuffer is a transparent JObject wrapper with no Drop side effects
unsafe impl JObjectRef for JByteBuffer<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"[Ljava.nio.ByteBuffer;");

    type Kind<'env> = JByteBuffer<'env>;
    type GlobalKind = JByteBuffer<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'env>(
        env: &'env Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'env> {
        let api = JByteBufferAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }
    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JByteBuffer::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JByteBuffer::from_raw(global_ref)
    }
}
