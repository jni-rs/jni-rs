use std::{borrow::Cow, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClass, JMethodID, JObject, JValue, LoaderContext},
    signature::JavaType,
    strings::JNIStr,
    sys::{jclass, jobject},
    DEFAULT_LOCAL_FRAME_CAPACITY,
};

use super::Reference;

#[cfg(doc)]
use crate::errors::Error;

/// A `java.lang.ClassLoader` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
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
            env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
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
    /// Creates a [`JClassLoader`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    ///
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.lang.ClassLoader`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'local` lifetime.
    pub unsafe fn from_raw<'local>(env: &Env<'local>, raw: jobject) -> JClassLoader<'local> {
        JClassLoader(JObject::from_raw(env, raw))
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JClassLoader` always has the `'static` lifetime.
    pub const fn null() -> JClassLoader<'static> {
        JClassLoader(JObject::null())
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw()
    }

    /// Cast a local reference to a [`JClassLoader`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.lang.ClassLoader`.
    ///
    /// Also see these other options for casting local or global references to a [`JClassLoader`]:
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
    ) -> Result<JClassLoader<'any_local>> {
        env.cast_local::<JClassLoader>(obj)
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
            JClass::from_raw(env, cls.into_raw() as jclass)
        };
        Ok(cls_obj)
    }
}

// SAFETY: JClassLoader is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JClassLoader<'_> {
    type Kind<'env> = JClassLoader<'env>;
    type GlobalKind = JClassLoader<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.lang.ClassLoader"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        _loader_context: &LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JClassLoaderAPI`)
        let api = JClassLoaderAPI::get(env)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JClassLoader(JObject::kind_from_raw(local_ref))
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JClassLoader(JObject::global_kind_from_raw(global_ref))
    }
}
