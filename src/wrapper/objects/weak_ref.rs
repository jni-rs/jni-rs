use std::ops::Deref;

use jni_sys::jobject;
use log::{debug, warn};

use crate::{
    env::JNIEnv,
    errors::{Error, Result},
    objects::{GlobalRef, JObject},
    sys, JavaVM,
};

use super::JObjectRef;

// Note: `WeakRef` must not implement `Into<JObject>`! If it did, then it would be possible to
// wrap it in `AutoLocal`, which would cause undefined behavior upon drop as a result of calling
// the wrong JNI function to delete the reference.

/// A global reference to a Java object that does *not* prevent it from being
/// garbage collected.
///
/// <dfn>Weak global references</dfn> have the same properties as [ordinary
/// “strong” global references][GlobalRef], with one exception: a weak global
/// reference does not prevent the referenced Java object from being garbage
/// collected. In other words, the Java object can be garbage collected even if
/// there is a weak global reference to it.
///
///
/// # Upgrading
///
/// Because the Java object referred to by a weak global reference may be
/// garbage collected at any moment, it cannot be directly used (such as
/// calling methods on the referenced Java object). Instead, it must first be
/// <dfn>upgraded</dfn> to a local or strong global reference, using the
/// [`WeakRef::upgrade_local`] or [`WeakRef::upgrade_global`] method,
/// respectively.
///
/// Both upgrade methods return an [`Option`]. If, when the upgrade method is
/// called, the Java object has not yet been garbage collected, then the
/// `Option` will be [`Some`] containing a newly created strong reference that
/// can be used as normal. If not, the `Option` will be [`None`].
///
/// Upgrading a weak global reference does not delete it. It is only deleted
/// when the `WeakRef` is dropped, and it can be upgraded more than once.
///
///
/// # Creating and Deleting
///
/// To create a weak global reference, use the [`JNIEnv::new_weak_ref`] method.
/// To delete it, simply drop the `WeakRef` (but be sure to do so on an attached
/// thread if possible; see the warning below).
///
/// It is also possible to create a new JNI weak global reference from an
/// existing one. To do that, use the [`WeakRef::clone_in_jvm`] method.
///
///
/// # Warning: Drop On an Attached Thread If Possible
///
/// When a `WeakRef` is dropped, a JNI call is made to delete the global
/// reference. If this frequently happens on a thread that is not already
/// attached to the JVM, the thread will be temporarily attached using
/// [`JavaVM::attach_current_thread_for_scope`], causing a severe performance
/// penalty.
///
/// To avoid this performance penalty, ensure that `WeakRef`s are only dropped
/// on a thread that is already attached (or never dropped at all).
///
/// In the event that a global reference is dropped on an unattached thread, a
/// message is [logged][log] at [`log::Level::Warn`].
#[repr(transparent)]
#[derive(Debug)]
pub struct WeakRef<T>
where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + JObjectRef + Send + Sync,
{
    obj: T,
}

unsafe impl<T> Send for WeakRef<T> where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + JObjectRef + Send + Sync
{
}

unsafe impl<T> Sync for WeakRef<T> where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + JObjectRef + Send + Sync
{
}

impl<T> Default for WeakRef<T>
where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + JObjectRef + Send + Sync,
{
    fn default() -> Self {
        Self::null()
    }
}

impl<T, U> AsRef<U> for WeakRef<T>
where
    T: AsRef<U>
        + Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Default
        + JObjectRef
        + Send
        + Sync,
{
    fn as_ref(&self) -> &U {
        self.obj.as_ref()
    }
}

impl<T> Deref for WeakRef<T>
where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + JObjectRef + Send + Sync,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<T> WeakRef<T>
where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + JObjectRef + Send + Sync,
{
    /// Creates a new auto-delete wrapper for the `'static` weak global reference
    ///
    /// Note: It's more likely that you want to look at the [`JNIEnv::new_weak_ref`] API instead
    /// of this, since you can't get `'static` reference types through safe APIs.
    ///
    /// The [`JNIEnv`] reference here serves as proof that the current thread is attached, which
    /// implies [`JavaVM::singleton()`] is initialized, which is required by the `Drop`
    /// implementation.
    ///
    /// # Safety
    ///
    /// If the given reference is non-null, it must represent a weak global JNI reference.
    pub unsafe fn new(_env: &JNIEnv, obj: T) -> Self {
        Self { obj }
    }

    /// Creates a [`GlobalRef`] wrapper for a `null` reference
    ///
    /// This is equivalent [`WeakRef::default()`]
    ///
    /// A `null` [`WeakRef`] acts as-if the object has been garbage collected
    /// ([`Self::is_garbage_collected()`] will return `true`).
    pub fn null() -> Self {
        Self { obj: T::default() }
    }

    /// Returns the raw JNI weak reference.
    pub fn as_raw(&self) -> sys::jweak {
        self.obj.as_raw()
    }

    /// Creates a new local reference to this object.
    ///
    /// This returns `None` if the object has already been garbage collected, otherwise it returns
    /// `Some(new_local_reference)`.
    ///
    /// If this method returns `Some(r)`, it is guaranteed that the object will not be garbage
    /// collected at least until `r` is deleted or becomes invalid.
    pub fn upgrade_local<'local>(
        &self,
        env: &mut JNIEnv<'local>,
    ) -> Result<Option<T::Kind<'local>>> {
        match env.new_local_ref(self) {
            Ok(local_ref) => Ok(Some(local_ref)),
            Err(Error::ObjectFreed) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Creates a new strong global reference to this object.
    ///
    /// This returns `None` if the object has already been garbage collected, otherwise it returns
    /// `Some(new_local_reference)`.
    ///
    /// If this method returns `Some(r)`, it is guaranteed that the object will not be garbage
    /// collected at least until `r` is deleted or becomes invalid.
    pub fn upgrade_global(&self, env: &JNIEnv) -> Result<Option<GlobalRef<T::GlobalKind>>> {
        match env.new_global_ref(self) {
            Err(Error::ObjectFreed) => Ok(None),
            Err(err) => Err(err),
            Ok(global_ref) => Ok(Some(global_ref)),
        }
    }

    /// Checks if the object referred to by this `WeakRef` has been garbage collected.
    ///
    /// Note that garbage collection can happen at any moment, so a return of `Ok(true)` from this
    /// method does not guarantee that [`WeakRef::upgrade_local`] or [`WeakRef::upgrade_global`]
    /// will succeed.
    ///
    /// This is equivalent to
    /// <code>self.[is_same_object][WeakRef::is_same_object](env, [JObject::null]\())</code>.
    pub fn is_garbage_collected(&self, env: &JNIEnv) -> bool {
        env.is_same_object(self, JObject::null())
    }

    /// Returns true if this weak reference refers to the given object. Otherwise returns false.
    ///
    /// If `object` is [null][JObject::null], then this method is equivalent to
    /// [`WeakRef::is_garbage_collected`]: it returns true if the object referred to by this
    /// `WeakRef` has been garbage collected, or false if the object has not yet been garbage
    /// collected.
    #[deprecated = "Use JNIEnv::is_same_object"]
    pub fn is_same_object<'local, O>(&self, env: &JNIEnv<'local>, object: O) -> bool
    where
        O: AsRef<JObject<'local>>,
    {
        env.is_same_object(self, object)
    }

    /// Returns true if this weak reference refers to the same object as another weak reference.
    /// Otherwise returns false.
    ///
    /// This method will also return true if both weak references refer to an object that has been
    /// garbage collected.
    #[deprecated = "Use JNIEnv::is_same_object"]
    pub fn is_weak_ref_to_same_object(&self, env: &JNIEnv, other: &Self) -> bool {
        env.is_same_object(self, other)
    }

    /// Creates a new weak reference to the same object that this one refers to.
    ///
    /// This method returns `None` if the object has already been garbage collected.
    pub fn clone_in_jvm(&self, env: &mut JNIEnv<'_>) -> Result<Option<WeakRef<T::GlobalKind>>> {
        match env.new_weak_ref(self) {
            Err(Error::ObjectFreed) => Ok(None),
            Err(err) => Err(err),
            Ok(weak_ref) => Ok(Some(weak_ref)),
        }
    }
}

impl<T> Drop for WeakRef<T>
where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + JObjectRef + Send + Sync,
{
    fn drop(&mut self) {
        let obj = std::mem::take(&mut self.obj);

        // It's redundant to explicitly call DeleteWeakGlobalRef with a null pointer and we don't
        // assume that a JavaVM has been initialized if we only wrap a 'static null pointer
        if !obj.is_null() {
            // Panic: If we have a non-null reference, we know JavaVM::singleton() must have been
            // initialized (and can't return an error) because ::new() takes a JNIEnv reference.
            let vm = JavaVM::singleton().expect("JavaVM singleton uninitialized");
            let res = vm.attach_current_thread_for_scope(
                |env| -> Result<()> {
                    // If the JNIEnv is borrowing from an AttachGuard that owns the current thread
                    // attachment that means the thread was not already attached
                    if env.guard().owns_attachment() {
                        warn!("Dropping a WeakRef in a detached thread. Fix your code if this message appears frequently (see the WeakRef docs).");
                    }

                // Safety: This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
                // jni-rs requires JNI_VERSION > 1.2
                unsafe {
                    jni_call_unchecked!(env, v1_2, DeleteWeakGlobalRef, obj.as_raw());
                }
                Ok(())
            });

            if let Err(err) = res {
                debug!("error dropping weak ref: {:#?}", err);
            }
        }
    }
}

impl<T> JObjectRef for WeakRef<T>
where
    T: Into<JObject<'static>> + AsRef<JObject<'static>> + Default + JObjectRef + Send + Sync,
{
    type Kind<'env> = T::Kind<'env>;
    type GlobalKind = T::GlobalKind;

    fn as_raw(&self) -> jobject {
        self.obj.as_raw()
    }

    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        T::from_local_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        T::from_global_raw(global_ref)
    }
}

#[test]
fn test_weak_ref_send() {
    fn assert_send<T: Send>() {}
    assert_send::<WeakRef<JObject<'static>>>();
}

#[test]
fn test_weak_ref_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<WeakRef<JObject<'static>>>();
}
