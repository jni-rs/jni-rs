use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClass, JMethodID, JObject, JValue, LoaderContext},
    signature::JavaType,
    strings::JNIStr,
    sys::{jclass, jobject},
};

use super::JObjectRef;

/// A `java.lang.ClassLoader` reference
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JClassLoader<'local>(JObject<'local>);

impl<'local> AsRef<JClassLoader<'local>> for JClassLoader<'local> {
    fn as_ref(&self) -> &JClassLoader<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JClassLoader<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JClassLoader<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JClassLoader<'local>> for JObject<'local> {
    fn from(other: JClassLoader<'local>) -> JObject<'local> {
        other.0
    }
}

struct JClassLoaderAPI {
    class: Global<JClass<'static>>,
    load_class_method: JMethodID,
}

impl JClassLoaderAPI {
    fn get(env: &Env<'_>) -> Result<&'static Self> {
        static JCLASS_LOADER_API: OnceCell<JClassLoaderAPI> = OnceCell::new();
        JCLASS_LOADER_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                // NB: Self::CLASS_NAME is a binary name with dots, not slashes
                let class = env.find_class(c"java/lang/ClassLoader")?;
                let class = env.new_global_ref(&class).unwrap();
                let load_class_method = env.get_method_id(
                    &class,
                    c"loadClass",
                    c"(Ljava/lang/String;)Ljava/lang/Class;",
                )?;
                Ok(Self {
                    class,
                    load_class_method,
                })
            })
        })
    }
}

impl JClassLoader<'_> {
    /// Creates a [`JClass`] that wraps the given `raw` [`jclass`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jclass) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Returns the raw JNI pointer.
    pub const fn as_raw(&self) -> jclass {
        self.0.as_raw() as jclass
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jclass {
        self.0.into_raw() as jclass
    }

    /// Loads a class by name using this class loader.
    ///
    /// This is a Java method binding for `java.lang.ClassLoader.loadClass(String)`.
    ///
    /// # Throws
    ///
    /// `ClassNotFoundException` if the class cannot be found.
    pub fn load_class<'local>(
        &self,
        name: &JNIStr,
        env: &mut Env<'local>,
    ) -> Result<JClass<'local>> {
        let api = JClassLoaderAPI::get(env)?;

        let name = env.new_string(name)?;

        // SAFETY:
        // - we know that `self` is a valid `JClassLoader` reference and `load_class_method` is a valid method ID.
        // - we know that `loadClass` returns a valid `Class` reference.
        let cls_obj = unsafe {
            let cls = env
                .call_method_unchecked(
                    self,
                    api.load_class_method,
                    JavaType::Object,
                    &[JValue::Object(&name).as_jni()],
                )?
                .l()?;
            JClass::from_raw(cls.into_raw())
        };
        Ok(cls_obj)
    }
}

// SAFETY: JClassLoader is a transparent JObject wrapper with no Drop side effects
unsafe impl JObjectRef for JClassLoader<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.ClassLoader");

    type Kind<'env> = JClassLoader<'env>;
    type GlobalKind = JClassLoader<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'env>(
        env: &'env Env<'_>,
        _loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'env> {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JClassLoaderAPI`)
        let api = JClassLoaderAPI::get(env)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JClassLoader::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JClassLoader::from_raw(global_ref)
    }
}
