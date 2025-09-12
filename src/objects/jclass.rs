use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClassLoader, JMethodID, JObject, JStaticMethodID, JValue, LoaderContext},
    signature::JavaType,
    strings::JNIStr,
    sys::{jclass, jobject},
};

use super::Reference;

/// Lifetime'd representation of a `jclass`. Just a `JObject` wrapped in a new
/// class.
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JClass<'local>(JObject<'local>);

impl<'local> AsRef<JClass<'local>> for JClass<'local> {
    fn as_ref(&self) -> &JClass<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JClass<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JClass<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JClass<'local>> for JObject<'local> {
    fn from(other: JClass) -> JObject {
        other.0
    }
}
struct JClassAPI {
    class: Global<JClass<'static>>,
    get_class_loader_method: JMethodID,
    for_name_method: JStaticMethodID,
    for_name_with_loader_method: JStaticMethodID,
}

impl JClassAPI {
    pub fn get(env: &Env<'_>) -> Result<&'static Self> {
        static JCLASS_API: OnceCell<JClassAPI> = OnceCell::new();
        JCLASS_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                // NB: Self::CLASS_NAME is a binary name with dots, not slashes
                let class = env.find_class(JNIStr::from_cstr(c"java/lang/Class"))?;
                let class = env.new_global_ref(class)?;
                let get_class_loader_method =
                    env.get_method_id(&class, c"getClassLoader", c"()Ljava/lang/ClassLoader;")?;
                let for_name_method = env.get_static_method_id(
                    &class,
                    c"forName",
                    c"(Ljava/lang/String;)Ljava/lang/Class;",
                )?;
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
            })
        })
    }
}

impl JClass<'_> {
    /// Creates a [`JClass`] that wraps the given `raw` [`jclass`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jclass) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Returns the raw JNI pointer.
    pub const fn as_raw(&self) -> jclass {
        self.0.as_raw() as jclass
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jclass {
        self.0.into_raw() as jclass
    }

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
        let api = JClassAPI::get(env)?;

        // Safety: We know that `getClassLoader` is a valid method on `java/lang/Class` that has no
        // arguments and it returns a valid `ClassLoader` instance.
        let loader = unsafe {
            let loader = env
                .call_method_unchecked(self, api.get_class_loader_method, JavaType::Object, &[])?
                .l()?;
            JClassLoader::from_raw(loader.into_raw())
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
        let api = JClassAPI::get(env)?;

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
            JClass::from_raw(class.into_raw())
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
        let api = JClassAPI::get(env)?;

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
            JClass::from_raw(class.into_raw())
        };
        Ok(class)
    }
}

// SAFETY: JClass is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JClass<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.Class");

    type Kind<'env> = JClass<'env>;
    type GlobalKind = JClass<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        _loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JClassAPI`)
        let api = JClassAPI::get(env)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JClass::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JClass::from_raw(global_ref)
    }
}
