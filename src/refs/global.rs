use std::{borrow::Cow, ops::Deref};

use jni_sys::jobject;
use log::{debug, warn};

use crate::{
    env::Env,
    errors::Result,
    objects::{JClass, JObject, LoaderContext},
    strings::JNIStr,
    sys, JavaVM,
};

#[cfg(doc)]
use crate::objects::Weak;

use super::Reference;

// Note: `Global` must not implement `Into<JObject>`! If it did, then it would be possible to
// wrap it in `Auto`, which would cause undefined behavior upon drop as a result of calling
// the wrong JNI function to delete the reference.

/// A global reference to a Java object.
///
/// Global references are slower to create and delete than ordinary local
/// references, but have several properties that distinguish them:
///
/// * Global references are not bound to the lifetime of a [`Env`].
///
/// * Global references are not bound to any particular thread; they have the
///   [`Send`] and [`Sync`] traits.
///
/// * Until a global reference is dropped, it will prevent the referenced Java
///   object from being garbage collected.
///
/// * It takes more time to create or delete a global reference than to create
///   or delete a local reference.
///
/// These properties make global references useful in a few specific situations:
///
/// * When you need to keep a reference to the same Java object across multiple
///   invocations of a native method, especially if you need a guarantee that
///   it's the exact same object every time, one way to do it is by storing a
///   global reference to it in a Rust `static` variable.
///
/// * When you need to send a Java object reference to a different thread, or
///   use a Java object reference from several different threads at the same
///   time, a global reference can be used to do so.
///
/// * When you need a Java object to not be garbage collected too soon, because
///   some side effect will happen (via `java.lang.Object::finalize`,
///   `java.lang.ref.Cleaner`, or the like) when it is garbage collected, a
///   global reference can be used to prevent it from being garbage collected.
///   (This hold is released when the global reference is dropped.)
///
/// See also [`Weak`], a global reference that does *not* prevent the
/// underlying Java object from being garbage collected.
///
///
/// # Creating and Deleting
///
/// To create a global reference, use the [`Env::new_global_ref`] method. To
/// delete it, simply drop the `Global` (but be sure to do so on an attached
/// thread if possible; see the warning below).
///
/// Note that, because global references take more time to create or delete than
/// local references do, they should only be used when their benefits outweigh
/// this drawback. Also note that this performance penalty does not apply to
/// *using* a global reference (such as calling methods on the underlying Java
/// object), only to creation and deletion of the reference.
///
///
/// # Warning: Drop On an Attached Thread If Possible
///
/// When a `Global` is dropped, a JNI call is made to delete the global
/// reference. If this frequently happens on a thread that is not already
/// attached to the JVM, the thread will be temporarily attached using
/// [`JavaVM::attach_current_thread_for_scope`], causing a severe performance
/// penalty.
///
/// To avoid this performance penalty, ensure that `Global`s are only dropped
/// on a thread that is already attached (or never dropped at all).
///
/// In the event that a global reference is dropped on an unattached thread, a
/// message is [logged][log] at [`log::Level::Warn`].
#[repr(transparent)]
#[derive(Debug)]
pub struct Global<T>
where
    T: Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + Reference
        + Send
        + Sync
        + 'static,
{
    obj: T,
}

/// A temporary type alias to sign post that `GlobalRef` has been renamed to `Global`.
#[deprecated(
    since = "0.22.0",
    note = r#"Since 0.22, `GlobalRef` has been renamed to `Global`."#
)]
pub type GlobalRef<T> = Global<T>;

unsafe impl<T> Send for Global<T> where
    T: Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + Reference
        + Send
        + Sync
        + 'static
{
}

unsafe impl<T> Sync for Global<T> where
    T: Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + Reference
        + Send
        + Sync
        + 'static
{
}

impl<T> Default for Global<T>
where
    T: Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + Reference
        + Send
        + Sync
        + 'static,
{
    fn default() -> Self {
        Self::null()
    }
}

impl<T, U> AsRef<U> for Global<T>
where
    T: AsRef<U>
        + Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + Reference
        + Send
        + Sync,
{
    fn as_ref(&self) -> &U {
        self.obj.as_ref()
    }
}

impl<T> Deref for Global<T>
where
    T: Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + Reference
        + Send
        + Sync
        + 'static,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<T> Global<T>
where
    T: Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + Reference
        + Send
        + Sync
        + 'static,
{
    /// Creates a new auto-delete wrapper for the `'static` global reference
    ///
    /// Note: It's more likely that you want to look at the [`Env::new_global_ref`] API instead
    /// of this, since you can't get `'static` reference types through safe APIs.
    ///
    /// The [`Env`] reference here serves as proof that the current thread is attached, which
    /// implies [`JavaVM::singleton()`] is initialized, which is required by the `Drop`
    /// implementation.
    ///
    /// # Safety
    ///
    /// If the given reference is non-null, it must represent a global JNI reference.
    pub unsafe fn new(_env: &Env, obj: T) -> Self {
        Self { obj }
    }

    /// Creates a [`Global`] wrapper for a `null` reference
    ///
    /// This is equivalent [`Global::default()`]
    pub fn null() -> Self {
        Self { obj: T::default() }
    }

    /// Unwrap the RAII, auto-delete wrapper, returning the original global
    /// reference.
    ///
    /// This prevents the global reference from being automatically deleted when
    /// this guard is dropped.
    ///
    /// # Leaking References
    ///
    /// When unwrapping a [`Global`] you should consider how else you will
    /// ensure that the reference will get deleted.
    ///
    /// The global reference may end leaking unless a new [`Global`] wrapper
    /// is create later, or you find some way to call the JNI `DeleteGlobalRef`
    /// API on the raw reference.
    ///
    /// # Safety
    ///
    /// The unwrapped reference type must not be treated like a local reference
    /// which may be difficult to guarantee since Rust doesn't support negative
    /// lifetime bounds for trait implementations.
    ///
    /// For example the returned type will implement `Into<JObject>` which means
    /// it could be wrapped by [`Auto`], which would lead to undefined behavior.
    ///
    /// Reference types with a `'static` lifetime are an unsafe liability that
    /// should not be exposed by-value in the public API because they will implement
    /// `Into<JObject>` and may be treated like local references.
    pub(crate) unsafe fn unwrap(mut self) -> T {
        let obj = std::mem::take(&mut self.obj); // Leave a `Default/null`` reference in self.obj that doesn't need to be deleted
        std::mem::forget(self); // Skip redundant Drop for `Default/null` reference
        obj
    }

    /// Unwrap to the internal global reference
    pub fn into_raw(self) -> sys::jobject {
        // Safety: there's no chance ot treating `obj` as a local reference
        // since it's also immediately unwrapped
        let obj = unsafe { self.unwrap() };
        let obj: JObject = obj.into();
        obj.into_raw()
    }

    /// Borrows a `JObject` referring to the same Java object as this
    /// `Global`.
    ///
    /// This method is zero-cost and does not create a new local reference.
    ///
    /// `Global` also implements <code>[AsRef]&lt;[JObject]&gt;</code>.
    /// That trait's `as_ref` method does the same thing as this method.
    pub fn as_obj(&self) -> &JObject<'static> {
        self.as_ref()
    }
}

impl<T> Drop for Global<T>
where
    T: Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + Reference
        + Send
        + Sync
        + 'static,
{
    fn drop(&mut self) {
        let obj = std::mem::take(&mut self.obj);

        // It's redundant to explicitly call DeleteGlobalRef with a null pointer and we don't
        // assume that a JavaVM has been initialized if we only wrap a 'static null pointer
        if !obj.is_null() {
            // Panic: If we have a non-null reference, we know JavaVM::singleton() must have been
            // initialized (and can't return an error) because ::new() takes a Env reference.
            let vm = JavaVM::singleton().expect("JavaVM singleton uninitialized");
            let res = vm.attach_current_thread_for_scope(
                |env| -> Result<()> {
                    // If the Env is borrowing from an AttachGuard that owns the current thread
                    // attachment that means the thread was not already attached
                    if env.owns_attachment() {
                        warn!("A JNI global reference was dropped on a thread that is not attached. This will cause a performance problem if it happens frequently. For more information, see the documentation for `jni::objects::Global`.");
                    }
                    // Safety: This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
                    unsafe {
                        jni_call_unchecked!(env, v1_1, DeleteGlobalRef, obj.as_raw());
                    }
                    Ok(())
                },
            );

            if let Err(err) = res {
                debug!("error dropping global ref: {:#?}", err);
            }
        }
    }
}

// SAFETY: Kind and GlobalKind are implicitly transparent wrappers if T is
// implemented correctly / safely.
unsafe impl<T> Reference for Global<T>
where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + Reference + Send + Sync,
{
    type Kind<'env> = T::Kind<'env>;
    type GlobalKind = T::GlobalKind;

    fn as_raw(&self) -> jobject {
        self.obj.as_raw()
    }

    fn class_name() -> Cow<'static, JNIStr> {
        T::class_name()
    }

    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller> {
        T::lookup_class(env, loader_context)
    }

    unsafe fn kind_from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        T::kind_from_raw::<'env>(local_ref)
    }

    unsafe fn global_kind_from_raw(global_ref: jobject) -> Self::GlobalKind {
        T::global_kind_from_raw(global_ref)
    }
}

#[test]
fn test_global_ref_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Global<JObject<'static>>>();
}

#[test]
fn test_global_ref_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<Global<JObject<'static>>>();
}
