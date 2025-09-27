use crate::{
    errors::*,
    objects::{
        Global, JClass, JIterator, JMethodID, JObject, JSet, JValue, LoaderContext, Reference,
    },
    signature::{Primitive, ReturnType},
    sys::jobject,
    Env,
};

use std::ops::Deref;

struct JMapAPI {
    class: Global<JClass<'static>>,
    get_method: JMethodID,
    put_method: JMethodID,
    remove_method: JMethodID,
    entry_set_method: JMethodID,
}

crate::define_reference_type!(
    type = JMap,
    class = "java.util.Map",
    init = |env, class| {
        Ok(Self {
            class: env.new_global_ref(class)?,
            get_method: env.get_method_id(class, c"get", c"(Ljava/lang/Object;)Ljava/lang/Object;")?,
            put_method: env.get_method_id(class, c"put", c"(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;")?,
            remove_method: env.get_method_id(class, c"remove", c"(Ljava/lang/Object;)Ljava/lang/Object;")?,
            entry_set_method: env.get_method_id(class, c"entrySet", c"()Ljava/util/Set;")?,
        })
    }
);

impl<'local> JMap<'local> {
    /// Cast a local reference to a `JMap`
    ///
    /// See [`JMap::cast_local`] for more information.
    #[deprecated(
        since = "0.22.0",
        note = "use JMap::cast_local instead or Env::new_cast_local_ref/cast_local/as_cast_local or Env::new_cast_global_ref/cast_global/as_cast_global"
    )]
    pub fn from_env<'any_local>(
        obj: impl Reference + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
        env: &mut Env<'_>,
    ) -> Result<JMap<'any_local>> {
        env.cast_local::<JMap>(obj)
    }

    /// Look up the value for a key. Returns `Some` if it's found and `None` if
    /// a null pointer would be returned.
    pub fn get<'top_local>(
        &self,
        env: &mut Env<'top_local>,
        key: &JObject,
    ) -> Result<Option<JObject<'top_local>>> {
        let api = JMapAPI::get(env, &LoaderContext::None)?;
        // SAFETY: We keep the class loaded, and fetched the method ID for this function.
        // Provided argument is statically known as a JObject/null, rather than another primitive type.
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.get_method,
                ReturnType::Object,
                &[JValue::from(key).as_jni()],
            )
        };

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match e {
                Error::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Look up the value for a key. Returns `Some` with the old value if the
    /// key already existed and `None` if it's a new key.
    pub fn put<'other_local_2>(
        &self,
        env: &mut Env<'other_local_2>,
        key: &JObject,
        value: &JObject,
    ) -> Result<Option<JObject<'other_local_2>>> {
        let api = JMapAPI::get(env, &LoaderContext::None)?;
        // SAFETY: We keep the class loaded, and fetched the method ID for this function.
        // Provided argument is statically known as a JObject/null, rather than another primitive type.
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.put_method,
                ReturnType::Object,
                &[JValue::from(key).as_jni(), JValue::from(value).as_jni()],
            )
        };

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match e {
                Error::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Remove a value from the map. Returns `Some` with the removed value and
    /// `None` if there was no value for the key.
    pub fn remove<'other_local_2>(
        &self,
        env: &mut Env<'other_local_2>,
        key: &JObject,
    ) -> Result<Option<JObject<'other_local_2>>> {
        let api = JMapAPI::get(env, &LoaderContext::None)?;
        // SAFETY: We keep the class loaded, and fetched the method ID for this function.
        // Provided argument is statically known as a JObject/null, rather than another primitive type.
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.remove_method,
                ReturnType::Object,
                &[JValue::from(key).as_jni()],
            )
        };

        match result {
            Ok(val) => Ok(Some(val.l()?)),
            Err(e) => match e {
                Error::NullPtr(_) => Ok(None),
                _ => Err(e),
            },
        }
    }

    /// Get the entry set for the map.
    ///
    /// This returns a [JSet] view of the mappings contained in the map, which can be used to iterate over the key/value pairs.
    ///
    /// Also see [JSet::iterator] and [Self::iter]
    pub fn entry_set<'env_local>(&self, env: &mut Env<'env_local>) -> Result<JSet<'env_local>> {
        let api = JMapAPI::get(env, &LoaderContext::None)?;
        // SAFETY: We keep the class loaded, and fetched the method ID for this function. Arg list is known empty.
        let entry_set = unsafe {
            env.call_method_unchecked(self, api.entry_set_method, ReturnType::Object, &[])
        }?
        .l()?;
        let set = JSet::cast_local(entry_set, env)?;
        Ok(set)
    }

    /// Get key/value iterator for the map. This is done by getting the
    /// `EntrySet` from java and iterating over it.
    ///
    /// The returned iterator does not implement [`std::iter::Iterator`] and
    /// cannot be used with a `for` loop. This is because its `next` method uses
    /// a `&mut Env` to call the Java iterator. Use a `while let` loop instead:
    ///
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::{JMap, JObject}};
    /// #
    /// # fn example(env: &mut Env, map: JMap) -> Result<()> {
    /// use jni::objects::IntoAuto as _; // for .auto()
    /// let mut iterator = map.iter(env)?;
    ///
    /// while let Some(entry) = iterator.next(env)? {
    ///     // Wrap as Auto<T> to avoid leaking while iterating
    ///     let key = entry.key(env)?.auto();
    ///     let value = entry.value(env)?.auto();
    ///
    ///     // Do something with `key` and `value` here.
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Each call to `next` creates two new local references. To prevent
    /// excessive memory usage or overflow error, the local references should be
    /// deleted using [`Env::delete_local_ref`] or wrapped with
    /// [`crate::objects::IntoAuto::auto`] before the next loop iteration.
    /// Alternatively, if the map is known to have a small, predictable size,
    /// the loop could be wrapped in [`Env::with_local_frame`] to delete all of
    /// the local references at once.
    pub fn iter<'env_local>(&self, env: &mut Env<'env_local>) -> Result<JMapIter<'env_local>> {
        let set = self.entry_set(env)?;
        let iterator = set.iterator(env)?;

        Ok(JMapIter { iterator })
    }
}

struct JMapEntryAPI {
    class: Global<JClass<'static>>,
    get_key_method: JMethodID,
    get_value_method: JMethodID,
    set_value_method: JMethodID,
}

crate::define_reference_type!(
    type = JMapEntry,
    class = "java.util.Map$Entry",
    init = |env, class| {
        Ok(Self {
            class: env.new_global_ref(class)?,
            get_key_method: env.get_method_id(class, c"getKey", c"()Ljava/lang/Object;")?,
            get_value_method: env.get_method_id(class, c"getValue", c"()Ljava/lang/Object;")?,
            set_value_method: env.get_method_id(
                class,
                c"setValue",
                c"(Ljava/lang/Object;)Ljava/lang/Object;",
            )?,
        })
    }
);

impl<'local> JMapEntry<'local> {
    /// Get the key of the map entry by calling the `getKey` method.
    ///
    /// # Throws
    ///
    /// May throw `IllegalStateException` if the entry has been removed from the map (depending on implementation)
    pub fn key<'env_local>(&self, env: &mut Env<'env_local>) -> Result<JObject<'env_local>> {
        let api = JMapEntryAPI::get(env, &LoaderContext::None)?;
        unsafe {
            env.call_method_unchecked(self, api.get_key_method, ReturnType::Object, &[])?
                .l()
        }
    }

    /// Get the value of the map entry by calling the `getValue` method.
    ///
    /// # Throws
    ///
    /// May throw `IllegalStateException` if the entry has been removed from the map (depending on implementation)
    pub fn value<'env_local>(&self, env: &mut Env<'env_local>) -> Result<JObject<'env_local>> {
        let api = JMapEntryAPI::get(env, &LoaderContext::None)?;
        unsafe {
            env.call_method_unchecked(self, api.get_value_method, ReturnType::Object, &[])?
                .l()
        }
    }

    /// Set the value of the map entry by calling the `setValue` method.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` if the backing map does not support the put operation
    /// - `ClassCastException` if the value is not of a compatible type
    /// - `NullPointerException` if a null value is given and the backing map doesn't allow storing null values
    /// - `IllegalArgumentException` if the values has a property that prevents it from being stored by the backing map
    /// - May throw `IllegalStateException` if the entry has been removed from the map (depending on implementation)
    pub fn set_value<'any_local, 'env_local>(
        &self,
        value: &JObject<'any_local>,
        env: &mut Env<'env_local>,
    ) -> Result<JObject<'env_local>> {
        let api = JMapEntryAPI::get(env, &LoaderContext::None)?;
        unsafe {
            env.call_method_unchecked(
                self,
                api.set_value_method,
                ReturnType::Primitive(Primitive::Void),
                &[JValue::from(value).as_jni()],
            )?
            .l()
        }
    }
}

/// An iterator over the keys and values in a map. See [`JMap::iter`] for more
/// information.
///
/// This is implemented as a thin wrapper over [`JIterator`] and the only
/// difference is that [JMapIter::next] will yield [JMapEntry] values,
/// (avoiding the need for a runtime type check, compared to using
/// [JIterator::next] followed by [`JMapEntry::cast_local`]).
///
/// This derefs to [`JIterator`].
#[derive(Debug)]
pub struct JMapIter<'iter_local> {
    iterator: JIterator<'iter_local>,
}

impl<'local> Deref for JMapIter<'local> {
    type Target = JIterator<'local>;

    fn deref(&self) -> &Self::Target {
        &self.iterator
    }
}

impl<'local> JMapIter<'local> {
    /// Advances the iterator and returns the next key-value pair in the
    /// `java.util.Map`, or `None` if there are no more objects.
    ///
    /// See [`JMap::iter`] for more information.
    ///
    /// This method creates two new local references. To prevent excessive
    /// memory usage or overflow error, the local references should be deleted
    /// using [`Env::delete_local_ref`] or wrapped with
    /// [`crate::objects::IntoAuto::auto`] before the next loop iteration.
    /// Alternatively, if the map is known to have a small, predictable size,
    /// the loop could be wrapped in [`Env::with_local_frame`] to delete all of
    /// the local references at once.
    ///
    /// This method returns:
    ///
    /// * `Ok(Some(_))`: if there was another key-value pair in the map.
    /// * `Ok(None)`: if there are no more key-value pairs in the map.
    /// * `Err(_)`: if there was an error calling the Java method to get the
    ///   next key-value pair.
    ///
    /// This is like [`std::iter::Iterator::next`], but requires a parameter of
    /// type `&mut Env` in order to call into Java.
    pub fn next<'env_local>(
        &mut self,
        env: &mut Env<'env_local>,
    ) -> Result<Option<JMapEntry<'env_local>>> {
        self.iterator.next(env)?.map_or(Ok(None), |entry| {
            // SAFETY: we know that the entrySet iterator will yield Map.Entry values
            // so we can safely downcast without needing a runtime type check
            let entry = unsafe { JMapEntry::from_raw(env, entry.into_raw()) };
            Ok(Some(entry))
        })
    }
}
