use crate::{
    env::JNIEnv,
    errors::Result,
    objects::{ClassKind, ClassRef, GlobalRef, JClassLoader, JMethodID, JObject, LoaderSource},
    signature::JavaType,
    strings::JNIStr,
    sys::{jclass, jobject},
    DataRef, JavaVM,
};

use super::JObjectRef;

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
    class: GlobalRef<JClass<'static>>,
    get_class_loader_method: JMethodID,
}

impl JClassAPI {
    pub fn get<'vm>(vm: &'vm JavaVM) -> Result<DataRef<'vm, Self>> {
        vm.get_cached_or_insert_with(|| {
            vm.with_env_current_frame(|env| {
                let class = env.find_class(JClass::FIND_CLASS_NAME)?;
                let class = env.new_global_ref(class)?;
                let get_class_loader_method =
                    env.get_method_id(&class, c"getClassLoader", c"()Ljava/lang/ClassLoader;")?;
                Ok(Self {
                    class,
                    get_class_loader_method,
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
    pub fn get_class_loader<'local>(
        &self,
        env: &mut JNIEnv<'local>,
    ) -> Result<JClassLoader<'local>> {
        let vm = env.get_java_vm();
        let api = JClassAPI::get(&vm)?;

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
}

impl JObjectRef for JClass<'_> {
    const FIND_CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java/lang/Class");
    const LOAD_CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.Class");
    const CLASS_KIND: ClassKind = ClassKind::Bootstrap;

    type Kind<'env> = JClass<'env>;
    type GlobalKind = JClass<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'vm>(vm: &'vm JavaVM, _loader_source: LoaderSource) -> Option<ClassRef<'vm>> {
        // As a special-case; we ignore loader_source just to be clear that there's no risk of
        // recursion.
        let api = JClassAPI::get(vm).ok()?;
        Some(api.map(|api| &api.class))
    }

    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JClass::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JClass::from_raw(global_ref)
    }
}
