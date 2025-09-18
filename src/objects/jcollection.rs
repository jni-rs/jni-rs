use std::{borrow::Cow, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClass, JIterator, JMethodID, JObject, JValue, LoaderContext},
    signature::{Primitive, ReturnType},
    strings::JNIStr,
    sys::jobject,
};

use super::Reference;

#[cfg(doc)]
use crate::errors::Error;

/// Wrapper for `java.utils.Map.Entry` references. Provides methods to get the key and value.
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JCollection<'local>(JObject<'local>);

impl<'local> AsRef<JCollection<'local>> for JCollection<'local> {
    fn as_ref(&self) -> &JCollection<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JCollection<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JCollection<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JCollection<'local>> for JObject<'local> {
    fn from(other: JCollection<'local>) -> JObject<'local> {
        other.0
    }
}

struct JCollectionAPI {
    class: Global<JClass<'static>>,
    add_method: JMethodID,
    remove_method: JMethodID,
    clear_method: JMethodID,
    contains_method: JMethodID,
    size_method: JMethodID,
    is_empty_method: JMethodID,
    iterator_method: JMethodID,
}

impl JCollectionAPI {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JCOLLECTION_API: OnceCell<JCollectionAPI> = OnceCell::new();
        JCOLLECTION_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JCollection>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();

                let add_method = env.get_method_id(&class, c"add", c"(Ljava/lang/Object;)Z")?;
                let remove_method =
                    env.get_method_id(&class, c"remove", c"(Ljava/lang/Object;)Z")?;
                let clear_method = env.get_method_id(&class, c"clear", c"()V")?;
                let contains_method =
                    env.get_method_id(&class, c"contains", c"(Ljava/lang/Object;)Z")?;
                let size_method = env.get_method_id(&class, c"size", c"()I")?;
                let is_empty_method = env.get_method_id(&class, c"isEmpty", c"()Z")?;
                let iterator_method =
                    env.get_method_id(&class, c"iterator", c"()Ljava/util/Iterator;")?;

                Ok(Self {
                    class,
                    add_method,
                    remove_method,
                    clear_method,
                    contains_method,
                    size_method,
                    is_empty_method,
                    iterator_method,
                })
            })
        })
    }
}

impl<'local> JCollection<'local> {
    /// Creates a [`JCollection`] that wraps the given `raw` [`jobject`]
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

    /// Cast a local reference to a [`JCollection`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.util.Collection`.
    ///
    /// Also see these other options for casting local or global references to a [`JCollection`]:
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
    ) -> Result<JCollection<'any_local>> {
        env.cast_local::<JCollection>(obj)
    }

    /// Adds the given element to this set if it is not already present
    ///
    /// Returns `true` if the collection was modified. Returns false if the collection already contains the element and
    /// the collection doesn't allow duplicates.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the add operation is not supported
    /// - `ClassCastException` - if the element type isn't compatible with the collection
    /// - `NullPointerException` - if the given element is null and the collection does not allow null values
    /// - `IllegalArgumentException` - if the element has a property that prevents it from being added to this collection
    /// - `IllegalStateException` - if the element cannot be added due to the current state of the collection
    pub fn add<'any_local>(
        &self,
        element: impl AsRef<JObject<'any_local>>,
        env: &mut Env<'_>,
    ) -> Result<bool> {
        let api = JCollectionAPI::get(env, &LoaderContext::default())?;
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.add_method,
                ReturnType::Primitive(Primitive::Boolean),
                &[JValue::from(element.as_ref()).as_jni()],
            )?
        };
        result.z()
    }

    /// Removes the given element from this collection if it is present
    ///
    /// Returns true if the element was contained in the collection and removed, false otherwise.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the remove operation is not supported
    /// - `ClassCastException` - if the element type isn't compatible with the collection
    /// - `NullPointerException` - if the given element is null and the collection does not allow null values
    pub fn remove<'any_local>(
        &self,
        element: impl AsRef<JObject<'any_local>>,
        env: &mut Env<'_>,
    ) -> Result<bool> {
        let api = JCollectionAPI::get(env, &LoaderContext::default())?;
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.remove_method,
                ReturnType::Primitive(Primitive::Boolean),
                &[JValue::from(element.as_ref()).as_jni()],
            )?
        };
        result.z()
    }

    /// Removes all of the elements from this collection.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the clear operation is not supported
    pub fn clear(&self, env: &mut Env<'_>) -> Result<()> {
        let api = JCollectionAPI::get(env, &LoaderContext::default())?;
        unsafe {
            env.call_method_unchecked(
                self,
                api.clear_method,
                ReturnType::Primitive(Primitive::Void),
                &[],
            )?;
        }
        Ok(())
    }

    /// Checks if the given element is present in this set.
    ///
    /// Returns `true` if the element is present, `false` otherwise.
    ///
    /// # Throws
    ///
    /// - `ClassCastException` - if the element type isn't compatible with the set
    /// - `NullPointerException` - if the given element is null and the set does not allow null values
    pub fn contains(&self, element: &JObject, env: &mut Env<'_>) -> Result<bool> {
        let api = JCollectionAPI::get(env, &LoaderContext::default())?;
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.contains_method,
                ReturnType::Primitive(Primitive::Boolean),
                &[JValue::from(element).as_jni()],
            )?
        };
        result.z()
    }

    /// Returns the number of elements in this collection.
    ///
    /// Returns [i32::MAX] if the collection size is too large to be represented as an i32.
    pub fn size(&self, env: &mut Env<'_>) -> Result<i32> {
        let api = JCollectionAPI::get(env, &LoaderContext::default())?;
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.size_method,
                ReturnType::Primitive(Primitive::Int),
                &[],
            )?
        };
        result.i()
    }

    /// Returns `true` if this collection contains no elements.
    pub fn is_empty(&self, env: &mut Env<'_>) -> Result<bool> {
        let api = JCollectionAPI::get(env, &LoaderContext::default())?;
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.is_empty_method,
                ReturnType::Primitive(Primitive::Boolean),
                &[],
            )?
        };
        result.z()
    }

    /// Returns an iterator (`java.util.Iterator`) over the elements in this collection.
    pub fn iterator<'env_local>(&self, env: &mut Env<'env_local>) -> Result<JIterator<'env_local>> {
        let api = JCollectionAPI::get(env, &LoaderContext::default())?;
        unsafe {
            let iterator = env
                .call_method_unchecked(self, api.iterator_method, ReturnType::Object, &[])?
                .l()?;
            Ok(JIterator::from_raw(iterator.into_raw()))
        }
    }
}

// SAFETY: JCollection is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JCollection<'_> {
    type Kind<'env> = JCollection<'env>;
    type GlobalKind = JCollection<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.util.Collection"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JCollectionAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JCollection::from_raw(local_ref)
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JCollection::from_raw(global_ref)
    }
}
