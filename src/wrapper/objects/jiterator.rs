use std::ops::Deref;

use once_cell::sync::OnceCell;

use crate::{
    env::Env,
    errors::{Error, Result},
    objects::{GlobalRef, JClass, JMethodID, JObject, LoaderContext},
    signature::{Primitive, ReturnType},
    strings::JNIStr,
    sys::jobject,
    JavaVM,
};

use super::JObjectRef;

/// Wrapper for `java.utils.Map.Entry` references. Provides methods to get the key and value.
#[repr(transparent)]
#[derive(Default)]
pub struct JIterator<'local>(JObject<'local>);

impl<'local> AsRef<JIterator<'local>> for JIterator<'local> {
    fn as_ref(&self) -> &JIterator<'local> {
        self
    }
}

impl<'local> AsRef<JObject<'local>> for JIterator<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> ::std::ops::Deref for JIterator<'local> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'local> From<JIterator<'local>> for JObject<'local> {
    fn from(other: JIterator<'local>) -> JObject<'local> {
        other.0
    }
}

struct JIteratorAPI {
    class: GlobalRef<JClass<'static>>,
    has_next_method: JMethodID,
    next_method: JMethodID,
    remove_method: JMethodID,
}

impl JIteratorAPI {
    fn get<'any_local>(
        vm: &JavaVM,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        static JITERATOR_API: OnceCell<JIteratorAPI> = OnceCell::new();
        JITERATOR_API.get_or_try_init(|| {
            vm.with_env_current_frame(|env| {
                let class = loader_context.load_class_for_type::<JIterator>(true, env)?;
                let class = env.new_global_ref(&class).unwrap();

                let has_next_method = env.get_method_id(&class, c"hasNext", c"()Z")?;
                let next_method = env.get_method_id(&class, c"next", c"()Ljava/lang/Object;")?;
                let remove_method = env.get_method_id(&class, c"remove", c"()V")?;

                Ok(Self {
                    class,
                    has_next_method,
                    next_method,
                    remove_method,
                })
            })
        })
    }
}

impl<'local> JIterator<'local> {
    /// Creates a [`JIterator`] that wraps the given `raw` [`jobject`]
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

    /// Cast a local reference to a `JIterator`
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `java.util.Iterator`.
    ///
    /// Also see these other options for casting local or global references to a `JIterator`:
    /// - [Env::new_cast_local_ref]
    /// - [Env::cast_local]
    /// - [Env::as_cast_local]
    /// - [Env::new_cast_global_ref]
    /// - [Env::cast_global]
    /// - [Env::as_cast_global]
    ///
    /// # Errors
    ///
    /// Returns [Error::WrongObjectType] if the `IsInstanceOf` check fails.
    pub fn cast_local<'any_local>(
        obj: impl JObjectRef + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
        env: &mut Env<'_>,
    ) -> Result<JIterator<'any_local>> {
        env.cast_local::<JIterator>(obj)
    }

    /// Returns true if the iteration has more elements.
    pub fn has_next(&self, env: &mut Env<'_>) -> Result<bool> {
        let vm = env.get_java_vm();
        let api = JIteratorAPI::get(&vm, &LoaderContext::None)?;
        unsafe {
            env.call_method_unchecked(
                self,
                api.has_next_method,
                ReturnType::Primitive(Primitive::Boolean),
                &[],
            )?
            .z()
        }
    }

    /// Returns the next element in the iteration, if it exists.
    ///
    /// Returns `Some(element)` if the iteration has more elements, or `None` if
    /// it has reached the end.
    ///
    /// This is like [`std::iter::Iterator::next`], but requires a parameter of
    /// type `&mut Env` in order to call into Java.
    ///
    /// Any exceptions thrown are assumed to be a `NoSuchElementException` and
    /// are caught + cleared before returning `None`.
    ///
    /// ## Beware of creating excessive local references in the current JNI stack frame
    ///
    /// This method creates a new local reference. To prevent excessive memory
    /// usage or overflow errors (when called repeatedly in a loop), the local
    /// reference should be deleted using [`Env::delete_local_ref`] or
    /// [`Env::auto_local`] before the next loop iteration. Alternatively, if
    /// the collection is known to have a small, predictable size, the loop could be
    /// wrapped in [`Env::with_local_frame`] to delete all of the local
    /// references at once.
    pub fn next<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<Option<JObject<'env_local>>> {
        let vm = env.get_java_vm();
        let api = JIteratorAPI::get(&vm, &LoaderContext::None)?;
        unsafe {
            match env.call_method_unchecked(self, api.next_method, ReturnType::Object, &[]) {
                Ok(v) => v.l().map(Some),
                Err(Error::JavaException) => {
                    // Assume `NoSuchElementException` is thrown
                    env.exception_clear();
                    Ok(None)
                }
                Err(e) => Err(e),
            }
        }
    }

    /// Removes the current element from the iteration (if supported by the iterator)
    ///
    /// This can only be called once after [Self::next] is called.
    ///
    /// # Throws
    ///
    /// - `UnsupportedOperationException` if the operation is not supported.
    /// - `IllegalStateException` if the iterator is in an invalid state (i.e [Self::next] has not been called).
    pub fn remove(&self, env: &mut Env<'_>) -> Result<()> {
        let vm = env.get_java_vm();
        let api = JIteratorAPI::get(&vm, &LoaderContext::None)?;
        unsafe {
            env.call_method_unchecked(
                self,
                api.remove_method,
                ReturnType::Primitive(Primitive::Void),
                &[],
            )?;
            Ok(())
        }
    }
}

// SAFETY: JIterator is a transparent JObject wrapper with no Drop side effects
unsafe impl JObjectRef for JIterator<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.util.Iterator");

    type Kind<'env> = JIterator<'env>;
    type GlobalKind = JIterator<'static>;

    fn as_raw(&self) -> jobject {
        self.0.as_raw()
    }

    fn lookup_class<'vm>(
        vm: &'vm JavaVM,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = GlobalRef<JClass<'static>>> + 'vm> {
        let api = JIteratorAPI::get(vm, &loader_context)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JIterator::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JIterator::from_raw(global_ref)
    }
}
