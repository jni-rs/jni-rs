use std::{borrow::Cow, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{
        Global, JClass, JClassLoader, JMethodID, JObject, JStaticMethodID, JString, JValue,
        LoaderContext,
    },
    signature::{Primitive, ReturnType},
    strings::JNIStr,
    sys::{jobject, jstring},
};

use super::Reference;

#[cfg(doc)]
use crate::errors::Error;

/// A `java.lang.Thread` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JThread<'local>(JObject<'local>);

impl<'local> AsRef<JThread<'local>> for JThread<'local> {
    fn as_ref(&self) -> &JThread<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JThread<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JThread<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JThread<'local>> for JObject<'local> {
    fn from(other: JThread) -> JObject {
        other.0
    }
}

struct JThreadAPI {
    class: Global<JClass<'static>>,
    current_thread_method: JStaticMethodID,
    get_name_method: JMethodID,
    set_name_method: JMethodID,
    get_id_method: JMethodID,
    get_context_class_loader_method: JMethodID,
    set_context_class_loader_method: JMethodID,
}
impl JThreadAPI {
    fn get(env: &Env<'_>) -> Result<&'static Self> {
        static JTHREAD_API: OnceCell<JThreadAPI> = OnceCell::new();
        JTHREAD_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                // NB: Self::CLASS_NAME is a binary name with dots, not slashes
                let class = env.find_class(JNIStr::from_cstr(c"java/lang/Thread"))?;
                let class = env.new_global_ref(&class).unwrap();
                let current_thread_method = env
                    .get_static_method_id(&class, c"currentThread", c"()Ljava/lang/Thread;")
                    .expect("Thread.currentThread method not found");
                let get_name_method = env
                    .get_method_id(&class, c"getName", c"()Ljava/lang/String;")
                    .expect("Thread.getName method not found");
                let set_name_method = env
                    .get_method_id(&class, c"setName", c"(Ljava/lang/String;)V")
                    .expect("Thread.setName method not found");
                let get_id_method = env
                    .get_method_id(&class, c"getId", c"()J")
                    .expect("Thread.getId method not found");
                let get_context_class_loader_method = env
                    .get_method_id(
                        &class,
                        c"getContextClassLoader",
                        c"()Ljava/lang/ClassLoader;",
                    )
                    .expect("Thread.getContextClassLoader method not found");
                let set_context_class_loader_method = env
                    .get_method_id(
                        &class,
                        c"setContextClassLoader",
                        c"(Ljava/lang/ClassLoader;)V",
                    )
                    .expect("Thread.setContextClassLoader method not found");
                Ok(Self {
                    class,
                    current_thread_method,
                    get_name_method,
                    set_name_method,
                    get_id_method,
                    get_context_class_loader_method,
                    set_context_class_loader_method,
                })
            })
        })
    }
}

impl JThread<'_> {
    /// Creates a [`JThread`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    ///
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.lang.Thread`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'local` lifetime.
    pub unsafe fn from_raw<'local>(env: &Env<'local>, raw: jobject) -> JThread<'local> {
        JThread(JObject::from_raw(env, raw))
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JThread` always has the `'static` lifetime.
    pub const fn null() -> JThread<'static> {
        JThread(JObject::null())
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw()
    }

    /// Cast a local reference to a [`JThread`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.lang.Thread`.
    ///
    /// Also see these other options for casting local or global references to a [`JThread`]:
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
    ) -> Result<JThread<'any_local>> {
        env.cast_local::<JThread>(obj)
    }

    /// Get the message of the throwable by calling the `getMessage` method.
    pub fn current_thread<'env_local>(env: &mut Env<'env_local>) -> Result<JThread<'env_local>> {
        let api = JThreadAPI::get(env)?;

        // Safety: We know that `currentThread` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns a valid `Thread` instance.
        unsafe {
            let message = env
                .call_static_method_unchecked(
                    &api.class,
                    api.current_thread_method,
                    ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JThread::from_raw(env, message.into_raw()))
        }
    }

    /// Gets the name of this thread.
    pub fn get_name<'env_local>(&self, env: &mut Env<'env_local>) -> Result<JString<'env_local>> {
        let api = JThreadAPI::get(env)?;

        // Safety: We know that `getName` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns a valid `String` instance.
        unsafe {
            let name = env
                .call_method_unchecked(self, api.get_name_method, ReturnType::Object, &[])?
                .l()?;
            Ok(JString::from_raw(env, name.into_raw() as jstring))
        }
    }

    /// Sets the name of this thread.
    ///
    /// # Throws
    ///
    /// - `SecurityException` if the current thread is not allowed to modify this thread's name
    pub fn set_name(&self, name: &JString<'_>, env: &mut Env<'_>) -> Result<()> {
        let api = JThreadAPI::get(env)?;

        // Safety: We know that `setName` is a valid method on `java/lang/Thread` that takes a
        // single String argument and returns void.
        unsafe {
            env.call_method_unchecked(
                self,
                api.set_name_method,
                ReturnType::Primitive(Primitive::Void),
                &[JValue::Object(name.as_ref()).as_jni()],
            )?;
            Ok(())
        }
    }

    /// Gets the ID of this thread.
    pub fn get_id(&self, env: &mut Env<'_>) -> Result<i64> {
        let api = JThreadAPI::get(env)?;

        // Safety: We know that `getId` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns a valid `long` value.
        unsafe {
            let id = env
                .call_method_unchecked(
                    self,
                    api.get_id_method,
                    ReturnType::Primitive(Primitive::Long),
                    &[],
                )?
                .j()?;
            Ok(id)
        }
    }

    /// Gets the context class loader for this thread.
    ///
    /// This is a Java method binding for `java.lang.Thread#getContextClassLoader()`.
    ///
    /// # Throws
    ///
    /// Throws `SecurityException` if the current thread can't access its context class loader.
    pub fn get_context_class_loader<'local>(
        &self,
        env: &mut Env<'local>,
    ) -> Result<JClassLoader<'local>> {
        let api = JThreadAPI::get(env)?;

        // Safety: We know that `getContextClassLoader` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns a valid `ClassLoader` instance.
        unsafe {
            let loader = env
                .call_method_unchecked(
                    self,
                    api.get_context_class_loader_method,
                    ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JClassLoader::from_raw(env, loader.into_raw()))
        }
    }

    /// Sets the context class loader for this thread.
    ///
    /// The `loader` may be `null` to indicate the system class loader.
    ///
    /// This is a Java method binding for `java.lang.Thread#setContextClassLoader(java.lang.ClassLoader)`.
    ///
    /// # Throws
    ///
    /// Throws `SecurityException` if the current thread can't access its context class loader.
    pub fn set_context_class_loader<'loader_local, 'env_local>(
        &self,
        loader: &JClassLoader<'loader_local>,
        env: &mut Env<'env_local>,
    ) -> Result<()> {
        let api = JThreadAPI::get(env)?;

        // Safety: We know that `setContextClassLoader` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns void.
        unsafe {
            env.call_method_unchecked(
                self,
                api.set_context_class_loader_method,
                ReturnType::Primitive(Primitive::Void),
                &[JValue::Object(loader.as_ref()).as_jni()],
            )?;
            Ok(())
        }
    }
}

// SAFETY: JThread is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JThread<'_> {
    type Kind<'env> = JThread<'env>;
    type GlobalKind = JThread<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.lang.Thread"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        _loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JThreadAPI`)
        let api = JThreadAPI::get(env)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JThread(JObject::kind_from_raw(local_ref))
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JThread(JObject::global_kind_from_raw(global_ref))
    }
}
