use std::{marker::PhantomData, ops::Deref};

use once_cell::sync::OnceCell;

use crate::{
    errors::Result,
    objects::{Global, JClass, LoaderContext},
    strings::JNIStr,
    sys::jobject,
    Env,
};

use super::JObjectRef;

/// Wrapper around [`jni_sys::jobject`] that adds a lifetime to ensure that
/// the underlying JNI pointer won't be accessible to safe Rust code if the
/// object reference is released.
///
/// It matches C's representation of the raw pointer, so it can be used in any
/// of the extern function argument positions that would take a `jobject`.
///
/// Most other types in the `objects` module deref to this, as they do in the C
/// representation.
///
/// The lifetime `'local` represents the local reference frame that this
/// reference belongs to. See the [`Env`] documentation for more information
/// about local reference frames. If `'local` is `'static`, then this reference
/// does not belong to a local reference frame, that is, it is either null or a
/// [global reference][Global].
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
            let vm = env.get_java_vm();
            vm.with_env_current_frame(|env| {
                // NB: Self::CLASS_NAME is a binary name with dots, not slashes
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
    /// * `raw` must be a valid raw JNI reference (or `null`).
    /// * There must not be any other `JObject` representing the same reference.
    /// * If `raw` represents a local reference then the `'local` lifetime must
    ///   not outlive the JNI stack frame that the local reference was created in.
    /// * Only global, weak global and `null` references may use a `'static` lifetime.
    pub const unsafe fn from_raw(raw: jobject) -> Self {
        Self {
            internal: raw,
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

    /// Creates a new null reference.
    ///
    /// Null references are always valid and do not belong to a local reference frame. Therefore,
    /// the returned `JObject` always has the `'static` lifetime.
    pub const fn null() -> JObject<'static> {
        unsafe { JObject::from_raw(std::ptr::null_mut() as jobject) }
    }
}

impl std::default::Default for JObject<'_> {
    fn default() -> Self {
        Self::null()
    }
}

// SAFETY: JObject is a transparent jobject wrapper with no Drop side effects
unsafe impl JObjectRef for JObject<'_> {
    const CLASS_NAME: &'static JNIStr = JNIStr::from_cstr(c"java.lang.Object");

    type Kind<'env> = JObject<'env>;
    type GlobalKind = JObject<'static>;

    fn as_raw(&self) -> jobject {
        self.as_raw()
    }

    fn lookup_class<'env>(
        env: &'env Env<'_>,
        _loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'env> {
        // As a special-case; we ignore loader_context just to be clear that there's no risk of
        // recursion. (`LoaderContext::load_class` depends on the `JObjectAPI`)
        let api = JObjectAPI::get(env)?;
        Ok(&api.class)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        JObject::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        JObject::from_raw(global_ref)
    }
}
