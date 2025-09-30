use std::{borrow::Cow, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::*,
    objects::{Global, JClass, JMethodID, JObject, JString, LoaderContext},
    signature::{Primitive, ReturnType},
    strings::JNIStr,
    sys::{jobject, jstring},
    DEFAULT_LOCAL_FRAME_CAPACITY,
};

use super::Reference;

/// A `java.lang.StackTraceElement` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JStackTraceElement<'local>(JObject<'local>);

impl<'local> AsRef<JStackTraceElement<'local>> for JStackTraceElement<'local> {
    fn as_ref(&self) -> &JStackTraceElement<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JStackTraceElement<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JStackTraceElement<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JStackTraceElement<'local>> for JObject<'local> {
    fn from(other: JStackTraceElement) -> JObject {
        other.0
    }
}

struct JStackTraceElementAPI {
    class: Global<JClass<'static>>,
    get_class_name_method: JMethodID,
    get_file_name_method: JMethodID,
    get_line_number_method: JMethodID,
    get_method_name_method: JMethodID,
    is_native_method: JMethodID,
    to_string_method: JMethodID,
}

impl JStackTraceElementAPI {
    fn get(env: &Env<'_>, loader_context: &LoaderContext<'_, '_>) -> Result<&'static Self> {
        static JSTACK_TRACE_ELEMENT_API: OnceCell<JStackTraceElementAPI> = OnceCell::new();
        JSTACK_TRACE_ELEMENT_API.get_or_try_init(|| {
            env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
                let class = loader_context.load_class_for_type::<JStackTraceElement>(false, env)?;
                let class = env.new_global_ref(&class).unwrap();

                let get_class_name_method = env
                    .get_method_id(&class, c"getClassName", c"()Ljava/lang/String;")
                    .expect("StackTraceElement.getClassName method not found");
                let get_file_name_method = env
                    .get_method_id(&class, c"getFileName", c"()Ljava/lang/String;")
                    .expect("StackTraceElement.getFileName method not found");
                let get_line_number_method = env
                    .get_method_id(&class, c"getLineNumber", c"()I")
                    .expect("StackTraceElement.getLineNumber method not found");
                let get_method_name_method = env
                    .get_method_id(&class, c"getMethodName", c"()Ljava/lang/String;")
                    .expect("StackTraceElement.getMethodName method not found");
                let is_native_method = env
                    .get_method_id(&class, c"isNative", c"()Z")
                    .expect("StackTraceElement.isNative method not found");
                let to_string_method = env
                    .get_method_id(&class, c"toString", c"()Ljava/lang/String;")
                    .expect("StackTraceElement.toString method not found");

                Ok(Self {
                    class,
                    get_class_name_method,
                    get_file_name_method,
                    get_line_number_method,
                    get_method_name_method,
                    is_native_method,
                    to_string_method,
                })
            })
        })
    }
}

impl JStackTraceElement<'_> {
    /// Creates a [`JStackTraceElement`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    ///
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.lang.StackTraceElement`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'local` lifetime.
    pub unsafe fn from_raw<'local>(env: &Env<'local>, raw: jobject) -> JStackTraceElement<'local> {
        JStackTraceElement(JObject::from_raw(env, raw))
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JStackTraceElement` always has the `'static` lifetime.
    pub const fn null() -> JStackTraceElement<'static> {
        JStackTraceElement(JObject::null())
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw()
    }

    /// Cast a local reference to a [`JStackTraceElement`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.lang.StackTraceElement`.
    ///
    /// Also see these other options for casting local or global references to a [`JStackTraceElement`]:
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
    ) -> Result<JStackTraceElement<'any_local>> {
        env.cast_local::<JStackTraceElement>(obj)
    }

    /// Get the class name of the stack trace element.
    pub fn get_class_name<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<JString<'env_local>> {
        let api = JStackTraceElementAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `getClassName` is a valid method on `java/lang/StackTraceElement` that has no
        // arguments and it returns a valid `String` instance.
        unsafe {
            let class_name = env
                .call_method_unchecked(self, api.get_class_name_method, ReturnType::Object, &[])?
                .l()?;
            Ok(JString::from_raw(env, class_name.into_raw() as jstring))
        }
    }

    /// Get the file name of the stack trace element, if available.
    pub fn get_file_name<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<JString<'env_local>> {
        let api = JStackTraceElementAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `getFileName` is a valid method on `java/lang/StackTraceElement` that has no
        // arguments and it returns a valid `String` instance or null.
        unsafe {
            let file_name = env
                .call_method_unchecked(self, api.get_file_name_method, ReturnType::Object, &[])?
                .l()?;
            Ok(JString::from_raw(env, file_name.into_raw() as jstring))
        }
    }

    /// Get the line number of the stack trace element.
    pub fn get_line_number(&self, env: &mut Env<'_>) -> Result<i64> {
        let api = JStackTraceElementAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `getLineNumber` is a valid method on `java/lang/StackTraceElement` that has no
        // arguments and it returns a valid `int` value.
        unsafe {
            let line_number = env
                .call_method_unchecked(
                    self,
                    api.get_line_number_method,
                    ReturnType::Primitive(Primitive::Int),
                    &[],
                )?
                .j()?;
            Ok(line_number)
        }
    }

    /// Get the method name of the stack trace element.
    pub fn get_method_name<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<JString<'env_local>> {
        let api = JStackTraceElementAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `getMethodName` is a valid method on `java/lang/StackTraceElement` that has no
        // arguments and it returns a valid `String` instance.
        unsafe {
            let method_name = env
                .call_method_unchecked(self, api.get_method_name_method, ReturnType::Object, &[])?
                .l()?;
            Ok(JString::from_raw(env, method_name.into_raw() as jstring))
        }
    }

    /// Check if the stack trace element corresponds with a native method.
    pub fn is_native_method(&self, env: &mut Env<'_>) -> Result<bool> {
        let api = JStackTraceElementAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `isNative` is a valid method on `java/lang/StackTraceElement` that has no
        // arguments and it returns a valid `boolean` value.
        unsafe {
            let is_native = env
                .call_method_unchecked(
                    self,
                    api.is_native_method,
                    ReturnType::Primitive(Primitive::Boolean),
                    &[],
                )?
                .z()?;
            Ok(is_native)
        }
    }

    /// Returns a string representation of this stack trace element.
    pub fn try_to_string<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<JString<'env_local>> {
        let api = JStackTraceElementAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `toString` is a valid method on `java/lang/StackTraceElement` that has no
        // arguments and it returns a valid `String` instance.
        unsafe {
            let string = env
                .call_method_unchecked(self, api.to_string_method, ReturnType::Object, &[])?
                .l()?;
            Ok(JString::from_raw(env, string.into_raw() as jstring))
        }
    }
}

// SAFETY: JStackTraceElement is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JStackTraceElement<'_> {
    type Kind<'env> = JStackTraceElement<'env>;
    type GlobalKind = JStackTraceElement<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.lang.StackTraceElement"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: &LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JStackTraceElementAPI::get(env, loader_context)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JStackTraceElement(JObject::kind_from_raw(local_ref))
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JStackTraceElement(JObject::global_kind_from_raw(global_ref))
    }
}
