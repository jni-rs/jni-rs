use std::{borrow::Cow, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    errors::Result,
    objects::{Global, JClass, JObject, LoaderContext, Reference},
    strings::JNIStr,
    sys::jobject,
    Env,
};

#[cfg(doc)]
use crate::errors::Error;

/// A `java.nio.ByteBuffer` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
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
    ///
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.nio.ByteBuffer`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'local` lifetime.
    pub unsafe fn from_raw<'local>(env: &Env<'local>, raw: jobject) -> JByteBuffer<'local> {
        JByteBuffer(JObject::from_raw(env, raw))
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JByteBuffer` always has the `'static` lifetime.
    pub const fn null() -> JByteBuffer<'static> {
        JByteBuffer(JObject::null())
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw() as jobject
    }

    /// Cast a local reference to a [`JByteBuffer`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.nio.ByteBuffer`.
    ///
    /// Also see these other options for casting local or global references to a [`JByteBuffer`]:
    /// - [Env::as_cast]
    /// - [Env::new_cast_local_ref]
    /// - [Env::cast_local]
    /// - [Env::new_cast_global_ref]
    /// - [Env::cast_global]
    ///
    /// # Errors
    ///
    /// Returns [Error::WrongObjectType] if the `IsInstanceOf` check fails.
    pub fn cast_local<'any_local>(
        obj: impl Reference + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
        env: &mut Env<'_>,
    ) -> Result<JByteBuffer<'any_local>> {
        env.cast_local::<JByteBuffer>(obj)
    }
}

// SAFETY: JByteBuffer is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JByteBuffer<'_> {
    type Kind<'env> = JByteBuffer<'env>;
    type GlobalKind = JByteBuffer<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"[Ljava.nio.ByteBuffer;"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JByteBufferAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }
    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JByteBuffer(JObject::kind_from_raw(local_ref))
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JByteBuffer(JObject::global_kind_from_raw(global_ref))
    }
}
