use jni_sys::jobject;
use once_cell::sync::OnceCell;

use crate::{
    errors::*,
    objects::{
        Global, JClass, JIterator, JMethodID, JObject, JSet, JValue, LoaderContext, Reference,
    },
    signature::{Primitive, ReturnType},
    strings::JNIStr,
    Env, DEFAULT_LOCAL_FRAME_CAPACITY,
};

use std::{borrow::Cow, ops::Deref};

/// A `java.util.Map` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JMap<'local>(JObject<'local>);

impl<'local> AsRef<JMap<'local>> for JMap<'local> {
    fn as_ref(&self) -> &JMap<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JMap<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JMap<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JMap<'local>> for JObject<'local> {
    fn from(other: JMap<'local>) -> JObject<'local> {
        other.0
    }
}

struct JMapAPI {
    class: Global<JClass<'static>>,
    get_method: JMethodID,
    put_method: JMethodID,
    remove_method: JMethodID,
    entry_set_method: JMethodID,
}

impl JMapAPI {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JMAP_API: OnceCell<JMapAPI> = OnceCell::new();
        JMAP_API.get_or_try_init(|| {
            env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
                let class = loader_context.load_class_for_type::<JMap>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();

                let get_method =
                    env.get_method_id(&class, c"get", c"(Ljava/lang/Object;)Ljava/lang/Object;")?;
                let put_method = env.get_method_id(
                    &class,
                    c"put",
                    c"(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
                )?;
                let remove_method = env.get_method_id(
                    &class,
                    c"remove",
                    c"(Ljava/lang/Object;)Ljava/lang/Object;",
                )?;
                let entry_set_method =
                    env.get_method_id(&class, c"entrySet", c"()Ljava/util/Set;")?;

                Ok(Self {
                    class,
                    get_method,
                    put_method,
                    remove_method,
                    entry_set_method,
                })
            })
        })
    }
}

impl<'local> JMap<'local> {
    /// Creates a [`JMap`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    ///
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.util.Map`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'env_local` lifetime.
    pub unsafe fn from_raw<'env_local>(env: &Env<'env_local>, raw: jobject) -> JMap<'env_local> {
        JMap(JObject::from_raw(env, raw))
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JMap` always has the `'static` lifetime.
    pub const fn null() -> JMap<'static> {
        JMap(JObject::null())
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw()
    }

    /// Cast a local reference to a [`JMap`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.util.Map`.
    ///
    /// Also see these other options for casting local or global references to a [`JMap`]:
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
    ) -> Result<JMap<'any_local>> {
        env.cast_local::<JMap>(obj)
    }

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

// SAFETY: JMap is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JMap<'_> {
    type Kind<'env> = JMap<'env>;
    type GlobalKind = JMap<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.util.Map"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JMapAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JMap(JObject::kind_from_raw(local_ref))
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JMap(JObject::global_kind_from_raw(global_ref))
    }
}

/// A `java.util.Map.Entry` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JMapEntry<'local>(JObject<'local>);

impl<'local> AsRef<JMapEntry<'local>> for JMapEntry<'local> {
    fn as_ref(&self) -> &JMapEntry<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JMapEntry<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JMapEntry<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JMapEntry<'local>> for JObject<'local> {
    fn from(other: JMapEntry<'local>) -> JObject<'local> {
        other.0
    }
}

struct JMapEntryAPI {
    class: Global<JClass<'static>>,
    get_key_method: JMethodID,
    get_value_method: JMethodID,
    set_value_method: JMethodID,
}

impl JMapEntryAPI {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JMAPENTRY_API: OnceCell<JMapEntryAPI> = OnceCell::new();
        JMAPENTRY_API.get_or_try_init(|| {
            env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
                let class = loader_context.load_class_for_type::<JMapEntry>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();

                let get_key_method =
                    env.get_method_id(&class, c"getKey", c"()Ljava/lang/Object;")?;
                let get_value_method =
                    env.get_method_id(&class, c"getValue", c"()Ljava/lang/Object;")?;
                let set_value_method = env.get_method_id(
                    &class,
                    c"setValue",
                    c"(Ljava/lang/Object;)Ljava/lang/Object;",
                )?;
                Ok(Self {
                    class,
                    get_key_method,
                    get_value_method,
                    set_value_method,
                })
            })
        })
    }
}

impl<'local> JMapEntry<'local> {
    /// Creates a [`JMapEntry`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    ///
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.util.Map.Entry`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'env_local` lifetime.
    pub unsafe fn from_raw<'env_local>(
        env: &Env<'env_local>,
        raw: jobject,
    ) -> JMapEntry<'env_local> {
        JMapEntry(JObject::from_raw(env, raw))
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JMapEntry` always has the `'static` lifetime.
    pub const fn null() -> JMapEntry<'static> {
        JMapEntry(JObject::null())
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobject {
        self.0.into_raw()
    }

    /// Cast a local reference to a [`JMapEntry`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.util.Map.Entry`.
    ///
    /// Also see these other options for casting local or global references to a [`JMapEntry`]:
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
    ) -> Result<JMapEntry<'any_local>> {
        env.cast_local::<JMapEntry>(obj)
    }

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

// SAFETY: JMapEntry is a transparent JObject wrapper with no Drop side effects
unsafe impl Reference for JMapEntry<'_> {
    type Kind<'env> = JMapEntry<'env>;
    type GlobalKind = JMapEntry<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.util.Map$Entry"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JMapEntryAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JMapEntry(JObject::kind_from_raw(local_ref))
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JMapEntry(JObject::global_kind_from_raw(global_ref))
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
