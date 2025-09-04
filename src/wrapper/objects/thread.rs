use crate::{
    env::JNIEnv,
    errors::Result,
    objects::{ClassKind, ClassRef, GlobalRef, JClass, JClassLoader, JMethodID, JObject, JString, LoaderContext},
    strings::JNIStr,
    sys::{jobject, jthrowable},
    DataRef, JavaVM,
};

use super::JObjectRef;

/// Lifetime'd representation of a `jthrowable`. Just a `JObject` wrapped in a
/// new class.
#[repr(transparent)]
#[derive(Default)]
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
    class: GlobalRef<JClass<'static>>,
    current_thread_method: JStaticMethodID,
    get_context_class_loader_method: JMethodID,
    set_context_class_loader_method: JMethodID,
}
impl JThreadAPI {
    fn get<'vm, 'any_local>(
        vm: &'vm JavaVM,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<DataRef<'vm, Self>> {
        vm.get_cached_or_insert_with(|| {
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class::<JThread>(env)?;
                let class = env.new_global_ref(&class).unwrap();
                let current_thread_method = env
                    .get_static_method_id(&class, c"currentThread", c"()Ljava/lang/Thread;")
                    .expect("Thread.currentThread method not found");
                let get_context_class_loader_method = env
                    .get_method_id(&class, c"getContextClass", c"()Ljava/lang/ClassLoader;")
                    .expect("Thread.getContextClassLoader method not found");
                let set_context_class_loader_method = env
                    .get_method_id(&class, c"setContextClass", c"(Ljava/lang/ClassLoader;)V")
                    .expect("Thread.setContextClassLoader method not found");
                Ok(Self {
                    class,
                    current_thread_method,
                    get_context_class_loader_method,
                    set_context_class_loader_method
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
    pub fn current_thread(env: &mut JNIEnv<'_>) -> Result<JThread<'_>> {
        let vm = env.get_java_vm();
        let api = JThreadAPI::get(&vm, &LoaderContext::None)?;

        // Safety: We know that `getMessage` is a valid method on `java/lang/Throwable` that has no
        // arguments and it returns a valid `String` instance.
        unsafe {
            let message = env
                .call_static_method_unchecked(
                    api.class.as_raw(),
                    api.current_thread_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JString::from_raw(message.into_raw() as _))
        }
    }

    pub fn get_context_class_loader(&self, env: &mut JNIEnv<'_>) -> Result<JClassLoader<'_>> {
        let vm = env.get_java_vm();
        let api = JThreadAPI::get(&vm, &LoaderContext::None)?;

        // Safety: We know that `getContextClassLoader` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns a valid `ClassLoader` instance.
        unsafe {
            let cause = env
                .call_method_unchecked(
                    self,
                    api.get_cause_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JClassLoader::from_raw(cause.into_raw() as _))
        }
    }

    pub fn set_context_class_loader(&self, loader: &JClassLoader<'_>, env: &mut JNIEnv<'_>) -> Result<JClassLoader<'_>> {
        let vm = env.get_java_vm();
        let api = JThreadAPI::get(&vm, &LoaderContext::None)?;

        // Safety: We know that `getContextClassLoader` is a valid method on `java/lang/Thread` that has no
        // arguments and it returns a valid `ClassLoader` instance.
        unsafe {
            let cause = env
                .call_method_unchecked(
                    self,
                    api.get_cause_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JClassLoader::from_raw(cause.into_raw() as _))
        }
    }
}

impl JObjectRef for JThread<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.Thread");

    type Kind<'env> = JThread<'env>;
    type GlobalKind = JThread<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'vm>(vm: &'vm JavaVM, loader_source: LoaderContext) -> Option<ClassRef<'vm>> {
        let api = JThreadAPI::get(vm, &loader_source).ok()?;
        Some(api.map(|api| &api.class))
    }

    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JThread::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JThread::from_raw(global_ref)
    }
}