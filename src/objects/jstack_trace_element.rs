use crate::{
    env::Env,
    errors::*,
    objects::{Global, JClass, JMethodID, JString, LoaderContext},
    signature::{Primitive, ReturnType},
    sys::jstring,
};

struct JStackTraceElementAPI {
    class: Global<JClass<'static>>,
    get_class_name_method: JMethodID,
    get_file_name_method: JMethodID,
    get_line_number_method: JMethodID,
    get_method_name_method: JMethodID,
    is_native_method: JMethodID,
    to_string_method: JMethodID,
}

crate::define_reference_type!(
    type = JStackTraceElement,
    class = "java.lang.StackTraceElement",
    init = |env, class| {
        Ok(Self {
            class: env.new_global_ref(class)?,
            get_class_name_method: env.get_method_id(class, c"getClassName", c"()Ljava/lang/String;")?,
            get_file_name_method: env.get_method_id(class, c"getFileName", c"()Ljava/lang/String;")?,
            get_line_number_method: env.get_method_id(class, c"getLineNumber", c"()I")?,
            get_method_name_method: env.get_method_id(class, c"getMethodName", c"()Ljava/lang/String;")?,
            is_native_method: env.get_method_id(class, c"isNative", c"()Z")?,
            to_string_method: env.get_method_id(class, c"toString", c"()Ljava/lang/String;")?,
        })
    }
);

impl JStackTraceElement<'_> {
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
