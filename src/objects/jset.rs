use std::{borrow::Cow, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::Result,
    objects::{Cast, Global, JClass, JCollection, JIterator, JObject, LoaderContext},
    strings::JNIStr,
    sys::jobject,
    DEFAULT_LOCAL_FRAME_CAPACITY,
};

use super::Reference;

#[cfg(doc)]
use crate::errors::Error;

/// A `java.util.Set` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
#[repr(transparent)]
#[derive(Debug, Default)]
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
        unsafe { JCollection::kind_from_raw(other.into_raw()) }
    }
}

struct JSetAPI {
    class: Global<JClass<'static>>,
}

impl JSetAPI {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JSET_API: OnceCell<JSetAPI> = OnceCell::new();
        JSET_API.get_or_try_init(|| {
            env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
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
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.util.Set`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'local` lifetime.
    pub unsafe fn from_raw<'local_inner>(
        env: &Env<'local_inner>,
        raw: jobject,
    ) -> JSet<'local_inner> {
        JSet(JObject::from_raw(env, raw))
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JSet` always has the `'static` lifetime.
    pub const fn null() -> JSet<'static> {
        JSet(JObject::null())
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw()
    }

    /// Cast a local reference to a [`JSet`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.util.Set`.
    ///
    /// Also see these other options for casting local or global references to a [`JSet`]:
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
        env: &mut Env<'_>,
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
        env: &mut Env<'_>,
    ) -> Result<bool> {
        self.as_collection().remove(element, env)
    }

    /// Removes all of the elements from this set.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the clear operation is not supported
    pub fn clear(&self, env: &mut Env<'_>) -> Result<()> {
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
    pub fn contains(&self, element: &JObject, env: &mut Env<'_>) -> Result<bool> {
        self.as_collection().contains(element, env)
    }

    /// Returns the number of elements in this set.
    pub fn size(&self, env: &mut Env<'_>) -> Result<i32> {
        self.as_collection().size(env)
    }

    /// Returns `true` if this set contains no elements.
    pub fn is_empty(&self, env: &mut Env<'_>) -> Result<bool> {
        self.as_collection().is_empty(env)
    }

    /// Returns an iterator (`java.util.Iterator`) over the elements in this set.
    pub fn iterator<'env_local>(&self, env: &mut Env<'env_local>) -> Result<JIterator<'env_local>> {
        self.as_collection().iterator(env)
    }
}

// SAFETY: JSet is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JSet<'_> {
    type Kind<'env> = JSet<'env>;
    type GlobalKind = JSet<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.util.Set"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JSetAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JSet(JObject::kind_from_raw(local_ref))
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JSet(JObject::global_kind_from_raw(global_ref))
    }
}
