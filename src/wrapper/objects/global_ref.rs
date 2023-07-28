use std::{mem, ops::Deref, sync::Arc};

use log::{debug, warn};

use crate::{errors::Result, objects::JObject, sys, JNIEnv, JNIVersion, JavaVM};

#[cfg(doc)]
use crate::objects::WeakRef;

// Note: `GlobalRef` must not implement `Into<JObject>`! If it did, then it would be possible to
// wrap it in `AutoLocal`, which would cause undefined behavior upon drop as a result of calling
// the wrong JNI function to delete the reference.

/// A global reference to a Java object.
///
/// Global references are slower to create and delete than ordinary local
/// references, but have several properties that distinguish them:
///
/// * Global references are not bound to the lifetime of a [`JNIEnv`].
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
/// These properties make global references useful in a few specific
/// situations:
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
/// See also [`WeakRef`], a global reference that does *not* prevent the
/// underlying Java object from being garbage collected.
///
///
/// # Creating and Deleting
///
/// To create a global reference, use the [`JNIEnv::new_global_ref`] method.
/// To delete it, simply drop the `GlobalRef` (but be sure to do so on an
/// attached thread if possible; see the warning below).
///
/// Note that, because global references take more time to create or delete
/// than local references do, they should only be used when their benefits
/// outweigh this drawback. Also note that this performance penalty does not
/// apply to *using* a global reference (such as calling methods on the
/// underlying Java object), only to creation and deletion of the reference.
///
///
/// # Clone and Drop Behavior
///
/// `GlobalRef` implements [`Clone`] using [`Arc`], making it inexpensive and
/// infallible. If a `GlobalRef` is cloned, the underlying JNI global
/// reference will only be deleted when the last of the clones is dropped.
///
/// It is also possible to create a new JNI global reference from an existing
/// one. Assuming you have a `JNIEnv` named `env` and a `GlobalRef` named `x`,
/// use [`JNIEnv::new_global_ref`] like this:
///
/// ```no_run
/// # use jni::{JNIEnv, objects::GlobalRef};
/// # let mut env: JNIEnv = unimplemented!();
/// # let x: GlobalRef = unimplemented!();
/// # let _ =
/// env.new_global_ref(&x)
/// # ;
/// ```
///
///
/// # Warning: Drop On an Attached Thread If Possible
///
/// When a `GlobalRef` is dropped, a JNI call is made to delete the global
/// reference. If this frequently happens on a thread that is not already
/// attached to the JVM, the thread will be temporarily attached using
/// [`JavaVM::attach_current_thread`], causing a severe performance penalty.
///
/// To avoid this performance penalty, ensure that `GlobalRef`s are only
/// dropped on a thread that is already attached (or never dropped at all).
///
/// In the event that a global reference is dropped on an unattached thread, a
/// message is [logged][log] at [`log::Level::Warn`].

#[derive(Clone, Debug)]
pub struct GlobalRef {
    inner: Arc<GlobalRefGuard>,
}

#[derive(Debug)]
struct GlobalRefGuard {
    obj: JObject<'static>,
    vm: JavaVM,
}

impl AsRef<GlobalRef> for GlobalRef {
    fn as_ref(&self) -> &GlobalRef {
        self
    }
}

impl AsRef<JObject<'static>> for GlobalRef {
    fn as_ref(&self) -> &JObject<'static> {
        self
    }
}

impl Deref for GlobalRef {
    type Target = JObject<'static>;

    fn deref(&self) -> &Self::Target {
        &self.inner.obj
    }
}

impl GlobalRef {
    /// Creates a new wrapper for a global reference.
    ///
    /// # Safety
    ///
    /// Expects a valid raw global reference that should be created with `NewGlobalRef` JNI function.
    pub(crate) unsafe fn from_raw(vm: JavaVM, raw_global_ref: sys::jobject) -> Self {
        GlobalRef {
            inner: Arc::new(GlobalRefGuard::from_raw(vm, raw_global_ref)),
        }
    }

    /// Borrows a `JObject` referring to the same Java object as this
    /// `GlobalRef`.
    ///
    /// This method is zero-cost and does not create a new local reference.
    ///
    /// `GlobalRef` also implements <code>[AsRef]&lt;[JObject]&gt;</code>.
    /// That trait's `as_ref` method does the same thing as this method.
    pub fn as_obj(&self) -> &JObject<'static> {
        self.as_ref()
    }
}

impl GlobalRefGuard {
    /// Creates a new global reference guard. This assumes that `NewGlobalRef`
    /// has already been called.
    const unsafe fn from_raw(vm: JavaVM, obj: sys::jobject) -> Self {
        GlobalRefGuard {
            obj: JObject::from_raw(obj),
            vm,
        }
    }
}

impl Drop for GlobalRefGuard {
    fn drop(&mut self) {
        let raw: sys::jobject = mem::take(&mut self.obj).into_raw();

        let drop_impl = |env: &JNIEnv| -> Result<()> {
            // Safety: This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
            unsafe {
                jni_call_unchecked!(env, v1_1, DeleteGlobalRef, raw);
            }
            Ok(())
        };

        // Safety: we can assume we couldn't have created the global reference in the first place without
        // having already required the JavaVM to support JNI >= 1.4
        let res = match unsafe { self.vm.get_env(JNIVersion::V1_4) } {
            Ok(env) => drop_impl(&env),
            Err(_) => {
                warn!("A JNI global reference was dropped on a thread that is not attached. This will cause a performance problem if it happens frequently. For more information, see the documentation for `jni::objects::GlobalRef`.");
                self.vm
                    .attach_current_thread()
                    .and_then(|env| drop_impl(&env))
            }
        };

        if let Err(err) = res {
            debug!("error dropping global ref: {:#?}", err);
        }
    }
}
