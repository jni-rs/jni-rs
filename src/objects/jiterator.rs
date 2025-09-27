use crate::{
    env::Env,
    errors::{Error, Result},
    objects::{Global, JClass, JMethodID, JObject, LoaderContext},
    signature::{Primitive, ReturnType},
    sys::jobject,
};

struct JIteratorAPI {
    class: Global<JClass<'static>>,
    has_next_method: JMethodID,
    next_method: JMethodID,
    remove_method: JMethodID,
}

crate::define_reference_type!(
    type = JIterator,
    class = "java.util.Iterator",
    init = |env, class| {
        Ok(Self {
            class: env.new_global_ref(class)?,
            has_next_method: env.get_method_id(class, c"hasNext", c"()Z")?,
            next_method: env.get_method_id(class, c"next", c"()Ljava/lang/Object;")?,
            remove_method: env.get_method_id(class, c"remove", c"()V")?,
        })
    }
);
impl<'local> JIterator<'local> {
    /// Returns true if the iteration has more elements.
    pub fn has_next(&self, env: &mut Env<'_>) -> Result<bool> {
        let api = JIteratorAPI::get(env, &LoaderContext::None)?;
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
    /// reference should be deleted using [`Env::delete_local_ref`] or wrapped
    /// with [`crate::objects::IntoAuto::auto`] before the next loop iteration.
    /// Alternatively, if the collection is known to have a small, predictable
    /// size, the loop could be wrapped in [`Env::with_local_frame`] to delete
    /// all of the local references at once.
    pub fn next<'env_local>(
        &self,
        env: &mut Env<'env_local>,
    ) -> Result<Option<JObject<'env_local>>> {
        let api = JIteratorAPI::get(env, &LoaderContext::None)?;
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
        let api = JIteratorAPI::get(env, &LoaderContext::None)?;
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
