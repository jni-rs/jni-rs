use std::{borrow::Cow, ops::Deref};

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClass, JMethodID, JValue, LoaderContext},
    signature::JavaType,
    strings::JNIStr,
    sys::jclass,
};

struct JClassLoaderAPI {
    class: Global<JClass<'static>>,
    load_class_method: JMethodID,
}

crate::define_reference_type!(
    JClassLoader,
    "java.lang.ClassLoader",
    |env: &mut Env, _loader_context: &LoaderContext| {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JClassLoaderAPI`)
        let class = env.find_class(JNIStr::from_cstr(c"java/lang/ClassLoader"))?;
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
    }
);

impl JClassLoader<'_> {
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
        let api = JClassLoaderAPI::get(env, &LoaderContext::None)?;

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
