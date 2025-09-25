use std::{borrow::Cow, ops::Deref};

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClassLoader, JMethodID, JStaticMethodID, JValue, LoaderContext},
    signature::JavaType,
    strings::JNIStr,
};

#[cfg(doc)]
use crate::errors::Error;

struct JClassAPI {
    class: Global<JClass<'static>>,
    get_class_loader_method: JMethodID,
    for_name_method: JStaticMethodID,
    for_name_with_loader_method: JStaticMethodID,
}

crate::define_reference_type!(
    JClass,
    "java.lang.Class",
    |env: &mut Env, _loader_context: &LoaderContext| {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JClassAPI`)
        let class = env.find_class(JNIStr::from_cstr(c"java/lang/Class"))?;
        let class = env.new_global_ref(class)?;
        let get_class_loader_method =
            env.get_method_id(&class, c"getClassLoader", c"()Ljava/lang/ClassLoader;")?;
        let for_name_method =
            env.get_static_method_id(&class, c"forName", c"(Ljava/lang/String;)Ljava/lang/Class;")?;
        let for_name_with_loader_method = env.get_static_method_id(
            &class,
            c"forName",
            c"(Ljava/lang/String;ZLjava/lang/ClassLoader;)Ljava/lang/Class;",
        )?;
        Ok(Self {
            class,
            get_class_loader_method,
            for_name_method,
            for_name_with_loader_method,
        })
    }
);

impl JClass<'_> {
    /// Returns the class loader for this class.
    ///
    /// This is used to find the class loader that was responsible for loading this class.
    ///
    /// It may return null for bootstrap classes or objects representing primitive types not associated with a class loader.
    ///
    /// # Throws
    ///
    /// `SecurityException` if the class loader cannot be accessed.
    pub fn get_class_loader<'local>(&self, env: &mut Env<'local>) -> Result<JClassLoader<'local>> {
        let api = JClassAPI::get(env, &LoaderContext::None)?;

        // Safety: We know that `getClassLoader` is a valid method on `java/lang/Class` that has no
        // arguments and it returns a valid `ClassLoader` instance.
        let loader = unsafe {
            let loader = env
                .call_method_unchecked(self, api.get_class_loader_method, JavaType::Object, &[])?
                .l()?;
            JClassLoader::from_raw(env, loader.into_raw())
        };
        Ok(loader)
    }

    /// Finds a class by its fully-qualified binary name or array descriptor.
    ///
    /// This is a method binding for `java.lang.Class.forName(String)`
    ///
    /// This method is used to locate a class by its name, which may be either a fully-qualified
    /// binary name (e.g., `java.lang.String`) or an array descriptor (e.g., `[Ljava.lang.String;`).
    ///
    /// Note: that unlike `FindClass` the names use dot (`.`) notation instead of slash (`/`) notation.
    ///
    /// # Throws
    ///
    /// This method may throw a `ClassNotFoundException` if the class cannot be found.
    pub fn for_name<'local, C>(class_name: C, env: &mut Env<'local>) -> Result<JClass<'local>>
    where
        C: AsRef<JNIStr>,
    {
        let api = JClassAPI::get(env, &LoaderContext::None)?;

        let class_name = env.new_string(class_name)?;

        // Safety: We know that `forName` is a valid static method on `java/lang/Class` that takes
        // a String and returns a valid `Class` instance.
        let class = unsafe {
            let class = env
                .call_static_method_unchecked(
                    &api.class,
                    api.for_name_method,
                    JavaType::Object,
                    &[JValue::Object(&class_name).as_jni()],
                )?
                .l()?;
            JClass::from_raw(env, class.into_raw())
        };
        Ok(class)
    }

    /// Finds a class by its fully-qualified binary name or array descriptor.
    ///
    /// This is a method binding for `java.lang.Class.forName(String, boolean, ClassLoader)`
    ///
    /// This method is used to locate a class by its name (via the ClassLoader) which may be either
    /// a fully-qualified binary name (e.g., `java.lang.String`) or an array descriptor (e.g.,
    /// `[Ljava.lang.String;`).
    ///
    /// Note: that unlike `FindClass` the names use dot (`.`) notation instead of slash (`/`) notation.
    ///
    /// If initialized is true, the class will be initialized before it is returned.
    ///
    /// # Throws
    ///
    /// This method may throw a `ClassNotFoundException` if the class cannot be found.
    pub fn for_name_with_loader<'loader_local, 'env_local, C, L>(
        class_name: C,
        initialize: bool,
        loader: L,
        env: &mut Env<'env_local>,
    ) -> Result<JClass<'env_local>>
    where
        C: AsRef<JNIStr>,
        L: AsRef<JClassLoader<'loader_local>>,
    {
        let api = JClassAPI::get(env, &LoaderContext::None)?;

        let class_name = env.new_string(class_name)?;

        // Safety: We know that `forName` is a valid static method on `java/lang/Class` that takes
        // a String, initializer boolean and a ClassLoader and returns a valid `Class` instance.
        let class = unsafe {
            let class = env
                .call_static_method_unchecked(
                    &api.class,
                    api.for_name_with_loader_method,
                    JavaType::Object,
                    &[
                        JValue::Object(&class_name).as_jni(),
                        JValue::Bool(initialize).as_jni(),
                        JValue::Object(loader.as_ref()).as_jni(),
                    ],
                )?
                .l()?;
            JClass::from_raw(env, class.into_raw())
        };
        Ok(class)
    }
}
