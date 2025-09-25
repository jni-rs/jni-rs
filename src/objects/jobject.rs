use std::{borrow::Cow, marker::PhantomData, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    errors::Result,
    objects::{Global, JClass, LoaderContext},
    strings::JNIStr,
    sys::jobject,
    Env, DEFAULT_LOCAL_FRAME_CAPACITY,
};

use super::Reference;

#[cfg(doc)]
use crate::{objects::JString, refs::Weak};

/// A `java.lang.Object` wrapper that is tied to a JNI local reference frame.
///
/// This is a `#[repr(transparent)]` wrapper around a `jobject` JNI reference.
///
/// Since it is `#[repr(transparent)]`, it can be used to capture references
/// passed to native methods while also associating them with a local reference
/// frame lifetime for the method call.
///
/// # Casting
///
/// Most other types in the `objects` module implement `Into<JObject>` or
/// `AsRef<JObject>` to allow easy upcasting to `JObject`.
///
/// For downcasting (i.e converting to a more specific type), with runtime
/// checks, use one of these APIs:
/// - [Env::as_cast]
/// - [Env::new_cast_local_ref]
/// - [Env::cast_local]
/// - [Env::new_cast_global_ref]
/// - [Env::cast_global]
///
/// or look for a `cast_local` API like [`JString::cast_local`].
///
/// # Local Reference Frame Lifetime
///
/// The lifetime `'local` represents the local reference frame that this
/// reference belongs to. See the [`Env`] documentation for more information
/// about local reference frames.
///
/// The lifetime may be `'static` if the reference has a [`Global`] or [`Weak`]
/// wrapper that indicates that the reference is global or weak (i.e it does not
/// belong to a local reference frame).
///
/// Note that an *owned* `JObject` is always a local reference and will never
/// have the `'static` lifetime. [`Global`] does implement
/// <code>[AsRef]&lt;JObject&lt;'static>></code>, but this only yields a
/// *borrowed* `&JObject<'static>`, never an owned `JObject<'static>`.
///
/// Local references belong to a single thread and are not safe to share across
/// threads. This type implements [`Send`] and [`Sync`] if and only if the
/// lifetime `'local` is `'static`.
#[repr(transparent)]
#[derive(Debug)]
pub struct JObject<'local> {
    internal: jobject,
    lifetime: PhantomData<&'local ()>,
}

unsafe impl Send for JObject<'static> {}
unsafe impl Sync for JObject<'static> {}

impl<'local> AsRef<JObject<'local>> for JObject<'local> {
    fn as_ref(&self) -> &JObject<'local> {
        self
    }
}

impl<'local> AsMut<JObject<'local>> for JObject<'local> {
    fn as_mut(&mut self) -> &mut JObject<'local> {
        self
    }
}

impl ::std::ops::Deref for JObject<'_> {
    type Target = jobject;

    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

struct JObjectAPI {
    class: Global<JClass<'static>>,
    // no methods cached for now
}
impl JObjectAPI {
    fn get(env: &Env<'_>) -> Result<&'static Self> {
        static JOBJECT_API: OnceCell<JObjectAPI> = OnceCell::new();
        JOBJECT_API.get_or_try_init(|| {
            env.with_local_frame(DEFAULT_LOCAL_FRAME_CAPACITY, |env| {
                let class = env.find_class(JNIStr::from_cstr(c"java/lang/Object"))?;
                let class = env.new_global_ref(class)?;
                Ok(JObjectAPI { class })
            })
        })
    }
}

impl JObject<'_> {
    /// Creates a [`JObject`] that wraps the given `raw` [`jobject`]
    ///
    /// # Safety
    ///
    /// - `raw` must be a valid raw JNI local reference (or `null`).
    /// - There must not be any other owning [`Reference`] wrapper for the same reference.
    /// - The local reference must belong to the current thread and not outlive the
    ///   JNI stack frame associated with the [Env] `'local` lifetime.
    pub unsafe fn from_raw<'local>(_env: &Env<'local>, raw: jobject) -> JObject<'local> {
        JObject::kind_from_raw(raw)
    }

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JObject` always has the `'static` lifetime.
    pub const fn null() -> JObject<'static> {
        JObject {
            internal: std::ptr::null_mut(),
            lifetime: PhantomData,
        }
    }

    /// Returns the raw JNI pointer.
    pub const fn as_raw(&self) -> jobject {
        self.internal
    }

    /// Unwrap to the internal jni type.
    pub const fn into_raw(self) -> jobject {
        self.internal
    }
}

impl std::default::Default for JObject<'_> {
    fn default() -> Self {
        Self::null()
    }
}

// SAFETY: JObject is a transparent jobject wrapper with no Drop side effects
unsafe impl Reference for JObject<'_> {
    type Kind<'env> = JObject<'env>;
    type GlobalKind = JObject<'static>;

    fn as_raw(&self) -> jobject {
        self.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        Cow::Borrowed(JNIStr::from_cstr(c"java.lang.Object"))
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        _loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JObjectAPI`)
        let api = JObjectAPI::get(env)?;
        Ok(&api.class)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JObject {
            internal: local_ref,
            lifetime: PhantomData,
        }
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        JObject {
            internal: global_ref,
            lifetime: PhantomData,
        }
    }
}
