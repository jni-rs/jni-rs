use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClass, JMethodID, JObject, JObjectArray, JString, LoaderContext},
    strings::JNIStr,
    sys::{jobject, jthrowable},
};

use super::Reference;

/// Lifetime'd representation of a `jthrowable`. Just a `JObject` wrapped in a
/// new class.
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JThrowable<'local>(JObject<'local>);

impl<'local> AsRef<JThrowable<'local>> for JThrowable<'local> {
    fn as_ref(&self) -> &JThrowable<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JThrowable<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JThrowable<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JThrowable<'local>> for JObject<'local> {
    fn from(other: JThrowable) -> JObject {
        other.0
    }
}

struct JThrowableAPI {
    class: Global<JClass<'static>>,
    get_message_method: JMethodID,
    get_cause_method: JMethodID,
    get_stack_trace_method: JMethodID,
}
impl JThrowableAPI {
    fn get<'any_local>(
        env: &Env<'any_local>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JTHROWABLE_API: OnceCell<JThrowableAPI> = OnceCell::new();
        JTHROWABLE_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JThrowable>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();
                let get_message_method = env
                    .get_method_id(&class, c"getMessage", c"()Ljava/lang/String;")
                    .expect("JThrowable.getMessage method not found");
                let get_cause_method = env
                    .get_method_id(&class, c"getCause", c"()Ljava/lang/Throwable;")
                    .expect("JThrowable.getCause method not found");
                let get_stack_trace_method = env
                    .get_method_id(
                        &class,
                        c"getStackTrace",
                        c"()[Ljava/lang/StackTraceElement;",
                    )
                    .expect("JThrowable.getStackTrace method not found");
                Ok(Self {
                    class,
                    get_message_method,
                    get_cause_method,
                    get_stack_trace_method,
                })
            })
        })
    }
}

impl JThrowable<'_> {
    /// Creates a [`JThrowable`] that wraps the given `raw` [`jthrowable`]
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
    pub fn get_message<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<JString<'env_local>> {
        let api = JThrowableAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `getMessage` is a valid method on `java/lang/Throwable` that has no
        // arguments and it returns a valid `String` instance.
        unsafe {
            let message = env
                .call_method_unchecked(
                    self,
                    api.get_message_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JString::from_raw(message.into_raw() as _))
        }
    }

    /// Get the cause of the throwable by calling the `getCause` method.
    pub fn get_cause<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<JThrowable<'env_local>> {
        let api = JThrowableAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `getCause` is a valid method on `java/lang/Throwable` that has no
        // arguments and it returns a valid `Throwable` instance.
        unsafe {
            let cause = env
                .call_method_unchecked(
                    self,
                    api.get_cause_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            Ok(JThrowable::from_raw(cause.into_raw() as _))
        }
    }

    // TODO: it would be nice if we had a generic `JObjectArray<T: Reference>` type so we could
    // create a `JStackTraceElement` `Reference` type.
    /// Gets the stack trace of the throwable by calling the `getStackTrace` method.
    pub fn get_stack_trace<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<JObjectArray<'env_local>> {
        let api = JThrowableAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `getStackTrace` is a valid method on `java/lang/Throwable` that has no
        // arguments and it returns a valid `StackTraceElement` array, which we can
        // safely cast as a `JObjectArray`.
        unsafe {
            let stack_trace = env
                .call_method_unchecked(
                    self,
                    api.get_stack_trace_method,
                    crate::signature::ReturnType::Array,
                    &[],
                )?
                .l()?;
            Ok(JObjectArray::from_raw(stack_trace.into_raw() as _))
        }
    }
}

// SAFETY: JThrowable is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JThrowable<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.Throwable");

    type Kind<'env> = JThrowable<'env>;
    type GlobalKind = JThrowable<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JThrowableAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JThrowable::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JThrowable::from_raw(global_ref)
    }
}
