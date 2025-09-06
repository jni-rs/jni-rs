use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    env::JNIEnv,
    errors::Result,
    objects::{GlobalRef, JClass, JMethodID, JObject, LoaderContext},
    strings::JNIStr,
    sys::{jobject, jstring},
    JavaVM,
};

use super::JObjectRef;

/// Lifetime'd representation of a `jstring`. Just a `JObject` wrapped in a new
/// class.
#[repr(transparent)]
#[derive(Default)]
pub struct JString<'local>(JObject<'local>);

impl<'local> AsRef<JString<'local>> for JString<'local> {
    fn as_ref(&self) -> &JString<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JString<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JString<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JString<'local>> for JObject<'local> {
    fn from(other: JString) -> JObject {
        other.0
    }
}

struct JStringAPI {
    class: GlobalRef<JClass<'static>>,
    intern_method: JMethodID,
}

impl JStringAPI {
    fn get<'any_local>(
        vm: &JavaVM,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JSTRING_API: OnceCell<JStringAPI> = OnceCell::new();
        JSTRING_API.get_or_try_init(|| {
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JString>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();
                let intern_method = env
                    .get_method_id(&class, c"intern", c"()Ljava/lang/String;")
                    .expect("JString.intern method not found");

                Ok(Self {
                    class,
                    intern_method,
                })
            })
        })
    }
}

impl JString<'_> {
    /// Creates a [`JString`] that wraps the given `raw` [`jstring`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jstring) -> Self {
        Self(JObject::from_raw(raw as jobject))
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jstring {
        self.0.into_raw() as jstring
    }

    /// Returns a canonical, interned version of this string.
    pub fn intern<'local>(&self, env: &mut JNIEnv<'local>) -> Result<JString<'local>> {
        let vm = env.get_java_vm();
        let api = JStringAPI::get(&vm, &LoaderContext::None)?;

        // Safety: We know that `intern` is a valid method on `java/lang/String` that has no
        // arguments and it returns a valid `String` instance.
        let interned = unsafe {
            let interned = env
                .call_method_unchecked(
                    self,
                    api.intern_method,
                    crate::signature::ReturnType::Object,
                    &[],
                )?
                .l()?;
            JString::from_raw(interned.into_raw() as jstring)
        };
        Ok(interned)
    }
}

// SAFETY: JString is a transparent JObject wrapper with no Drop side effects
unsafe impl JObjectRef for JString<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.String");

    type Kind<'env> = JString<'env>;
    type GlobalKind = JString<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'vm>(
        vm: &'vm JavaVM,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = GlobalRef<JClass<'static>>> + 'vm> {
        let api = JStringAPI::get(vm, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JString::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JString::from_raw(global_ref)
    }
}
