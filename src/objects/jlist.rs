use jni_sys::jobject;
use once_cell::sync::OnceCell;

use crate::{
    errors::*,
    objects::{
        Cast, Global, JClass, JCollection, JIterator, JMethodID, JObject, JObjectRef, JValue,
        LoaderContext,
    },
    signature::{Primitive, ReturnType},
    strings::JNIStr,
    sys::jint,
    Env,
};

use std::ops::Deref;

/// Wrapper for `java.utils.List` references. Provides methods to get, add, and
/// remove elements.
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JList<'local>(JObject<'local>);

impl<'local> AsRef<JList<'local>> for JList<'local> {
    fn as_ref(&self) -> &JList<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JList<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JList<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JList<'local>> for JObject<'local> {
    fn from(other: JList<'local>) -> JObject<'local> {
        other.0
    }
}

impl<'local> From<JList<'local>> for JCollection<'local> {
    fn from(other: JList<'local>) -> JCollection<'local> {
        // SAFETY: Any `java.lang.List` is also a `java.util.Collection`
        unsafe { JCollection::from_raw(other.into_raw()) }
    }
}

struct JListAPI {
    class: Global<JClass<'static>>,
    get_method: JMethodID,
    add_idx_method: JMethodID,
    remove_method: JMethodID,
}

impl JListAPI {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JLIST_API: OnceCell<JListAPI> = OnceCell::new();
        JLIST_API.get_or_try_init(|| {
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JList>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();

                let get_method = env.get_method_id(&class, c"get", c"(I)Ljava/lang/Object;")?;
                let add_idx_method =
                    env.get_method_id(&class, c"add", c"(ILjava/lang/Object;)V")?;
                let remove_method =
                    env.get_method_id(&class, c"remove", c"(I)Ljava/lang/Object;")?;

                Ok(Self {
                    class,
                    get_method,
                    add_idx_method,
                    remove_method,
                })
            })
        })
    }
}

impl<'local> JList<'local> {
    /// Creates a [`JList`] that wraps the given `raw` [`jobject`]
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

    /// Cast a local reference to a [`JList`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.util.List`.
    ///
    /// Also see these other options for casting local or global references to a [`JList`]:
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
        obj: impl JObjectRef + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
        env: &mut Env<'_>,
    ) -> Result<JList<'any_local>> {
        env.cast_local::<JList>(obj)
    }

    /// Cast a local reference to a `JList`
    ///
    /// See [`JList::cast_local`] for more information.
    #[deprecated(
        since = "0.22.0",
        note = "use JList::cast_local instead or Env::new_cast_local_ref/cast_local/as_cast_local or Env::new_cast_global_ref/cast_global/as_cast_global"
    )]
    pub fn from_env<'any_local>(
        obj: impl JObjectRef + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
        env: &mut Env<'_>,
    ) -> Result<JList<'any_local>> {
        env.cast_local::<JList>(obj)
    }

    /// Casts this `JList` to a `JCollection`
    ///
    /// This does not require a runtime type check since any `java.lang.List` is also a `java.util.Collection`
    pub fn as_collection(&self) -> Cast<'local, '_, JCollection<'local>> {
        // SAFETY: we know that any `java.lang.List` is also a `java.util.Collection`
        unsafe { Cast::<JCollection>::new_unchecked(self) }
    }

    /// Look up the value for a key. Returns `Some` if it's found and `None` if
    /// a null pointer would be returned.
    pub fn get<'top_local>(
        &self,
        env: &mut Env<'top_local>,
        idx: jint,
    ) -> Result<Option<JObject<'top_local>>> {
        let api = JListAPI::get(env, &LoaderContext::None)?;
        // SAFETY: We keep the class loaded, and fetched the method ID for this function.
        // The arguments and return type match the method signature
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.get_method,
                ReturnType::Object,
                &[JValue::from(idx).as_jni()],
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

    /// Append an element to the list
    pub fn add(&self, env: &mut Env, value: &JObject) -> Result<bool> {
        self.as_collection().add(value, env)
    }

    /// Insert an element at a specific index
    pub fn insert(&self, env: &mut Env, idx: jint, value: &JObject) -> Result<()> {
        let api = JListAPI::get(env, &LoaderContext::None)?;
        // SAFETY: We keep the class loaded, and fetched the method ID for this function.
        // The arguments and return type match the method signature
        let result = unsafe {
            env.call_method_unchecked(
                self,
                api.add_idx_method,
                ReturnType::Primitive(Primitive::Void),
                &[JValue::from(idx).as_jni(), JValue::from(value).as_jni()],
            )
        };

        let _ = result?;
        Ok(())
    }

    /// Remove an element from the list by index
    ///
    /// Returns the removed element
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the remove operation is not supported
    /// - `IndexOutOfBoundsException` - if the index is out of bounds
    pub fn remove<'other_local_2>(
        &self,
        env: &mut Env<'other_local_2>,
        idx: jint,
    ) -> Result<JObject<'other_local_2>> {
        let api = JListAPI::get(env, &LoaderContext::None)?;
        // SAFETY: We keep the class loaded, and fetched the method ID for this function.
        // The arguments and return type match the method signature
        unsafe {
            env.call_method_unchecked(
                self,
                api.remove_method,
                ReturnType::Object,
                &[JValue::from(idx).as_jni()],
            )?
            .l()
        }
    }

    /// Removes the first occurrence of `value` from this [JList], if it's present.
    ///
    /// Returns `true` if an element was removed.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the remove operation is not supported
    /// - `ClassCastException` - if the element type isn't compatible with the set
    /// - `NullPointerException` - if the given element is null and the set does not allow null values
    pub fn remove_item(&self, env: &mut Env<'_>, value: &JObject) -> Result<bool> {
        self.as_collection().remove(value, env)
    }

    /// Removes all of the elements from this list.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` - if the clear operation is not supported
    pub fn clear(&self, env: &mut Env<'_>) -> Result<()> {
        self.as_collection().clear(env)
    }

    // FIXME: this shouldn't need a mutable Env reference since it doesn't create any
    // new local references that are returned to the caller. Currently it's required
    // because we don't have an alternative to `call_method_unchecked` that takes a shared
    // reference, based on the assertion that the method returns a primitive type.
    /// Get the size of the list
    pub fn size(&self, env: &mut Env) -> Result<jint> {
        self.as_collection().size(env)
    }

    /// Returns `true` if this list is empty.
    pub fn is_empty(&self, env: &mut Env<'_>) -> Result<bool> {
        self.as_collection().is_empty(env)
    }

    /// Pop the last element from the list
    ///
    /// Note that this calls `size()` to determine the last index.
    #[deprecated(
        since = "0.22.0",
        note = "java.util.List has no pop() method. This non-standard utility will be removed from a future version"
    )]
    pub fn pop<'other_local_2>(
        &self,
        env: &mut Env<'other_local_2>,
    ) -> Result<Option<JObject<'other_local_2>>> {
        let size = self.size(env)?;
        if size == 0 {
            return Ok(None);
        }
        self.remove(env, size - 1).map(Some)
    }

    /// Returns an iterator (`java.util.Iterator`) over the elements in this
    /// list.
    ///
    /// The returned iterator does not implement [`std::iter::Iterator`] and
    /// cannot be used with a `for` loop. This is because its `next` method uses
    /// a `&mut Env` to call the Java iterator. Use a `while let` loop instead:
    ///
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::{JList, JObject}};
    /// #
    /// # fn example(env: &mut Env, list: JList) -> Result<()> {
    /// use jni::objects::IntoAuto as _; // for .auto()
    /// let mut iterator = list.iter(env)?;
    ///
    /// while let Some(obj) = iterator.next(env)? {
    ///     let obj = obj.auto(); // Wrap as Auto<T> to avoid leaking while iterating
    ///
    ///     // Do something with `obj` here.
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Each call to `next` creates a new local reference. To prevent excessive
    /// memory usage or overflow errors, the local reference should be deleted
    /// using [`Env::delete_local_ref`] or wrapped with
    /// [`crate::objects::IntoAuto::auto`] before the next loop iteration.
    /// Alternatively, if the list is known to have a small, predictable size,
    /// the loop could be wrapped in [`Env::with_local_frame`] to delete all of
    /// the local references at once.
    pub fn iter<'env_local>(&self, env: &mut Env<'env_local>) -> Result<JIterator<'env_local>> {
        self.as_collection().iterator(env)
    }
}

// SAFETY: JList is a transparent JObject wrapper with no Drop side effects
unsafe impl JObjectRef for JList<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.util.List");

    type Kind<'env> = JList<'env>;
    type GlobalKind = JList<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'env>(
        env: &'env Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'env> {
        let api = JListAPI::get(env, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JList::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JList::from_raw(global_ref)
    }
}
