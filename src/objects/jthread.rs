use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{
        Global, JClass, JClassLoader, JMethodID, JObject, JStaticMethodID, JValue, LoaderContext,
    },
    strings::JNIStr,
    sys::{jobject, jthrowable},
};

use super::Reference;

/// Lifetime'd representation of a `jthrowable`. Just a `JObject` wrapped in a
/// new class.
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
                    get_context_class_loader_method,
                    set_context_class_loader_method,
                })
            })
        })
    }
}

impl JThread<'_> {
    /// Creates a [`JThread`] that wraps the given `raw` [`jthrowable`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jthrowable) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jthrowable {
        self.0.into_raw() as jthrowable
    }

    /// Get the message of the throwable by calling the `getMessage` method.
    pub fn current_thread<'local>(env: &mut Env<'_>) -> Result<JThread<'local>> {
        let api = JThreadAPI::get(env)?;

        // Safety: We know that `currentThread` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns a valid `Thread` instance.
        unsafe {
            let message = env
                .call_static_method_unchecked(
                    &api.class,
                    api.current_thread_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JThread::from_raw(message.into_raw()))
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
            let cause = env
                .call_method_unchecked(
                    self,
                    api.get_context_class_loader_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JClassLoader::from_raw(cause.into_raw()))
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
    ) -> Result<JClassLoader<'env_local>> {
        let api = JThreadAPI::get(env)?;

        // Safety: We know that `getContextClassLoader` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns a valid `ClassLoader` instance.
        unsafe {
            let cause = env
                .call_method_unchecked(
                    self,
                    api.set_context_class_loader_method,
                    crate::signature::ReturnType::Object,
                    &[JValue::Object(loader.as_ref()).as_jni()],
                )?
                .l()?;
            Ok(JClassLoader::from_raw(cause.into_raw()))
        }
    }
}

// SAFETY: JThread is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JThread<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.Thread");

    type Kind<'env> = JThread<'env>;
    type GlobalKind = JThread<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'env>(
        env: &'env Env<'_>,
        _loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'env> {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JThreadAPI`)
        let api = JThreadAPI::get(env)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JThread::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JThread::from_raw(global_ref)
    }
}
