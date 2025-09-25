use crate::{
    env::Env,
    errors::Result,
    objects::{
        Global, JClass, JMethodID, JObjectArray, JStackTraceElement, JString, LoaderContext,
    },
    sys::{jstring, jthrowable},
};

#[cfg(doc)]
use crate::errors::Error;

struct JThrowableAPI {
    class: Global<JClass<'static>>,
    get_message_method: JMethodID,
    get_cause_method: JMethodID,
    get_stack_trace_method: JMethodID,
}

crate::define_reference_type!(
    type = JThrowable,
    class = "java.lang.Throwable",
    raw = jthrowable,
    init = |env, class| {
        Ok(JThrowableAPI {
            class: env.new_global_ref(class)?,
            get_message_method: env.get_method_id(class, c"getMessage", c"()Ljava/lang/String;")?,
            get_cause_method: env.get_method_id(class, c"getCause", c"()Ljava/lang/Throwable;")?,
            get_stack_trace_method: env.get_method_id(class, c"getStackTrace", c"()[Ljava/lang/StackTraceElement;")?,
        })
    }
);

impl JThrowable<'_> {
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
