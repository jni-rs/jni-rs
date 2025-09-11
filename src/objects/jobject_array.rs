use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClass, JObject, JObjectRef, LoaderContext},
    strings::JNIStr,
    sys::{jobject, jobjectArray},
};

use super::AsJArrayRaw;

/// Lifetime'd representation of a [`jobjectArray`] which wraps a [`JObject`] reference
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JObjectArray<'local>(JObject<'local>);

impl<'local> AsRef<JObjectArray<'local>> for JObjectArray<'local> {
    fn as_ref(&self) -> &JObjectArray<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JObjectArray<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JObjectArray<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JObjectArray<'local>> for JObject<'local> {
    fn from(other: JObjectArray) -> JObject {
        other.0
    }
}

unsafe impl<'local> AsJArrayRaw<'local> for JObjectArray<'local> {}

struct JObjectArrayAPI {
    class: Global<JClass<'static>>,
}

impl JObjectArrayAPI {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JOBJECT_ARRAY_API: OnceCell<JObjectArrayAPI> = OnceCell::new();
        JOBJECT_ARRAY_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JObjectArray>(false, env)?;
                let class = env.new_global_ref(&class).unwrap();
                Ok(Self { class })
            })
        })
    }
}

impl JObjectArray<'_> {
    /// Creates a [`JObjectArray`] that wraps the given `raw` [`jobjectArray`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jobjectArray) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Returns the raw JNI pointer.
    pub const fn as_raw(&self) -> jobjectArray {
        self.0.as_raw() as jobjectArray
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobjectArray {
        self.0.into_raw() as jobjectArray
    }

    /// Returns the length of the array.
    pub fn len(&self, env: &Env) -> Result<usize> {
        let array = null_check!(self.as_raw(), "JObjectArray::len self argument")?;
        let len = unsafe { jni_call_unchecked!(env, v1_1, GetArrayLength, array) } as usize;
        Ok(len)
    }
}

// SAFETY: JObjectArray is a transparent JObject wrapper with no Drop side effects
unsafe impl JObjectRef for JObjectArray<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"[Ljava.lang.Object;");

    type Kind<'env> = JObjectArray<'env>;
    type GlobalKind = JObjectArray<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'env>(
        env: &'env Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'env> {
        let api = JObjectArrayAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JObjectArray::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JObjectArray::from_raw(global_ref)
    }
}
