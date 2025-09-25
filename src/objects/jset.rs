use crate::{
    env::Env,
    errors::Result,
    objects::{Cast, Global, JClass, JCollection, JIterator, JObject},
};

use super::Reference as _;

#[cfg(doc)]
use crate::errors::Error;

impl<'local> From<JSet<'local>> for JCollection<'local> {
    fn from(other: JSet<'local>) -> JCollection<'local> {
        // SAFETY: Any `java.lang.Set` is also a `java.util.Collection`
        unsafe { JCollection::kind_from_raw(other.into_raw()) }
    }
}

struct JSetAPI {
    class: Global<JClass<'static>>,
}

crate::define_reference_type!(
    type = JSet,
    class = "java.util.Set",
    init = |env, class| {
        Ok(Self { class: env.new_global_ref(&class)? })
    }
);

impl<'local> JSet<'local> {
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
