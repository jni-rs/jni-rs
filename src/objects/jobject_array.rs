use std::{
    any::{Any, TypeId},
    borrow::Cow,
    collections::HashMap,
    ops::Deref,
    sync::{OnceLock, RwLock},
};

use crate::{
    env::Env,
    errors::Result,
    objects::{Global, JClass, JObject, LoaderContext, Reference},
    strings::{JNIStr, JNIString},
    sys::{jobject, jobjectArray},
    JavaVM, DEFAULT_LOCAL_FRAME_CAPACITY,
};

use super::AsJArrayRaw;

#[cfg(doc)]
use crate::errors::Error;

/// A `java.lang.Object[]` wrapper that is tied to a JNI local reference frame.
///
/// See the [`JObject`] documentation for more information about reference
/// wrappers, how to cast them, and local reference frame lifetimes.
///
#[repr(transparent)]
#[derive(Debug, Default)]
pub struct JObjectArray<'local, E: Reference + 'local = JObject<'local>> {
    array: JObject<'local>,
    _marker: std::marker::PhantomData<E>,
}

impl<'local, E: Reference> AsRef<JObjectArray<'local, E>> for JObjectArray<'local, E> {
    fn as_ref(&self) -> &JObjectArray<'local, E> {
        self
    }
}

impl<'local, E: Reference> AsRef<JObject<'local>> for JObjectArray<'local, E> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local, E: Reference> ::std::ops::Deref for JObjectArray<'local, E> {
    type Target = JObject<'local>;

    fn deref(&self) -> &Self::Target {
        &self.array
    }
}

impl<'local, E: Reference> From<JObjectArray<'local, E>> for JObject<'local> {
    fn from(other: JObjectArray<'local, E>) -> JObject<'local> {
        other.array
    }
}

unsafe impl<'local, E: Reference> AsJArrayRaw<'local> for JObjectArray<'local, E> {}

struct JObjectArrayAPI<E: Reference> {
    class: Global<JClass<'static>>,
    _marker: std::marker::PhantomData<E>,
}

// Unlike other Reference types, JObjectArray is generic and we can't simply use
// a static `OnceCell` to cache state since Rust statics can't be generic.
static API_REGISTRY: OnceLock<RwLock<HashMap<TypeId, &'static (dyn Any + Send + Sync)>>> =
    OnceLock::new();

impl<E: Reference + Send + Sync> JObjectArrayAPI<E> {
    fn get<'any_local>(
        env: &Env<'_>,
        loader_context: &LoaderContext<'any_local, '_>,
    ) -> Result<&'static Self> {
        let map = API_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()));
        let tid = TypeId::of::<Self>();

        // Fast path (read-lock)
        if let Some(any_ref) = map.read().unwrap().get(&tid) {
            // Stored as &'static dyn Any; downcast back to &'static JObjectArrayAPI<E>
            return Ok(any_ref
                .downcast_ref::<Self>()
                .expect("TypeId matched but downcast failed"));
        }

        // Slow path: do the class lookup and cache

        // So we can avoid holding any lock while doing (slow) JNI lookups, then
        // in the unlikely case that another thread is also trying to look up
        // the same state then we let them race and keep the first one to finish.

        let created: JObjectArrayAPI<E> = {
            env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| -> Result<_> {
                let class = loader_context.load_class_for_type::<JObjectArray<E>>(false, env)?;
                let class = env.new_global_ref(&class)?;
                Ok(JObjectArrayAPI {
                    class,
                    _marker: std::marker::PhantomData,
                })
            })?
        };

        let mut write = map.write().unwrap();

        // Another thread might have inserted while we acquired the write lock:
        if let Some(any_ref) = write.get(&tid) {
            let api = any_ref
                .downcast_ref::<Self>()
                .expect("TypeId matched but downcast failed");
            return Ok(api);
        }

        // Leak it to get a true 'static reference and erase the type for storage
        let leaked: &'static JObjectArrayAPI<E> = Box::leak(Box::new(created));
        write.insert(tid, leaked as &'static (dyn Any + Send + Sync));

        Ok(leaked)
    }
}

impl<'local, E: Reference + 'local> JObjectArray<'local, E> {
    /// Creates a [`JObjectArray`] that wraps the given `raw` [`jobjectArray`]
    ///
    /// # Safety
    ///
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - `raw` must be an instance of `java.lang.Object[]` where each element is an instance of `E`.
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'env_local` lifetime.
    pub unsafe fn from_raw<'env_local>(
        env: &Env<'env_local>,
        raw: jobjectArray,
    ) -> JObjectArray<'env_local, E::Kind<'env_local>> {
        JObjectArray {
            array: JObject::from_raw(env, raw as jobject),
            _marker: std::marker::PhantomData,
        }
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JObjectArray` always has the `'static` lifetime.
    pub const fn null() -> JObjectArray<'static, E::GlobalKind> {
        JObjectArray {
            array: JObject::null(),
            _marker: std::marker::PhantomData,
        }
    }

    /// Unwrap to the raw jni type.
    pub const fn into_raw(self) -> jobjectArray {
        self.array.into_raw() as jobjectArray
    }

    /// Cast a local reference to a [`JObjectArray<T>`]
    ///
    /// This will do a runtime (`IsInstanceOf`) check that the object is an instance of `T[]`.
    ///
    /// Also see these other options for casting local or global references to a [`JObjectArray<T>`]:
    /// - [Env::as_cast]
    /// - [Env::new_cast_local_ref]
    /// - [Env::cast_local]
    /// - [Env::new_cast_global_ref]
    /// - [Env::cast_global]
    ///
    /// # Errors
    ///
    /// Returns [Error::WrongObjectType] if the `IsInstanceOf` check fails.
    pub fn cast_local<'any_local, O: Reference + 'static>(
        obj: impl Reference + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
        env: &mut Env<'_>,
    ) -> Result<<JObjectArray<'any_local, O> as Reference>::Kind<'any_local>> {
        env.cast_local::<JObjectArray<'any_local, O>>(obj)
    }

    /// Returns the length of the array.
    pub fn len(&self, env: &Env) -> Result<usize> {
        let array = null_check!(
            self.as_raw() as jobjectArray,
            "JObjectArray::len self argument"
        )?;
        let len = unsafe { jni_call_unchecked!(env, v1_1, GetArrayLength, array) } as usize;
        Ok(len)
    }

    /// Returns a local reference to an element of the [`JObjectArray`] `array`.
    pub fn get_element<'env_local>(
        &self,
        index: usize,
        env: &mut Env<'env_local>,
    ) -> Result<E::Kind<'env_local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        assert_eq!(env.level, JavaVM::thread_attach_guard_level());
        let array = null_check!(
            self.as_raw() as jobjectArray,
            "get_object_array_element array argument"
        )?;
        if index > i32::MAX as usize {
            return Err(crate::errors::Error::JniCall(
                crate::errors::JniError::InvalidArguments,
            ));
        }
        unsafe {
            jni_call_check_ex!(env, v1_1, GetObjectArrayElement, array, index as i32)
                .map(|obj| E::kind_from_raw(obj))
        }
    }

    /// Sets an element of the [`JObjectArray`] `array`.
    pub fn set_element<'any_local>(
        &self,
        index: usize,
        value: impl AsRef<E::Kind<'any_local>>,
        env: &Env<'_>,
    ) -> Result<()> {
        let array = null_check!(
            self.as_raw() as jobjectArray,
            "set_object_array_element array argument"
        )?;
        if index > i32::MAX as usize {
            return Err(crate::errors::Error::JniCall(
                crate::errors::JniError::InvalidArguments,
            ));
        }
        unsafe {
            jni_call_check_ex!(
                env,
                v1_1,
                SetObjectArrayElement,
                array,
                index as i32,
                value.as_ref().as_raw()
            )?;
        }
        Ok(())
    }
}

// SAFETY: JObjectArray is a transparent JObject wrapper with no Drop side effects
unsafe impl<'local, E: Reference + 'local> Reference for JObjectArray<'local, E> {
    type Kind<'env>
        = JObjectArray<'env, E::Kind<'env>>
    where
        <E as Reference>::Kind<'env>: 'env;
    type GlobalKind = JObjectArray<'static, E::GlobalKind>;

    fn as_raw(&self) -> jobject {
        self.array.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        let inner = E::class_name();
        let inner = inner.to_str();
        let name = if inner.len() == 1 || inner.starts_with("[") {
            // inner = primitive OR array
            format!("[{inner}")
        } else {
            // inner = object
            format!("[L{inner};")
        };
        let name: JNIString = name.into();
        Cow::Owned(name)
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: &LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        let api = JObjectArrayAPI::<E::GlobalKind>::get(env, loader_context)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JObjectArray {
            array: JObject::kind_from_raw(local_ref),
            _marker: std::marker::PhantomData,
        }
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JObjectArray {
            array: JObject::global_kind_from_raw(global_ref),
            _marker: std::marker::PhantomData,
        }
    }
}
