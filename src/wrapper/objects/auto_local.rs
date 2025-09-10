use std::{marker::PhantomData, mem::ManuallyDrop, ops::Deref, ptr};

use jni_sys::jobject;

use crate::{
    errors,
    objects::{GlobalRef, JClass, JObject, LoaderContext},
    strings::JNIStr,
    JavaVM,
};

use super::JObjectRef;

/// Auto-delete wrapper for local references.
///
/// Anything passed to a foreign method or returned from JNI methods that refers
/// to a `JObject` is considered a local reference.
///
/// JNI local references belong to a JNI stack frame (the top frame at the time
/// they are created). These reference are normally automatically deleted once
/// the the stack frame unwinds, such as when a foreign method implementation
/// returns back to the JavaVM.
///
/// In some situations you don't want to wait until the current stack frame
/// unwinds before you delete a local reference because you might be creating a
/// very large number of temporary references in a loop that can lead to running
/// out of memory if they aren't explicitly deleted before the stack frame
/// unwinds.
///
/// This wrapper type provides automatic local reference deletion when it goes
/// out of scope.
///
/// See also the [JNI specification][spec-references] for details on referencing
/// Java objects and some [extra information][android-jni-references].
///
/// Note that it's often not necessary, or even recommended, to create an
/// `AutoLocal` wrapper for local JNI references if you can instead rely on the
/// references being deleted when the current JNI stack frame unwinds.
///
/// It is usually more efficient to rely on stack frame unwinding to release
/// local references in bulk instead of creating `AutoLocal` wrappers that
/// are then deleted one-by-one.
///
/// If you aren't sure whether it's OK to create new local references in the
/// current JNI frame (perhaps because you don't know when it will unwind)
/// you can also consider using APIs like `Env::with_local_frame()` which
/// can run your code in a temporary stack frame that will release all local
/// references in bulk, without needing to use `AutoLocal`.
///
/// [spec-references]:
///     https://docs.oracle.com/en/java/javase/12/docs/specs/jni/design.html#referencing-java-objects
/// [android-jni-references]:
///     https://developer.android.com/training/articles/perf-jni#local-and-global-references
#[derive(Debug)]
pub struct AutoLocal<'local, T>
where
    T: Into<JObject<'local>>,
{
    obj: ManuallyDrop<T>,
    _lifetime: PhantomData<&'local T>,
}

impl<'local, T> AutoLocal<'local, T>
where
    // Note that this bound prevents `AutoLocal` from wrapping a `GlobalRef`, which implements
    // `AsRef<JObject<'static>>` but *not* `Into<JObject<'static>>`. This is good, because trying
    // to delete a global reference as though it were local would cause undefined behavior.
    T: Into<JObject<'local>>,
{
    /// Creates a new auto-delete wrapper for a local ref.
    ///
    /// Once this wrapper goes out of scope, the `delete_local_ref` will be
    /// called on the object. While wrapped, the object can be accessed via
    /// the `Deref` impl.
    pub fn new(obj: T) -> Self {
        AutoLocal {
            obj: ManuallyDrop::new(obj),
            _lifetime: PhantomData,
        }
    }

    /// Unwrap the RAII, auto-delete wrapper, returning the original local
    /// reference.
    ///
    /// This prevents the local reference from being automatically deleted when
    /// this guard is dropped, and the local reference will instead get deleted
    /// when the JNI local frame holding the reference gets unwound.
    ///
    /// # Leaking References
    ///
    /// When unwrapping an `AutoLocal` you should consider how else you will
    /// ensure that the local reference will get released.
    ///
    /// If you are implementing a native method then you may not need to keep
    /// and AutoLocal wrapper since you can assume that when you return back to
    /// the Java VM then the local JNI stack frame will unwind and delete all
    /// local references.
    ///
    /// Another option can be to use `Env::with_local_frame` or similar APIs
    /// that create a temporary JNI local frame where you can assume that all
    /// local references will be deleted when that local frame is unwound, after
    /// the given closure is called.
    ///
    pub fn unwrap(self) -> T {
        // We need to move `self.obj` out of `self`. Normally that's trivial, but moving out of a
        // type with a `Drop` implementation is not allowed. We'll have to do it manually (and
        // carefully) with `unsafe`.
        //
        // This could be done without `unsafe` by adding `where T: Default` and using
        // `std::mem::replace` to extract `self.obj`, but doing it this way avoids unnecessarily
        // running the drop routine on `self`.

        // Before we mutilate `self`, make sure its drop code will not be automatically run. That
        // would cause undefined behavior.
        let self_md = ManuallyDrop::new(self);

        unsafe {
            // Move `obj` out of `self` and return it.
            //
            // Safety: The `&mut` proves that `self_md.obj` is valid and not aliased. It is not
            // accessed again after this point. It is wrapped inside `ManuallyDrop`, and will
            // therefore not be dropped after it is moved.
            ptr::read(&*self_md.obj)
        }
    }

    /// Unwrap the RAII, auto-delete wrapper, returning the original local reference.
    ///
    /// See [`Self::unwrap`]
    #[deprecated = "Renamed to AutoLocal::unwrap"]
    pub fn forget(self) -> T {
        self.unwrap()
    }
}

impl<'local, T> Drop for AutoLocal<'local, T>
where
    T: Into<JObject<'local>>,
{
    fn drop(&mut self) {
        // Extract the local reference from `self.obj` so that we can delete it.
        //
        // This is needed because it is not allowed to move out of `self` during drop. A safe
        // alternative would be to wrap `self.obj` in `Option`, but that would incur a run-time
        // performance penalty from constantly checking if it's `None`.
        //
        // Safety: `self.obj` is not used again after this `take` call.
        let obj = unsafe { ManuallyDrop::take(&mut self.obj) };
        let obj: JObject = obj.into();

        // Null Objects are a special case that don't need to be explicitly deleted
        // and since they can also have a `'static` lifetime then we can't assume
        // the current thread is attached when dropping null references.
        if !obj.is_null() {
            let Ok(vm) = JavaVM::singleton() else {
                // Since we wrap a non-null reference with a lifetime associated with a JNI stack
                // frame we can (mostly) assume that JavaVM::singleton() must have been initialized
                // in order to get a Env reference.
                //
                // The only (remote) exception to this is that the reference came from a native
                // method argument and for some reason an AutoLocal wrapper was created before an
                // AttachGuard was created to access a Env reference (which would initialize the
                // JavaVM singleton).
                //
                // This would be a redundant thing to try, but just to err on the side of caution we
                // avoid panicking here and only log an error.
                log::error!("Failed to drop AutoLocal: No JavaVM initialized");
                // In this unlikely case it should be fine to return early, since it would be
                // redundant to explicitly delete the local reference of a native method argument.
                return;
            };

            // Panic:
            //
            // Since we have a non-null local reference associated with a JNI stack frame lifetime
            // we know that the thread is attached and so `with_env_current_frame()` can't return an
            // error.
            vm.with_env_current_frame(|env| -> errors::Result<()> {
                env.delete_local_ref(obj);
                Ok(())
            })
            .expect("Infallible"); // The closure is infallible
        }
    }
}

impl<'local, T, U> AsRef<U> for AutoLocal<'local, T>
where
    T: AsRef<U> + Into<JObject<'local>>,
{
    fn as_ref(&self) -> &U {
        self.obj.as_ref()
    }
}

impl<'local, T> Deref for AutoLocal<'local, T>
where
    T: Into<JObject<'local>>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

/// A trait for wrapping a local reference type into an [`AutoLocal`]
pub trait IntoAutoLocal<'local>: Sized + Into<JObject<'local>> {
    /// Wraps the local reference type into an auto-delete [`AutoLocal`] that will
    /// automatically delete the local reference when it is dropped
    fn auto(self) -> AutoLocal<'local, Self> {
        AutoLocal::new(self)
    }
}

impl<'local, T> IntoAutoLocal<'local> for T where T: Into<JObject<'local>> {}

impl<'local, T> From<T> for AutoLocal<'local, T>
where
    T: Into<JObject<'local>>,
{
    fn from(value: T) -> Self {
        AutoLocal::new(value)
    }
}

// SAFETY: Kind and GlobalKind are implicitly transparent wrappers if T is
// implemented correctly / safely.
unsafe impl<'local, T> JObjectRef for AutoLocal<'local, T>
where
    T: JObjectRef + Into<JObject<'local>>,
{
    const CLASS_NAME: &'static JNIStr = T::CLASS_NAME;

    type Kind<'env> = T::Kind<'env>;
    type GlobalKind = T::GlobalKind;

    fn as_raw(&self) -> jobject {
        self.obj.as_raw()
    }

    fn lookup_class<'vm>(
        vm: &'vm JavaVM,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = GlobalRef<JClass<'static>>> + 'vm> {
        T::lookup_class(vm, loader_context)
    }

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        T::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        T::from_global_raw(global_ref)
    }
}
