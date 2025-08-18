use jni_sys::jobject;

use crate::objects::JObject;

#[cfg(doc)]
use crate::objects::{AutoLocal, GlobalRef, JString};

/// A trait for types that represents a JNI reference (could be local, global or
/// weak global as well as wrapper types like [`AutoLocal`] and [`GlobalRef`])
///
///
/// This makes it possible for APIs like [`JNIEnv::new_global_ref`] to be given
/// a non-static local reference type like [`JString<'local>`] (or an
/// [`AutoLocal`] wrapper) and return a [`GlobalRef`] that is instead
/// parameterized by [`JString<'static>`].
pub trait JObjectRef: Sized {
    /// The generic associated [`Self::Kind`] type corresponds to the underlying
    /// class type (such as [`JObject`] or [`JString`]), parameterized by the
    /// lifetime that indicates whether the type holds a global reference
    /// (`'static`) or a local reference that's tied to a JNI stack frame.
    type Kind<'local>: JObjectRef + Default + Into<JObject<'local>> + AsRef<JObject<'local>>;
    // XXX: the compiler blows up if we try and specify a Send + Sync bound
    // here: "overflow evaluating the requirement..."
    //where
    //    Self::Kind<'static>: Send + Sync;
    //
    // As a workaround, we have a separate associated type

    /// The associated `GlobalKind` type should be equivalent to
    /// `Kind<'static>`, with the additional bound that ensures the type is
    /// `Send + Sync`
    type GlobalKind: JObjectRef
        + Default
        + Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Send
        + Sync;

    /// Returns the underlying, raw [`crate::sys::jobject`] reference.
    fn as_raw(&self) -> jobject;

    /// Returns `true` if this is a `null` object reference
    fn is_null(&self) -> bool {
        self.as_raw().is_null()
    }

    /// Returns `null` reference based on [`Self::Kind`]
    fn null<'any>() -> Self::Kind<'any> {
        Self::Kind::default()
    }

    /// Returns a new reference type based on [`Self::Kind`] for the given `local_ref` that is
    /// tied to the JNI stack frame for the given lifetime.
    ///
    /// # Safety
    ///
    /// The given lifetime must associated with an AttachGuard or a JNIEnv and represent a
    /// JNI stack frame.
    ///
    /// There must not be no other wrapper for the given `local_ref` reference (unless it is
    /// `null`)
    ///
    /// You are responsible to knowing that `Self::Kind` is a suitable wrapper type for the
    /// given `local_ref` reference. E.g. because the `local_ref` came from an `into_raw`
    /// call from the same type.
    ///
    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env>;

    /// Returns a (`'static`) reference type based on [`Self::GlobalKind`] for the given `global_ref`.
    ///
    /// # Safety
    ///
    /// There must not be no other wrapper for the given `global_ref` reference (unless it is
    /// `null`)
    ///
    /// You are responsible to knowing that `Self::GlobalKind` is a suitable wrapper type for the
    /// given `global_ref` reference. E.g. because the `global_ref` came from an `into_raw`
    /// call from the same type.
    ///
    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind;
}

impl<T> JObjectRef for &T
where
    T: JObjectRef,
{
    type Kind<'local> = T::Kind<'local>;
    type GlobalKind = T::GlobalKind;

    fn as_raw(&self) -> jobject {
        (*self).as_raw()
    }

    unsafe fn from_local_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        T::from_local_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        T::from_global_raw(global_ref)
    }
}
