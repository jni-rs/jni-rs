use std::sync::Arc;

use log::{debug, warn};

use crate::{
    errors::Result,
    objects::{GlobalRef, JObject},
    sys, JNIEnv, JNIVersion, JavaVM,
};

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
/// To delete it, simply drop the `WeakRef` (but be sure to do so on an
/// attached thread if possible; see the warning below).
///
///
/// # Clone and Drop Behavior
///
/// `WeakRef` implements [`Clone`] using [`Arc`], making it inexpensive and
/// infallible. If a `WeakRef` is cloned, the underlying JNI weak global
/// reference will only be deleted when the last of the clones is dropped.
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
/// [`JavaVM::attach_current_thread`], causing a severe performance penalty.
///
/// To avoid this performance penalty, ensure that `WeakRef`s are only
/// dropped on a thread that is already attached (or never dropped at all).
///
/// In the event that a global reference is dropped on an unattached thread, a
/// message is [logged][log] at [`log::Level::Warn`].

#[derive(Clone)]
pub struct WeakRef {
    inner: Arc<WeakRefGuard>,
}

struct WeakRefGuard {
    raw: sys::jweak,
    vm: JavaVM,
}

unsafe impl Send for WeakRefGuard {}
unsafe impl Sync for WeakRefGuard {}

impl WeakRef {
    /// Creates a new wrapper for a global reference.
    ///
    /// # Safety
    ///
    /// Expects a valid raw weak global reference that should be created with `NewWeakGlobalRef`
    /// JNI function.
    pub(crate) unsafe fn from_raw(vm: JavaVM, raw: sys::jweak) -> Self {
        WeakRef {
            inner: Arc::new(WeakRefGuard { raw, vm }),
        }
    }

    /// Returns the raw JNI weak reference.
    pub fn as_raw(&self) -> sys::jweak {
        self.inner.raw
    }

    /// Creates a new local reference to this object.
    ///
    /// This object may have already been garbage collected by the time this method is called. If
    /// so, this method returns `Ok(None)`. Otherwise, it returns `Ok(Some(r))` where `r` is the
    /// new local reference.
    ///
    /// If this method returns `Ok(Some(r))`, it is guaranteed that the object will not be garbage
    /// collected at least until `r` is deleted or becomes invalid.
    pub fn upgrade_local<'local>(&self, env: &JNIEnv<'local>) -> Result<Option<JObject<'local>>> {
        // XXX: Don't use env.new_local_ref here because that will treat `null`
        // return values (for non-null objects) as out-of-memory errors
        let r = unsafe {
            JObject::from_raw(jni_call_unchecked!(env, v1_2, NewLocalRef, self.as_raw()))
        };

        // Per JNI spec, `NewLocalRef` will return a null pointer if the object was GC'd.
        //
        // XXX: technically the `null` could also mean that the system is out of memory
        // but we have no way of differentiating that here.
        //
        if r.is_null() {
            Ok(None)
        } else {
            Ok(Some(r))
        }
    }

    /// Creates a new strong global reference to this object.
    ///
    /// This object may have already been garbage collected by the time this method is called. If
    /// so, this method returns `Ok(None)`. Otherwise, it returns `Ok(Some(r))` where `r` is the
    /// new strong global reference.
    ///
    /// If this method returns `Ok(Some(r))`, it is guaranteed that the object will not be garbage
    /// collected at least until `r` is dropped.
    pub fn upgrade_global(&self, env: &JNIEnv) -> Result<Option<GlobalRef>> {
        let r = env.new_global_ref(unsafe { JObject::from_raw(self.as_raw()) })?;

        // Unlike `NewLocalRef`, the JNI spec does *not* guarantee that `NewGlobalRef` will return a
        // null pointer if the object was GC'd, so we'll have to check.
        if env.is_same_object(&r, JObject::null()) {
            Ok(None)
        } else {
            Ok(Some(r))
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
        self.is_same_object(env, JObject::null())
    }

    // The following methods are wrappers around those `JNIEnv` methods that make sense for a weak
    // reference. These methods exist because they use `JObject::from_raw` on the raw pointer of a
    // weak reference. Although this usage is sound, it is `unsafe`. It's also confusing because
    // `JObject` normally represents a strong reference.

    /// Returns true if this weak reference refers to the given object. Otherwise returns false.
    ///
    /// If `object` is [null][JObject::null], then this method is equivalent to
    /// [`WeakRef::is_garbage_collected`]: it returns true if the object referred to by this
    /// `WeakRef` has been garbage collected, or false if the object has not yet been garbage
    /// collected.
    pub fn is_same_object<'local, O>(&self, env: &JNIEnv<'local>, object: O) -> bool
    where
        O: AsRef<JObject<'local>>,
    {
        env.is_same_object(unsafe { JObject::from_raw(self.as_raw()) }, object)
    }

    /// Returns true if this weak reference refers to the same object as another weak reference.
    /// Otherwise returns false.
    ///
    /// This method will also return true if both weak references refer to an object that has been
    /// garbage collected.
    pub fn is_weak_ref_to_same_object(&self, env: &JNIEnv, other: &WeakRef) -> bool {
        self.is_same_object(env, unsafe { JObject::from_raw(other.as_raw()) })
    }

    /// Creates a new weak reference to the same object that this one refers to.
    ///
    /// `WeakRef` implements [`Clone`], which should normally be used whenever a new `WeakRef` to
    /// the same object is needed. However, that only increments an internal reference count and
    /// does not actually create a new weak reference in the JVM. If you specifically need to have
    /// the JVM create a new weak reference, use this method instead of `Clone`.
    ///
    /// This method returns `Ok(None)` if the object has already been garbage collected.
    pub fn clone_in_jvm(&self, env: &JNIEnv) -> Result<Option<WeakRef>> {
        env.new_weak_ref(unsafe { JObject::from_raw(self.as_raw()) })
    }
}

impl Drop for WeakRefGuard {
    fn drop(&mut self) {
        fn drop_impl(env: &JNIEnv, raw: sys::jweak) -> Result<()> {
            // Safety: This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
            // jni-rs requires JNI_VERSION > 1.2
            unsafe {
                jni_call_unchecked!(env, v1_2, DeleteWeakGlobalRef, raw);
            }
            Ok(())
        }

        // Safety: we can assume we couldn't have created the weak reference in the first place without
        // having already required the JavaVM to support JNI >= 1.4
        let res = match unsafe { self.vm.get_env(JNIVersion::V1_4) } {
            Ok(env) => drop_impl(&env, self.raw),
            Err(_) => {
                warn!("Dropping a WeakRef in a detached thread. Fix your code if this message appears frequently (see the WeakRef docs).");
                self.vm
                    .attach_current_thread()
                    .and_then(|env| drop_impl(&env, self.raw))
            }
        };

        if let Err(err) = res {
            debug!("error dropping weak ref: {:#?}", err);
        }
    }
}

#[test]
fn test_weak_ref_send() {
    fn assert_send<T: Send>() {}
    assert_send::<WeakRef>();
}

#[test]
fn test_weak_ref_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<WeakRef>();
}
