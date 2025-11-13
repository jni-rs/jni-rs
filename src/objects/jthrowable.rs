use std::{borrow::Cow, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{
        Global, JClass, JMethodID, JObject, JObjectArray, JStackTraceElement, JString,
        LoaderContext,
    },
    strings::JNIStr,
    sys::{jobject, jstring, jthrowable},
    DEFAULT_LOCAL_FRAME_CAPACITY,
};

use super::Reference;

#[cfg(doc)]
use crate::errors::Error;

/// A `java.lang.Throwable` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
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
            env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
                let class = loader_context.load_class_for_type::<JThrowable>(env, true)?;
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
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.lang.Throwable`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'local` lifetime.
    pub unsafe fn from_raw<'local>(env: &Env<'local>, raw: jthrowable) -> JThrowable<'local> {
        JThrowable(JObject::from_raw(env, raw as jobject))
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JThrowable` always has the `'static` lifetime.
    pub const fn null() -> JThrowable<'static> {
        JThrowable(JObject::null())
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jthrowable {
        self.0.into_raw() as jthrowable
    }

    /// Cast a local reference to a [`JThrowable`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.lang.Throwable`.
    ///
    /// Also see these other options for casting local or global references to a [`JThrowable`]:
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
    ) -> Result<JThrowable<'any_local>> {
        env.cast_local::<JThrowable>(obj)
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
            Ok(JString::from_raw(env, message.into_raw() as jstring))
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
            Ok(JThrowable::from_raw(env, cause.into_raw() as jthrowable))
        }
    }

    /// Gets the stack trace of the throwable by calling the `getStackTrace` method.
    pub fn get_stack_trace<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<JObjectArray<'env_local, JStackTraceElement<'env_local>>> {
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
            Ok(JObjectArray::<JStackTraceElement>::from_raw(
                env,
                stack_trace.into_raw(),
            ))
        }
    }
}

// SAFETY: JThrowable is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JThrowable<'_> {
    type Kind<'env> = JThrowable<'env>;
    type GlobalKind = JThrowable<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.lang.Throwable"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: &LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JThrowableAPI::get(env, loader_context)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JThrowable(JObject::kind_from_raw(local_ref))
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JThrowable(JObject::global_kind_from_raw(global_ref))
    }
}
