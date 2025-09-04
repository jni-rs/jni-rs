use crate::{
    errors::Result,
    objects::{ClassKind, ClassRef, GlobalRef, JClass, JObject, JObjectRef, LoaderContext},
    strings::JNIStr,
    sys::jobject,
    DataRef, JavaVM,
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
    class: GlobalRef<JClass<'static>>,
}

impl JByteBufferAPI {
    fn get<'vm, 'any_local>(
        vm: &'vm JavaVM,
        loader_source: &LoaderContext<'any_local, '_>,
    ) -> Result<DataRef<'vm, Self>> {
        vm.get_cached_or_insert_with(|| {
            vm.with_env_current_frame(|env| {
                let class = loader_source.load_class::<JByteBuffer>(env)?;
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

impl JObjectRef for JByteBuffer<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"[Ljava/nio/ByteBuffer;");
    const LOAD_CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.nio.ByteBuffer");
    const CLASS_KIND: ClassKind = ClassKind::Bootstrap;

    type Kind<'env> = JByteBuffer<'env>;
    type GlobalKind = JByteBuffer<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'vm>(vm: &'vm JavaVM, loader_source: LoaderContext) -> Option<ClassRef<'vm>> {
        let api = JByteBufferAPI::get(vm, &loader_source).ok()?;
        Some(api.map(|api| &api.class))
    }
    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JByteBuffer::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JByteBuffer::from_raw(global_ref)
    }
}
