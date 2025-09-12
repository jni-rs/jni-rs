use std::{borrow::Cow, ffi::CString, ops::Deref};

use jni_sys::jobject;

use crate::{
    errors::Error,
    objects::{Global, JClass, JClassLoader, JObject, JThread},
    strings::{JNIStr, JNIString},
    Env,
};

#[cfg(doc)]
use crate::objects::{Auto, JString};

/// A trait for types that represents a JNI reference (could be local, global or
/// weak global as well as wrapper types like [`Auto`] and [`Global`])
///
///
/// This makes it possible for APIs like [`Env::new_global_ref`] to be given a
/// non-static local reference type like [`JString<'local>`] (or an [`Auto`]
/// wrapper) and return a [`Global`] that is instead parameterized by
/// [`JString<'static>`].
///
/// # Safety
///
/// The associated `Kind` and `GlobalKind` types must be transparent wrappers
/// around the underlying JNI object reference types (such as `JObject` or
/// `jobject`) and must not have any `Drop` side effects.
pub unsafe trait Reference: Sized {
    /// The generic associated [`Self::Kind`] type corresponds to the underlying
    /// class type (such as [`JObject`] or [`JString`]), parameterized by the
    /// lifetime that indicates whether the type holds a global reference
    /// (`'static`) or a local reference that's tied to a JNI stack frame.
    ///
    /// # Safety
    ///
    /// This must be a transparent `JObject` or `jobject` wrapper type that
    /// has no `Drop` side effects.
    type Kind<'local>: Reference + Default + Into<JObject<'local>> + AsRef<JObject<'local>> + 'local;
    // XXX: the compiler blows up if we try and specify a Send + Sync bound
    // here: "overflow evaluating the requirement..."
    //where
    //    Self::Kind<'static>: Send + Sync;
    //
    // As a workaround, we have a separate associated type

    /// The associated `GlobalKind` type should be equivalent to
    /// `Kind<'static>`, with the additional bound that ensures the type is
    /// `Send + Sync`
    ///
    /// # Safety
    ///
    /// This must be a transparent `JObject` or `jobject` wrapper type that
    /// has no `Drop` side effects.
    type GlobalKind: Reference
        + Default
        + Into<JObject<'static>>
        + AsRef<JObject<'static>>
        + Send
        + Sync
        + 'static;

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

    /// The fully qualified class name of the Java class represented by this
    /// reference.
    ///
    /// The class name is expected to be dot-separated, in the same format as
    /// `Class.getName()` and suitable for passing to `Class.forName()`
    ///
    /// For example: `"com.example.MyClass"`
    ///
    /// Note: this format is very similar to the FindClass naming conventions,
    /// except for the use of dots instead of slashes.
    ///
    /// An array of objects would look like: "[Ljava.lang.Object;" An array of
    /// integers would look like: "[I"
    ///
    /// This returns a `Cow` so that in the common case a `&'static JNIStr`
    /// literal can be returned but for Array types they may compose the name
    /// dynamically.
    ///
    /// There's no guarantee that the name is interned / cached, so it's not
    /// recommended to call this in any fast path, it's mainly intended for use
    /// when first loading a class, or for debugging.
    fn class_name() -> Cow<'static, JNIStr>;

    /// Borrows a global reference to the class implemented by this reference.
    ///
    /// This is used as part of downcasting checks to do a cached lookup of associated class
    /// references - avoiding the cost of repeated FindClass or loadClass calls.
    ///
    /// The implementation is expected to use [`once_cell::sync::OnceCell::get_or_try_init`] to
    /// lookup cached API state, including a `Global<JClass>`.
    ///
    /// In case no class reference is already cached then use [`LoaderContext::load_class`] to
    /// lookup a class reference.
    ///
    fn lookup_class<'caller>(
        env: &Env<'_>,
        loader_context: LoaderContext,
    ) -> crate::errors::Result<impl Deref<Target = Global<JClass<'static>>> + 'caller>;

    /// Returns a new reference type based on [`Self::Kind`] for the given `reference` that is tied
    /// to the specified lifetime.
    ///
    /// # Safety
    ///
    /// There must not be no other owning wrapper for the given `reference` (unless it is `null`)
    /// (as an exception it can be OK to create a temporary, hidden wrapper while borrowing an
    /// original, owning wrapper - e.g. as part of a type cast)
    ///
    /// Local references must have a lifetime that's associated with an AttachGuard or a Env that
    /// limits them to a single JNI stack frame.
    ///
    /// This can also be used to create a borrowed view of a global reference (e.g. as part of a
    /// type cast), which may be associated with a `'static` lifetime only so long as the lifetime of
    /// the view is limited by borrowing from the original global wrapper.
    ///
    /// You are responsible to knowing that `Self::Kind` is a suitable wrapper type for the given
    /// `reference`. E.g. because the `reference` came from an `into_raw` call from the same type.
    unsafe fn from_raw<'env>(reference: jobject) -> Self::Kind<'env>;

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

/// Represents the context that influences how a class may be loaded.
#[derive(Debug, Default)]
pub enum LoaderContext<'any_local, 'a> {
    /// There's no extra context that influences how the class should be loaded, and a default
    /// strategy will be used:
    ///
    /// 1. The Thread context will be used to find a ClassLoader to check via Class.forName
    /// 2. FindClass will be called
    #[default]
    None,
    /// A direct reference to the class loader that should be used (with no fallback to FindClass)
    Loader(&'a JClassLoader<'any_local>),
    /// In case we don't have a direct reference, to a `ClassLoader`, the ClassLoader associated
    /// with this object's Class may be checked
    ///
    /// This is used when downcasting, where we can speculate that the object being
    /// downcast _should_ be associated with the correct `ClassLoader`.
    ///
    /// The search strategy will be:
    /// 1. The Thread context will be used to find a ClassLoader to check via Class.forName
    /// 2. The ClassLoader associated with the object being downcast will be used
    /// 3. FindClass will be called
    FromObject(&'a JObject<'any_local>),
}

impl<'a, 'any_local> LoaderContext<'a, 'any_local> {
    /// Loads the class with the given name using the loader context.
    ///
    /// `name` should be a binary name like `"java.lang.String"` or an array descriptor like
    /// `"[Ljava.lang.String;"`.
    ///
    /// **Note:** that unlike [crate::env::Env::find_class], the name uses **dots instead of
    /// slashes** and should conform to the format that `Class.getName()` returns and that
    /// `Class.forName()` expects.
    ///
    /// **Note**: see [Self::find_class] if you need to load a class by its internal name, with
    /// slashes, instead of dots (compatible with [crate::env::Env::find_class]). If you have a
    /// choice, then prefer using [Self::load_class] because in the common case `Class.forName` will
    /// be called first, and that requires a **binary** name (so you can avoid a format conversion).
    ///
    /// `initialize` indicates whether a newly loaded class should be initialized (has no effect on
    /// already initialized classes).
    ///
    /// Returns a local reference to the loaded class, or a [`Error::ClassNotFound`] error if the
    /// class could not be found.
    ///
    /// The strategy for loading the class depends on the loader context (See [Self]).
    pub fn load_class<'env_local>(
        &self,
        name: &JNIStr,
        initialize: bool,
        env: &mut crate::env::Env<'env_local>,
    ) -> crate::errors::Result<JClass<'env_local>> {
        /// Convert a binary name or array descriptor (like `"java.lang.String"` or
        /// `"[Ljava.lang.String;"`) into an internal name like `"java/lang/String"` or
        /// `"[Ljava/lang/String;"` that can be passed to `FindClass`.
        fn internal_find_class_name<'name>(binary_name: &'name JNIStr) -> Cow<'name, JNIStr> {
            let bytes = binary_name.to_bytes();
            if !bytes.contains(&b'.') {
                Cow::Borrowed(binary_name)
            } else {
                // Convert from dot-notation to slash-notation
                let owned: Vec<u8> = bytes
                    .iter()
                    .map(|&b| if b == b'.' { b'/' } else { b })
                    .collect();
                let cstring = CString::new(owned).unwrap();
                let jni_string = unsafe { JNIString::from_cstring(cstring) };
                Cow::Owned(jni_string)
            }
        }

        fn lookup_tccl_with_catch<'local>(
            env: &mut crate::env::Env<'local>,
        ) -> crate::errors::Result<Option<JClassLoader<'local>>> {
            let current_thread = JThread::current_thread(env)?;
            match current_thread.get_context_class_loader(env) {
                Ok(tccl) => Ok(Some(tccl)),
                Err(Error::JavaException) => {
                    // SecurityException
                    env.exception_clear();
                    Ok(None)
                }
                Err(e) => Err(e),
            }
        }

        fn load_class_with_catch<'any_loader, 'any_local>(
            name: &JNIStr,
            initialize: bool,
            loader: &JClassLoader<'any_loader>,
            env: &mut crate::env::Env<'any_local>,
        ) -> crate::errors::Result<JClass<'any_local>> {
            // May throw ClassNotFoundException
            match JClass::for_name_with_loader(name, initialize, loader, env) {
                Ok(class) => Ok(class),
                Err(Error::JavaException) => {
                    // Assume it's a ClassNotFoundException
                    env.exception_clear();
                    Err(Error::ClassNotFound {
                        name: name.to_string(),
                    })
                }
                Err(e) => Err(e),
            }
        }

        fn find_class<'local>(
            name: &JNIStr,
            env: &mut crate::env::Env<'local>,
        ) -> crate::errors::Result<JClass<'local>> {
            let internal_name = internal_find_class_name(name);
            match env.find_class(&internal_name) {
                Ok(class) => Ok(class),
                Err(Error::NullPtr(_)) => Err(Error::ClassNotFound {
                    name: name.to_string(),
                }),
                Err(e) => Err(e),
            }
        }

        fn lookup_class_with_fallbacks<'local>(
            name: &JNIStr,
            initialize: bool,
            candidate: Option<&JObject>,
            env: &mut crate::env::Env<'local>,
        ) -> crate::errors::Result<JClass<'local>> {
            if let Some(tccl) = lookup_tccl_with_catch(env)? {
                match load_class_with_catch(name, initialize, &tccl, env) {
                    Ok(class) => return Ok(class),
                    Err(Error::ClassNotFound { .. }) => {
                        // Try the next fallback
                    }
                    Err(e) => return Err(e),
                }
            }

            if let Some(candidate) = candidate {
                let candidate_class = env.get_object_class(candidate)?;
                // Doesn't throw exception for missing loader
                let loader = candidate_class.get_class_loader(env)?;
                match load_class_with_catch(name, initialize, &loader, env) {
                    Ok(class) => return Ok(class),
                    Err(Error::ClassNotFound { .. }) => {
                        // Try the next fallback
                    }
                    Err(e) => return Err(e),
                }
            }

            find_class(name, env)
        }

        match self {
            LoaderContext::None => env.with_local_frame_returning_local::<_, JClass, _>(5, |env| {
                lookup_class_with_fallbacks(name, initialize, None, env)
            }),
            LoaderContext::FromObject(candidate) => env
                .with_local_frame_returning_local::<_, JClass, _>(5, |env| {
                    lookup_class_with_fallbacks(name, initialize, Some(candidate), env)
                }),
            LoaderContext::Loader(loader) => load_class_with_catch(name, initialize, loader, env),
        }
    }

    /// Loads the class with the given "internal" name using the loader context.
    ///
    /// This behaves the same as [Self::load_class] except that it uses the so-called "internal"
    /// name format that `FindClass` or [`crate::Env::find_class`] accepts, with slashes instead of
    /// dots.
    ///
    /// If possible, prefer to use [Self::load_class] to avoid a redundant name format conversion.
    pub fn find_class<'env_local>(
        &self,
        name: &JNIStr,
        initialize: bool,
        env: &mut crate::env::Env<'env_local>,
    ) -> crate::errors::Result<JClass<'env_local>> {
        /// Convert an internal name or array descriptor (like `"java/lang/String"` or
        /// `"[Ljava/lang/String;"`) into a binary name like `"java.lang.String"` or
        /// `"[Ljava.lang.String;"` that can be passed to [LoaderContext::load_class].
        fn internal_to_binary_class_name<'name>(internal: &'name JNIStr) -> Cow<'name, JNIStr> {
            let bytes = internal.to_bytes();
            if !bytes.contains(&b'/') {
                Cow::Borrowed(internal)
            } else {
                // Convert from slash-notation to dot-notation
                let owned: Vec<u8> = bytes
                    .iter()
                    .map(|&b| if b == b'/' { b'.' } else { b })
                    .collect();
                let cstring = CString::new(owned).unwrap();
                let jni_string = unsafe { JNIString::from_cstring(cstring) };
                Cow::Owned(jni_string)
            }
        }
        let internal_name = internal_to_binary_class_name(name);
        self.load_class(&internal_name, initialize, env)
    }

    /// Loads the class associated with the `JObjectRef` type `T`, using the given loader context.
    ///
    /// `initialize` indicates whether a newly loaded class should be initialized (has no effect
    /// on already initialized classes).
    ///
    /// Returns a local reference to the loaded class, or a [`Error::ClassNotFound`] error if the
    /// class could not be found.
    ///
    /// The strategy for loading the class depends on the loader context (See [Self]).
    pub fn load_class_for_type<'env_local, T: Reference>(
        &self,
        initialize: bool,
        env: &mut crate::env::Env<'env_local>,
    ) -> crate::errors::Result<JClass<'env_local>> {
        let class_name = T::class_name();
        self.load_class(&class_name, initialize, env)
    }
}

// SAFETY: Kind and GlobalKind are implicitly transparent wrappers if T is
// implemented correctly / safely.
unsafe impl<T> Reference for &T
where
    T: Reference,
{
    type Kind<'local> = T::Kind<'local>;
    type GlobalKind = T::GlobalKind;

    fn as_raw(&self) -> jobject {
        (*self).as_raw()
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

    unsafe fn from_raw<'env>(local_ref: jobject) -> Self::Kind<'env> {
        T::from_raw(local_ref)
    }

    unsafe fn from_global_raw(global_ref: jobject) -> Self::GlobalKind {
        T::from_global_raw(global_ref)
    }
}
