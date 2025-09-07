use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    env::JNIEnv,
    errors::Result,
    objects::{Cast, GlobalRef, JClass, JCollection, JIterator, JObject, LoaderContext},
    strings::JNIStr,
    sys::jobject,
    JavaVM,
};

use super::JObjectRef;

/// Wrapper for `java.utils.Map.Entry` references. Provides methods to get the key and value.
#[repr(transparent)]
#[derive(Default)]
pub struct JSet<'local>(JObject<'local>);

impl<'local> AsRef<JSet<'local>> for JSet<'local> {
    fn as_ref(&self) -> &JSet<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JSet<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JSet<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JSet<'local>> for JObject<'local> {
    fn from(other: JSet<'local>) -> JObject<'local> {
        other.0
    }
}

impl<'local> From<JSet<'local>> for JCollection<'local> {
    fn from(other: JSet<'local>) -> JCollection<'local> {
        // SAFETY: Any `java.lang.Set` is also a `java.util.Collection`
        unsafe { JCollection::from_raw(other.into_raw()) }
    }
}

struct JSetAPI {
    class: GlobalRef<JClass<'static>>,
}

impl JSetAPI {
    fn get<'any_local>(
        vm: &JavaVM,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JSET_API: OnceCell<JSetAPI> = OnceCell::new();
        JSET_API.get_or_try_init(|| {
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JSet>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();

                Ok(Self { class })
            })
        })
    }
}

impl<'local> JSet<'local> {
    /// Creates a [`JSet`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    ///
    /// `raw` may be a null pointer. If `raw` is not a null pointer, then:
    ///
    /// * `raw` must be a valid raw JNI local reference.
    /// * There must not be any other `JObject` representing the same local reference.
    /// * The lifetime `'local` must not outlive the local reference frame that the local reference
    ///   was created in.
    pub const unsafe fn from_raw(raw: jobject) -> Self {
        Self(JObject::from_raw(raw))
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw()
    }

    /// Cast a local reference to a `JSet`
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.util.Set`.
    ///
    /// Also see these other options for casting local or global references to a `JSet`:
    /// - [JNIEnv::new_cast_local_ref]
    /// - [JNIEnv::cast_local]
    /// - [JNIEnv::as_cast_local]
    /// - [JNIEnv::new_cast_global_ref]
    /// - [JNIEnv::cast_global]
    /// - [JNIEnv::as_cast_global]
    ///
    /// # Errors
    ///
    /// Returns [Error::WrongObjectType] if the `IsInstanceOf` check fails.
    pub fn cast_local<'any_local>(
        obj: impl JObjectRef + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
        env: &mut JNIEnv<'_>,
    ) -> Result<JSet<'any_local>> {
        env.cast_local::<JSet>(obj)
    }

    /// Casts this `JSet` to a `JCollection`
    ///
    /// This does not require a runtime type check since any `java.lang.Set` is also a `java.util.Collection`
    pub fn as_collection(&self) -> Cast<'local, '_, JCollection<'local>> {
        // SAFETY: we know that any `java.lang.Set` is also a `java.util.Collection`
        unsafe { Cast::<JCollection>::new_unchecked(self) }
    }

    /// Adds the given element to this set if it is not already present
    ///
    /// Returns `true` if the element was added, `false` if it was already present.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the add operation is not supported
    /// - `ClassCastException` - if the element type isn't compatible with the set
    /// - `NullPointerException` - if the given element is null and the set does not allow null values
    /// - `IllegalArgumentException` - if the element has a property that prevents it from being added to this set
    pub fn add<'any_local>(
        &self,
        element: impl AsRef<JObject<'any_local>>,
        env: &mut JNIEnv<'_>,
    ) -> Result<bool> {
        self.as_collection().add(element, env)
    }

    /// Removes the given element from this set if it is present
    ///
    /// Returns `true` if the element was removed.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the remove operation is not supported
    /// - `ClassCastException` - if the element type isn't compatible with the set
    /// - `NullPointerException` - if the given element is null and the set does not allow null values
    pub fn remove<'any_local>(
        &self,
        element: impl AsRef<JObject<'any_local>>,
        env: &mut JNIEnv<'_>,
    ) -> Result<bool> {
        self.as_collection().remove(element, env)
    }

    /// Removes all of the elements from this set.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the clear operation is not supported
    pub fn clear(&self, env: &mut JNIEnv<'_>) -> Result<()> {
        self.as_collection().clear(env)
    }

    /// Checks if the given element is present in this set.
    ///
    /// Returns `true` if the element is present, `false` otherwise.
    ///
    /// # Throws
    ///
    /// - `ClassCastException` - if the element type isn't compatible with the set
    /// - `NullPointerException` - if the given element is null and the set does not allow null values
    pub fn contains(&self, element: &JObject, env: &mut JNIEnv<'_>) -> Result<bool> {
        self.as_collection().contains(element, env)
    }

    /// Returns the number of elements in this set.
    pub fn size(&self, env: &mut JNIEnv<'_>) -> Result<i32> {
        self.as_collection().size(env)
    }

    /// Returns `true` if this set contains no elements.
    pub fn is_empty(&self, env: &mut JNIEnv<'_>) -> Result<bool> {
        self.as_collection().is_empty(env)
    }

    /// Returns an iterator (`java.util.Iterator`) over the elements in this set.
    pub fn iterator<'env_local>(
        &self,
        env: &mut JNIEnv<'env_local>,
    ) -> Result<JIterator<'env_local>> {
        self.as_collection().iterator(env)
    }
}

// SAFETY: JSet is a transparent JObject wrapper with no Drop side effects
unsafe impl JObjectRef for JSet<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.util.Set");

    type Kind<'env> = JSet<'env>;
    type GlobalKind = JSet<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'vm>(
        vm: &'vm JavaVM,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = GlobalRef<JClass<'static>>> + 'vm> {
        let api = JSetAPI::get(vm, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JSet::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JSet::from_raw(global_ref)
    }
}
