use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClass, JIterator, JMethodID, JObject, JValue, LoaderContext},
    signature::{Primitive, ReturnType},
    sys::jobject,
};

#[cfg(doc)]
use crate::errors::Error;

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

crate::define_reference_type!(
    type = JCollection,
    class = "java.util.Collection",
    init = |env, class| {
        Ok(Self {
            class: env.new_global_ref(class)?,
            add_method: env.get_method_id(class, c"add", c"(Ljava/lang/Object;)Z")?,
            remove_method: env.get_method_id(class, c"remove", c"(Ljava/lang/Object;)Z")?,
            clear_method: env.get_method_id(class, c"clear", c"()V")?,
            contains_method: env.get_method_id(class, c"contains", c"(Ljava/lang/Object;)Z")?,
            size_method: env.get_method_id(class, c"size", c"()I")?,
            is_empty_method: env.get_method_id(class, c"isEmpty", c"()Z")?,
            iterator_method: env.get_method_id(class, c"iterator", c"()Ljava/util/Iterator;")?,
        })
    }
);

impl JCollection<'_> {
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
            Ok(JIterator::from_raw(env, iterator.into_raw()))
        }
    }
}
