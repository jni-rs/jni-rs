use std::{
    convert::TryInto,
    marker::PhantomData,
    os::raw::{c_char, c_void},
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    ptr,
    str::FromStr,
    sync::{Mutex, MutexGuard},
};

use jni_sys::jobject;

use crate::{
    descriptors::Desc,
    errors::*,
    objects::{
        Auto, AutoElements, AutoElementsCritical, Global, IntoAuto, JByteBuffer, JClass, JFieldID,
        JList, JMap, JMethodID, JObject, JStaticFieldID, JStaticMethodID, JString, JThrowable,
        JValue, JValueOwned, ReleaseMode, TypeArray, Weak,
    },
    signature::{JavaType, Primitive, TypeSignature},
    strings::{JNIStr, JNIString, MUTF8Chars},
    sys::{
        self, jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort, jsize, jvalue,
        JNINativeMethod,
    },
    JNIVersion, JavaVM,
};
use crate::{
    errors::ErrorPolicy,
    objects::{
        type_array_sealed::TypeArraySealed, Cast, JBooleanArray, JByteArray, JCharArray,
        JDoubleArray, JFloatArray, JIntArray, JLongArray, JObjectArray, JPrimitiveArray,
        JShortArray, LoaderContext,
    },
};
use crate::{objects::AsJArrayRaw, signature::ReturnType};

use super::{objects::Reference, AttachGuard};

/// A non-transparent wrapper around a raw [`sys::JNIEnv`] pointer that provides
/// safe access to the Java Native Interface (JNI) functions.
///
/// Since [Env] is not a type that is publicly constructible, you must obtain a
/// reference, either through a thread attachment, by upgrading an
/// [`EnvUnowned`] within a native method, or via the (unsafe)
/// [`AttachGuard::from_unowned`] APIs.
///
/// See:
/// - [`JavaVM::attach_current_thread`]
/// - [`JavaVM::attach_current_thread_for_scope`]
/// - [`JavaVM::with_env`]
/// - [`JavaVM::with_env_current_frame`]
/// - [`EnvUnowned::with_env`]
///
/// [Env] is a non-transparent wrapper that is not FFI-safe, so it must not be
/// used to try and capture a raw [`sys::JNIEnv`] pointer passed to a native
/// method.
///
/// See [`EnvUnowned`] for a wrapper that is FFI-safe and can be used to capture
/// a `sys::JNIEnv` pointer passed to a native method.
///
/// # Overview of the API
///
/// All the [`Env`] methods aim to follow the JNI specification naming closely
/// enough to make it easier to map between the two. This means that method
/// names, parameter types, and return types will generally be consistent with
/// the JNI specification.
///
/// JNI APIs that can logically be thought of as methods, such as
/// `GetObjectArrayElement`, can be found as methods on the relevant Rust type,
/// in this case [`JObjectArray::get_element`].
///
/// # Exception handling
///
/// Since we're calling into the JVM with this, many methods also have the
/// potential to cause an exception to get thrown. If this is the case, an `Err`
/// result will be returned with the error kind [`Error::JavaException`]. Note
/// that this will _not_ clear the exception - it's up to the caller to decide
/// whether to do so or to let it continue being thrown.
///
/// # References and Lifetimes
///
/// As in C JNI, interactions with Java objects happen through
/// <dfn>references</dfn>, either local or global.
///
/// This crate provides various safe wrappers around JNI references, such as
/// [`JObject`], [`JClass`], [`JString`] and [`JList`] which all implement the
/// [`Reference`] trait.
///
/// By default these types represent <dfn>local references</dfn>, and they will
/// be associated with a lifetime that names the local reference frame they
/// belong to.
///
/// So for example `JObject<'local_1>`  and  `JObject<'local_2>` are local
/// reference that are owned by two different local reference frames and
/// although they may both be usable from a higher stack frame, it's not
/// possible to simply move ownership of a local reference from one local
/// reference frame to another.
///
/// A global reference can be represented by a [`Global`], which will wrap one
/// of the above primitive reference types like `Global<JObject<'static>>`. In
/// this case the `'static` lifetime indicates that the reference is not tied to
/// a local reference frame and can be used from any thread.
///
/// So long as there is at least one reference to a Java object, the JVM garbage
/// collector will not reclaim it.
///
/// <dfn>Global references</dfn> exist until deleted. Deletion occurs when the
/// `Global` is dropped.
///
/// <dfn>Local references</dfn> belong to a local reference frame, and exist
/// until [deleted][Env::delete_local_ref] or until the local reference frame is
/// exited. A new <dfn>local reference frame</dfn> is entered when a native
/// method is called from Java, or when Rust code does so explicitly using
/// [`Env::with_local_frame`]. That local reference frame is exited when the
/// native method or `with_local_frame` returns. When a local reference frame is
/// exited, all local references created inside it are deleted.
///
/// Unlike C JNI, this crate creates a separate `Env` for each local reference
/// frame. The associated Rust lifetime `'local` represents that local reference
/// frame. Rust's borrow checker will ensure that local references are not used
/// after their local reference frame exits (which would cause undefined
/// behavior).
///
/// Unlike global references, local references are not deleted when dropped by
/// default. This is for performance: it is faster for the JVM to delete all of
/// the local references in a frame all at once, than to delete each local
/// reference one at a time. However, this can cause a memory leak if the local
/// reference frame remains entered for a long time, such as a long-lasting
/// loop, in which case local references should be deleted explicitly. Local
/// references can be deleted when dropped if desired; use
/// [`Env::delete_local_ref`] or wrap with [`IntoAuto::auto`] to arrange that.
///
/// ## Lifetime Names
///
/// This crate uses the following convention for lifetime names:
///
/// * `'local` is the lifetime of a local reference frame, as described above.
///
/// * `'other_local`, `'other_local_1`, and `'other_local_2` are the lifetimes
///   of some other local reference frame, which may be but doesn't have to be
///   the same as `'local`. For example, [`Env::new_local_ref`] accepts a local
///   reference in any local reference frame `'other_local` and creates a new
///   local reference to the same object in `'local`.
///
/// * `'obj_ref` is the lifetime of a borrow of a JNI reference, like
///   <code>&amp;[JObject]</code> or <code>&amp;[Global]</code>. For example,
///   [`Env::get_list`] constructs a new [`JList`] that borrows a `&'obj_ref
///   JObject`.
///
/// ## `null` Java references
/// `null` Java references are handled by the following rules:
///   - If a `null` Java reference is passed to a method that expects a
///     non-`null` argument, an `Err` result with the kind `NullPtr` is
///     returned.
///   - If a JNI function returns `null` to indicate an error (e.g.
///     `new_int_array`), it is converted to `Err`/`NullPtr` or, where possible,
///     to a more applicable error type, such as `MethodNotFound`. If the JNI
///     function also throws an exception, the `JavaException` error kind will
///     be preferred.
///   - If a JNI function may return `null` Java reference as one of possible
///     reference values (e.g., `get_object_array_element` or
///     `get_field_unchecked`), it is converted to `JObject::null()`.
///
/// # `&self` and `&mut self`
///
/// Many of the methods on this type take a `&mut self` reference if they need
/// to return a new local reference. This is because new local references can
/// only be created in the current (top) local reference frame and this crate
/// needs to be able to name the lifetime of that local reference frame in order
/// to return it safely.
///
/// This crate is designed to take away mutable access to an `Env` when pushing
/// a new local reference frame so you can't accidentally attempt to create a
/// new local reference that would be associated with the wrong frame.
///
/// In some circumstances you may find you only have a shared [Env] reference
/// but need to create some hidden, temporary local references that will not be
/// returned to the caller. In this case you can use [`JavaVM::with_env`] or
/// [`JavaVM::with_env_current_frame`] to materialize a temporary mutable [Env]
/// reference for the top / current local reference frame.
///
/// Beware though that this crate also implements runtime checks as a failsafe
/// to ensure that local references are only created in the top local reference
/// frame.
///
/// If you use an API like [`JavaVM::with_env_current_frame`] to get a mutable
/// [Env] while you already have another mutable [Env] reference for a different
/// local reference frame, then you open up the risk of a panic if you try to
/// create a new local reference using the [Env] that is not associated with the
/// top local reference frame.
///
/// As a rule of thumb (to avoid the risk of runtime check failures) you should
/// avoid any attempt to materialize a mutable reference to an [`Env`] instance
/// if you already have a mutable [Env] reference.
///
/// See [issue #392] for background discussion on this topic.
///
/// [issue #392]: https://github.com/jni-rs/jni-rs/issues/392
///
/// ## `cannot borrow as mutable`
///
/// If a function takes two or more parameters, one of them is `Env`, and
/// another is something returned by a `Env` method (like [`JObject`]), then
/// calls to that function may not compile:
///
/// ```rust,compile_fail
/// # use jni::{errors::Result, Env, objects::*};
/// #
/// # fn f(env: &mut Env) -> Result<()> {
/// fn example_function(
///     env: &mut Env,
///     obj: &JObject,
/// ) {
///     // …
/// }
///
/// example_function(
///     env,
///     // ERROR: cannot borrow `*env` as mutable more than once at a time
///     &env.new_object(
///         c"com/example/SomeClass",
///         c"()V",
///         &[],
///     )?,
/// )
/// # ; Ok(())
/// # }
/// ```
///
/// To fix this, the `Env` parameter needs to come *last*:
///
/// ```rust,no_run
/// # use jni::{errors::Result, Env, objects::*};
/// #
/// # fn f(env: &mut Env) -> Result<()> {
/// fn example_function(
///     obj: &JObject,
///     env: &mut Env,
/// ) {
///     // …
/// }
///
/// example_function(
///     &env.new_object(
///         c"com/example/SomeClass",
///         c"()V",
///         &[],
///     )?,
///     env,
/// )
/// # ; Ok(())
/// # }
/// ```
///
/// # Checked and unchecked methods
///
/// Some of the methods come in two versions: checked (e.g. `call_method`) and
/// unchecked (e.g. `call_method_unchecked`). Under the hood, checked methods
/// perform some checks to ensure the validity of provided signatures, names and
/// arguments, and then call the corresponding unchecked method.
///
/// Checked methods are more flexible as they allow passing class names and
/// method/field descriptors as strings and may perform lookups of class objects
/// and method/field ids for you, also performing all the needed precondition
/// checks. However, these lookup operations are expensive, so if you need to
/// call the same method (or access the same field) multiple times, it is
/// [recommended](https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html#accessing-fields-and-methods)
/// to cache the instance of the class and the method/field id, e.g.
///   - in loops
///   - when calling the same Java callback repeatedly.
///
/// If you do not cache references to classes and method/field ids, you will
/// *not* benefit from the unchecked methods.
///
/// Calling unchecked methods with invalid arguments and/or invalid class and
/// method descriptors may lead to segmentation fault.
///
/// # Zero-copy `AsRef<JNIStr>` arguments
///
/// The [`Env`] JNI methods that take string arguments can often accept types
/// that implement the [`AsRef<JNIStr>`] trait and allow for zero-copy
/// conversions of `CStr` literals like `c"java/lang/String"`
///
/// Under the hood, the JNI specification for many functions requires that
/// string arguments are passed as NUL terminated, Modified UTF-8 encoded byte
/// arrays.
///
/// For the majority of the JNI functions there is no practical difference
/// between Modified UTF-8 and standard UTF-8 (for example class names and
/// method names and signatures) and you can rely on a `const` validation at
/// compile time that a given `CStr` literal is valid Modified UTF-8 string via
/// [`JNIStr::from_cstr`].
///
/// Anything that accepts an `AsRef<JNIStr>` can directly accept a `CStr`
/// literal like `c"java/lang/String"` or `c"(I)Ljava/lang/String;"` and will be
/// validated via [JNIStr::from_cstr]. Beware that validation failure will
/// simply panic but in practice there should be no reason for these arguments
/// to require non-UTF-8 strings.
///
/// For more complex strings that need full unicode support (for example when
/// calling [`Env::new_string`] then you should use [JNIString::from] instead on
/// relying on `CStr` literals.
///
#[derive(Debug)]
pub struct Env<'local> {
    /// A non-null JNIEnv pointer
    pub(crate) raw: *mut sys::JNIEnv,
    /// The current [`jni::AttachGuard`] nesting level, which we assert matches
    /// the top/current nesting level whenever some API will return a new local
    /// reference
    pub(crate) level: usize,
    owns_attachment: bool,
    _lifetime: PhantomData<&'local ()>,
}

impl Drop for Env<'_> {
    fn drop(&mut self) {
        // NOOP - we just implement Drop so that the compiler won't consider
        // Env to be FFI safe.
    }
}

impl<'local> Env<'local> {
    /// Returns an `UnsupportedVersion` error if the current JNI version is
    /// lower than the one given.
    #[allow(unused)]
    fn ensure_version(&self, version: JNIVersion) -> Result<()> {
        if self.version() < version {
            Err(Error::UnsupportedVersion)
        } else {
            Ok(())
        }
    }

    /// Create a [`Env`] associated with single JNI local reference frame.
    ///
    /// `level` represents the nesting level of the [`AttachGuard`] that owns
    /// this [`Env`]. This can be compared to [JavaVM::thread_attach_guard_level()]
    /// to check that this [`Env`] is associated with the top-most JNI stack frame.
    ///
    /// `owns_attachment` should be true if this [`Env`] is associated with an
    /// attachment that will detach the thread when dropped.
    pub(crate) unsafe fn new(raw: *mut sys::JNIEnv, level: usize, owns_attachment: bool) -> Self {
        let env = Env {
            raw,
            level,
            owns_attachment,
            _lifetime: PhantomData,
        };
        // Assuming that the application doesn't break the safety rules for
        // keeping the `AttachGuard` on the stack, and not re-ordering them,
        // we can assert that we only ever create an `Env` for the top-most
        // guard on the stack.
        env.assert_top();
        env
    }

    /// Runtime check that this [`Env`] represents the top JNI stack frame
    ///
    /// Any lower-level API that returns a new local reference must call this
    /// method to ensure the reference is tied to the correct JNI stack frame.
    ///
    /// All safe APIs that return a new local reference already call this but you
    /// may need to call this in `unsafe` code that uses [`crate::sys`] functions
    /// directly.
    ///
    /// See the safety documentation for [`AttachGuard`] for more details.
    pub fn assert_top(&self) {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        assert_eq!(self.level, JavaVM::thread_attach_guard_level());
    }

    /// Returns true if this [`Env`] is associated with a scoped attachment that
    /// will also detach the thread when it is dropped.
    ///
    /// An owned attachment would come from a call to
    /// [`JavaVM::attach_current_thread_for_scope`] but only if the thread was
    /// not already attached.
    ///
    /// This can be used to recognise when
    /// [`JavaVM::attach_current_thread_for_scope`] really needed to attach the
    /// thread, or if the thread was already attached.
    ///
    /// This is mostly useful for diagnostic purposes. For example the `Drop`
    /// implementation for [`Global`] and [`Weak`] will print a warning if they
    /// are dropped on a thread that is not attached, which is recognised with
    /// this method.
    pub fn owns_attachment(&self) -> bool {
        self.owns_attachment
    }

    /// Get the raw Env pointer
    pub fn get_raw(&self) -> *mut sys::JNIEnv {
        self.raw
    }

    /// Get the JNI version that this [`Env`] supports.
    pub fn version(&self) -> JNIVersion {
        // Safety: GetVersion is 1.1 API that must be valid
        JNIVersion::from(unsafe { jni_call_unchecked!(self, v1_1, GetVersion) })
    }

    /// Load a class from a buffer of raw class data.
    ///
    /// If `name` is null, the name of the class is inferred from the buffer.
    ///
    /// Note: This requires `&mut` because it returns a new local reference to a class.
    ///
    /// # Safety
    ///
    /// The `buf` pointer must be valid for `buf_len` bytes.
    unsafe fn define_class_impl(
        &mut self,
        name: *const c_char,
        loader: &JObject,
        buf: *const jbyte,
        buf_len: usize,
    ) -> Result<JClass<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        // Safety:
        // DefineClass is 1.1 API that must be valid
        // It is valid to potentially pass a `null` `name` to `DefineClass`, since the
        // name can bre read from the bytecode.
        unsafe {
            jni_call_check_ex_and_null_ret!(
                self,
                v1_1,
                DefineClass,
                name,
                loader.as_raw(),
                buf,
                buf_len as jsize
            )
            .map(|class| JClass::from_raw(class))
        }
    }

    /// Load a class from a buffer of raw class data.
    ///
    /// If `name` is `None` then the name of the loaded class will be inferred, otherwise the given
    /// `name` must match the name encoded within the class file data.
    ///
    /// Alternatively, call `define_class_jbyte()` if your data comes from a [`JByteArray`] or
    /// `&[jbyte]`.
    pub fn define_class<S>(
        &mut self,
        name: Option<S>,
        loader: &JObject,
        buf: &[u8],
    ) -> Result<JClass<'local>>
    where
        S: AsRef<JNIStr>,
    {
        let name: Option<&JNIStr> = name.as_ref().map(|n| n.as_ref());
        let name = name.map_or(ptr::null(), |n| n.as_ptr());
        // Safety: we know the pointer for the u8 slice is valid for buf.len() bytes
        unsafe { self.define_class_impl(name, loader, buf.as_ptr() as *const jbyte, buf.len()) }
    }

    /// Load a class from a buffer of raw class data.
    ///
    /// If `name` is `None` then the name of the loaded class will be inferred, otherwise the given
    /// `name` must match the name encoded within the class file data.
    ///
    /// This is the same as `define_class` but takes a `&[jbyte]` instead of `&[u8]`.
    pub fn define_class_jbyte<S>(
        &mut self,
        name: Option<S>,
        loader: &JObject,
        buf: &[jbyte],
    ) -> Result<JClass<'local>>
    where
        S: AsRef<JNIStr>,
    {
        let name: Option<&JNIStr> = name.as_ref().map(|n| n.as_ref());
        let name = name.as_ref().map_or(ptr::null(), |n| n.as_ptr());
        // Safety: we know the pointer for the u8 slice is valid for buf.len() bytes
        unsafe { self.define_class_impl(name, loader, buf.as_ptr() as *const jbyte, buf.len()) }
    }

    /// Look up a class by its fully-qualified name, via JNI `FindClass`.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::JClass};
    /// #
    /// # fn example<'local>(env: &mut Env<'local>) -> Result<()> {
    /// let class: JClass<'local> = env.find_class(c"java/lang/String")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Returns the loaded class, or a [`Error::NullPtr`] error if the class could not be found.
    ///
    /// ## Use [Reference::lookup_class], instead of [Env::find_class] where possible
    ///
    /// Whenever you need the class associated with some reference wrapper type (e.g. [`JObject`],
    /// [`JClass`], [`JString`] etc), prefer using [Reference::lookup_class] instead of this method.
    ///
    /// [Reference] is a trait that all of these reference wrapper types implement.
    ///
    /// All implementations of [Reference::lookup_class] will maintain a static cache holding a
    /// `Global<JClass>` that is cheap to lookup and doesn't require a JNI call or creating any new
    /// references.
    ///
    /// For example, lookup the class for `java.lang.String` / [JString] like this:
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::JClass};
    /// #
    /// # fn example<'local>(env: &mut Env<'local>) -> Result<()> {
    /// use jni::objects::{JString, Reference as _, LoaderContext};
    /// let string_class = JString::lookup_class(env, LoaderContext::None)?;
    /// let string_class_ref: &JClass = string_class.as_ref();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// ## Consider using [LoaderContext::load_class] in all other cases
    ///
    /// Unless you strictly need to use the JNI `FindClass` API directly to look up classes it may
    /// be best to use [LoaderContext::load_class] or [LoaderContext::find_class] instead of this
    /// method.
    ///
    /// [LoaderContext::load_class] can be given a specific `java.lang.ClassLoader`
    /// ([crate::objects::JClassLoader]) for finding application classes that `FindClass` may not
    /// find.
    ///
    /// Even without providing a `ClassLoader`, [LoaderContext::load_class] will check the context
    /// class loader of the current thread, which makes it possible to associate a thread with an
    /// application class loader. This may be particularly useful for native applications on Android
    /// because native threads will not normally be able to find application classes through
    /// `FindClass`.
    pub fn find_class<S>(&mut self, name: S) -> Result<JClass<'local>>
    where
        S: AsRef<JNIStr>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let name = name.as_ref();
        // Safety:
        // FindClass is 1.1 API that must be valid
        // name is non-null
        unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, FindClass, name.as_ptr())
                .map(|class| JClass::from_raw(class))
        }
    }

    /// Returns the superclass for a particular class. Returns None for `java.lang.Object` or
    /// an interface. As with [Self::find_class], takes a descriptor
    ///
    /// # Errors
    ///
    /// If a JNI call fails
    pub fn get_superclass<'other_local, T>(&mut self, class: T) -> Result<Option<JClass<'local>>>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let class = class.lookup(self)?;
        let superclass = unsafe {
            JClass::from_raw(jni_call_unchecked!(
                self,
                v1_1,
                GetSuperclass,
                class.as_ref().as_raw()
            ))
        };

        Ok((!superclass.is_null()).then_some(superclass))
    }

    // Like is_assignable_from but it doesn't need a mutable Env reference because it doesn't do any
    // descriptor lookups.
    fn is_assignable_from_class(&self, class1: &JClass, class2: &JClass) -> Result<bool> {
        let class1 = null_check!(class1, "is_assignable_from class1")?;
        let class2 = null_check!(class2, "is_assignable_from class2")?;

        // Safety:
        // - IsAssignableFrom is 1.1 API that must be valid
        // - We make sure class1 and class2 can't be null
        unsafe {
            Ok(jni_call_unchecked!(
                self,
                v1_1,
                IsAssignableFrom,
                class1.as_raw(), // MUST not be null
                class2.as_raw()  // MUST not be null
            ))
        }
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Tests whether class1 is assignable from class2.
    pub fn is_assignable_from<'other_local_1, 'other_local_2, T, U>(
        &mut self,
        class1: T,
        class2: U,
    ) -> Result<bool>
    where
        T: Desc<'local, JClass<'other_local_1>>,
        U: Desc<'local, JClass<'other_local_2>>,
    {
        let class1 = class1.lookup(self)?;
        let class1 = null_check!(class1.as_ref(), "is_assignable_from class1")?;
        let class2 = class2.lookup(self)?;
        let class2 = null_check!(class2.as_ref(), "is_assignable_from class2")?;

        self.is_assignable_from_class(class1, class2)
    }

    /// Checks if an object can be cast to a specific reference type.
    pub(crate) fn is_instance_of_cast_type<To: Reference>(&self, obj: &JObject) -> Result<bool> {
        let class = match To::lookup_class(self, LoaderContext::FromObject(obj)) {
            Ok(class) => class,
            Err(Error::ClassNotFound { name: _ }) => return Ok(false),
            Err(e) => return Err(e),
        };

        let class: &JClass = class.as_ref();
        self.is_instance_of_class(obj, class)
    }

    // An internal helper that implements is_instance_of except it doesn't take a
    // Desc for the class and doesn't need a mutable Env reference since it never
    // needs to allocate a new local reference.
    /// Returns true if the object reference can be cast to the given type.
    fn is_instance_of_class<'other_local_1, 'other_local_2, O, C>(
        &self,
        object: O,
        class: C,
    ) -> Result<bool>
    where
        O: AsRef<JObject<'other_local_1>>,
        C: AsRef<JClass<'other_local_2>>,
    {
        let class = null_check!(class.as_ref(), "is_instance_of class")?;

        // Safety:
        // - IsInstanceOf is 1.1 API that must be valid
        // - We make sure class can't be null
        unsafe {
            Ok(jni_call_unchecked!(
                self,
                v1_1,
                IsInstanceOf,
                object.as_ref().as_raw(), // may be null
                class.as_raw()            // MUST not be null
            ))
        }
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Returns true if the object reference can be cast to the given type.
    ///
    /// _NB: Unlike the operator `instanceof`, function `IsInstanceOf` *returns `true`*
    /// for all classes *if `object` is `null`.*_
    ///
    /// See [JNI documentation](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#IsInstanceOf)
    /// for details.
    pub fn is_instance_of<'other_local_1, 'other_local_2, O, T>(
        &mut self,
        object: O,
        class: T,
    ) -> Result<bool>
    where
        O: AsRef<JObject<'other_local_1>>,
        T: Desc<'local, JClass<'other_local_2>>,
    {
        let class = class.lookup(self)?;
        self.is_instance_of_class(object, class)
    }

    /// Returns true if ref1 and ref2 refer to the same Java object, or are both `NULL`. Otherwise,
    /// returns false.
    pub fn is_same_object<'other_local_1, 'other_local_2, O, T>(&self, ref1: O, ref2: T) -> bool
    where
        O: AsRef<JObject<'other_local_1>>,
        T: AsRef<JObject<'other_local_2>>,
    {
        // Safety:
        // - IsSameObject is 1.1 API that must be valid
        // - the spec allows either object reference to be `null`
        unsafe {
            jni_call_unchecked!(
                self,
                v1_1,
                IsSameObject,
                ref1.as_ref().as_raw(), // may be null
                ref2.as_ref().as_raw()  // may be null
            )
        }
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JThrowable>::lookup`)
    //
    /// Raise an exception from an existing object. This will continue being
    /// thrown in java unless `exception_clear` is called.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// env.throw((c"java/lang/Exception", c"something bad happened"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Defaulting to "java/lang/Exception":
    ///
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// env.throw(c"something bad happened")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn throw<'other_local, E>(&mut self, obj: E) -> Result<()>
    where
        E: Desc<'local, JThrowable<'other_local>>,
    {
        let throwable = obj.lookup(self)?;

        // Safety:
        // Throw is 1.1 API that must be valid
        //
        // We are careful to ensure that we don't drop the reference
        // to `throwable` after converting to a raw pointer.
        let res: i32 =
            unsafe { jni_call_unchecked!(self, v1_1, Throw, throwable.as_ref().as_raw()) };

        // Ensure that `throwable` isn't dropped before the JNI call returns.
        drop(throwable);

        if res == 0 {
            Ok(())
        } else {
            Err(Error::ThrowFailed(res))
        }
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    fn throw_new_optional(&self, class: &JClass, msg: Option<&JNIStr>) -> Result<()> {
        let throwable_class = JThrowable::lookup_class(self, LoaderContext::None)?;
        let throwable_class: &JClass = throwable_class.as_ref();

        if !self.is_assignable_from_class(class.as_ref(), throwable_class)? {
            return Err(Error::WrongObjectType);
        }
        let msg = msg.as_ref().map(|m| m.as_ref());

        // Safety:
        // ThrowNew is 1.1 API that must be valid
        //
        // We are careful to ensure that we don't drop the reference
        // to `class` or `msg` after converting to raw pointers.
        let res: i32 = unsafe {
            jni_call_unchecked!(
                self,
                v1_1,
                ThrowNew,
                class.as_raw(),
                msg.map(|m| m.as_ptr()).unwrap_or(std::ptr::null())
            )
        };

        if res == 0 {
            Ok(())
        } else {
            Err(Error::ThrowFailed(res))
        }
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Create and throw a new exception from a class descriptor and an error
    /// message.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// env.throw_new(c"java/lang/Exception", c"something bad happened")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Alternatively, see [Env::throw_new_void] if you want to construct an exception
    /// with no message argument.
    pub fn throw_new<'other_local, S, T>(&mut self, class: T, msg: S) -> Result<()>
    where
        S: AsRef<JNIStr>,
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let msg: &JNIStr = msg.as_ref();
        self.throw_new_optional(class.as_ref(), Some(msg))
    }

    /// Create and throw a new exception from a class descriptor and no error
    /// message.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// env.throw_new_void(c"java/lang/Exception")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// This will expect to find a constructor for the given `class` that takes no arguments.
    ///
    /// Alternatively, see [Env::throw_new] if you want to construct an exception
    /// with a message.
    pub fn throw_new_void<'other_local, T>(&mut self, class: T) -> Result<()>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        self.throw_new_optional(class.as_ref(), None)
    }

    /// Returns true if an exception is currently in the process of being thrown.
    ///
    /// This doesn't need to create any local references
    #[inline]
    pub fn exception_check(&self) -> bool {
        // Safety: ExceptionCheck is 1.2 API, which we check for in `from_raw()`
        unsafe { jni_call_unchecked!(self, v1_2, ExceptionCheck) }
    }

    /// Check whether or not an exception is currently in the process of being
    /// thrown.
    ///
    /// An exception is in this state from the time it gets thrown and
    /// not caught in a java function until `exception_clear` is called.
    pub fn exception_occurred(&mut self) -> Option<JThrowable<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let throwable = unsafe { jni_call_unchecked!(self, v1_1, ExceptionOccurred) };
        if throwable.is_null() {
            None
        } else {
            Some(unsafe { JThrowable::from_raw(throwable) })
        }
    }

    /// Print exception information to the console.
    pub fn exception_describe(&self) {
        // Safety: ExceptionDescribe is 1.1 API that must be valid
        unsafe { jni_call_unchecked!(self, v1_1, ExceptionDescribe) };
    }

    /// Clear an exception in the process of being thrown. If this is never
    /// called, the exception will continue being thrown when control is
    /// returned to java.
    pub fn exception_clear(&self) {
        // Safety: ExceptionClear is 1.1 API that must be valid
        unsafe { jni_call_unchecked!(self, v1_1, ExceptionClear) };
    }

    /// Abort the JVM with an error message.
    ///
    /// This method is guaranteed not to panic, call any JNI function other
    /// than [`FatalError`], or perform any heap allocations (although
    /// `FatalError` might perform heap allocations of its own).
    ///
    /// In exchange for these strong guarantees, this method requires an error
    /// message to already be suitably encoded, as described in the
    /// documentation for the [`JNIStr`] type.
    ///
    /// The simplest way to use this is to convert an ordinary Rust string to a
    /// [`JNIString`], like so:
    ///
    /// ```no_run
    /// # use jni::{Env, strings::JNIString};
    /// # let env: Env = unimplemented!();
    /// env.fatal_error(&JNIString::from("Game over, man! Game over!"))
    /// ```
    ///
    /// This can also be used in a way that's completely guaranteed to be
    /// panic- and allocation-free, but it is somewhat complicated and
    /// `unsafe`:
    ///
    /// ```no_run
    /// # use jni::{Env, strings::JNIStr};
    /// # use std::ffi::CStr;
    /// const MESSAGE: &JNIStr = unsafe {
    ///     JNIStr::from_cstr_unchecked(
    ///         CStr::from_bytes_with_nul_unchecked(
    ///             b"Game over, man! Game over!\0"
    ///         )
    ///     )
    /// };
    ///
    /// # let env: Env = unimplemented!();
    /// env.fatal_error(MESSAGE)
    /// ```
    ///
    /// When doing this, be careful not to forget the `\0` at the end of the
    /// string, and to correctly encode non-ASCII characters according to
    /// Java's [Modified UTF-8].
    ///
    /// [`FatalError`]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/functions.html#fatalerror
    /// [Modified UTF-8]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/types.html#modified-utf-8-strings
    pub fn fatal_error(&self, msg: &JNIStr) -> ! {
        // Safety: FatalError is 1.1 API that must be valid
        //
        // Very little is specified about the implementation of FatalError but we still
        // currently consider this "safe", similar to how `abort()` is considered safe.
        // It won't give the application an opportunity to clean or save state but the
        // process will be terminated.
        unsafe { jni_call_unchecked!(self, v1_1, FatalError, msg.as_ptr()) }
    }

    /// Create a new instance of a direct java.nio.ByteBuffer
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let buf = vec![0; 1024 * 1024];
    /// let (addr, len) = { // (use buf.into_raw_parts() on nightly)
    ///     let buf = buf.leak();
    ///     (buf.as_mut_ptr(), buf.len())
    /// };
    /// let direct_buffer = unsafe { env.new_direct_byte_buffer(addr, len) }?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Safety
    ///
    /// Expects a valid (non-null) pointer and length
    ///
    /// Caller must ensure the lifetime of `data` extends to all uses of the returned
    /// `ByteBuffer`. The JVM may maintain references to the `ByteBuffer` beyond the lifetime
    /// of this `Env`.
    pub unsafe fn new_direct_byte_buffer(
        &mut self,
        data: *mut u8,
        len: usize,
    ) -> Result<JByteBuffer<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let data = null_check!(data, "new_direct_byte_buffer data argument")?;
        // Safety: jni-rs requires JNI >= 1.4 and this is checked in `from_raw`
        let obj = jni_call_check_ex_and_null_ret!(
            self,
            v1_4,
            NewDirectByteBuffer,
            data as *mut c_void,
            len as jlong
        )?;
        Ok(JByteBuffer::from_raw(obj))
    }

    /// Returns the starting address of the memory of the direct
    /// java.nio.ByteBuffer.
    ///
    /// # Safety
    ///
    /// The caller must ensure the lifetime of `buf` extends to all uses of the
    /// returned pointer.
    pub fn get_direct_buffer_address(&self, buf: &JByteBuffer) -> Result<*mut u8> {
        let buf = null_check!(buf, "get_direct_buffer_address argument")?;
        // Safety: jni-rs requires JNI >= 1.4 and this is checked in `from_raw`
        unsafe {
            // GetDirectBufferAddress has no documented exceptions that it can throw
            let ptr =
                jni_call_only_check_null_ret!(self, v1_4, GetDirectBufferAddress, buf.as_raw())?;
            Ok(ptr as _)
        }
    }

    /// Returns the capacity (length) of the direct java.nio.ByteBuffer.
    ///
    /// # Terminology
    ///
    /// "capacity" here means the length that was passed to [`Self::new_direct_byte_buffer()`]
    /// which does not reflect the (potentially) larger size of the underlying allocation (unlike the `Vec`
    /// API).
    ///
    /// The terminology is simply kept from the original JNI API (`GetDirectBufferCapacity`).
    pub fn get_direct_buffer_capacity(&self, buf: &JByteBuffer) -> Result<usize> {
        let buf = null_check!(buf, "get_direct_buffer_capacity argument")?;
        // Safety: jni-rs requires JNI >= 1.4 and this is checked in `from_raw`
        unsafe {
            let capacity = jni_call_unchecked!(self, v1_4, GetDirectBufferCapacity, buf.as_raw());
            match capacity {
                -1 => Err(Error::JniCall(JniError::Unknown)),
                _ => Ok(capacity as usize),
            }
        }
    }

    /// Creates a new global reference to the Java object `obj`.
    ///
    /// Global references take more time to create or delete than ordinary
    /// local references do, but have several properties that make them useful
    /// in certain situations. See [`Global`] for more information.
    ///
    /// If you use this API to try and upgrade a [`Weak`] then it may return
    /// [`Error::ObjectFreed`] if the object has been garbage collected.
    pub fn new_global_ref<'any_local, O>(&self, obj: O) -> Result<Global<O::GlobalKind>>
    where
        O: Reference + AsRef<JObject<'any_local>>,
    {
        // Avoid passing null to `NewGlobalRef` so that we can recognise out-of-memory errors
        if obj.is_null() {
            return Ok(Global::null());
        }

        // Safety:
        // - the minimum supported JNI version is 1.4
        // - we can assume that `obj.raw()` is a valid reference
        // - we know there's no other wrapper for the reference passed to from_global_raw
        //   since we have just created it.
        let global_ref = unsafe {
            let global_ref = O::global_kind_from_raw(jni_call_unchecked!(
                self,
                v1_1,
                NewGlobalRef,
                obj.as_raw()
            ));
            Global::new(self, global_ref)
        };

        // Per JNI spec, `NewGlobalRef` will return a null pointer if the object was GC'd
        // (which could happen if `obj` is a `Weak`):
        //
        //  > it is recommended that a (strong) local or global reference to the
        //  > underlying object be acquired using one of the JNI functions
        //  > NewLocalRef or NewGlobalRef. These functions will return NULL if
        //  > the object has been freed.
        //
        if global_ref.is_null() {
            // In this case it's ambiguous whether there has been an out-of-memory error or
            // the object has been garbage collected and so we now _explicitly_ check
            // whether the object has been garbage collected.
            if self.is_same_object(obj, JObject::null()) {
                Err(Error::ObjectFreed)
            } else {
                Err(Error::JniCall(JniError::NoMemory))
            }
        } else {
            Ok(global_ref)
        }
    }

    /// Creates a new global reference and casts it to a different type.
    ///
    /// This is a convenience method that combines [`Self::cast_global`] and
    /// [`Self::new_global_ref`].
    ///
    /// It first checks if the object is an instance of the target type, and if so it creates a new
    /// global reference with the target type.
    ///
    /// `obj` can be a local reference or a global reference.
    ///
    /// For upcasting (converting to a more general type), consider using the `AsRef` trait
    /// implementations instead, which don't require runtime checks.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let local_obj: JObject = env.new_object(c"java/lang/String", c"()V", &[])?;
    /// let global_string = env.new_cast_global_ref::<JString>(local_obj)?;
    /// // global_string is now a `Global<JString>` that persists beyond local frames
    ///
    /// // For upcasting, the `AsRef` trait is more efficient:
    /// let as_obj_again: &JObject = global_string.as_ref(); // No runtime check needed
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::WrongObjectType`] if the object is not an instance of the target type.
    /// Returns [`Error::ClassNotFound`] if the target class cannot be found.
    pub fn new_cast_global_ref<'any_local, To>(
        &self,
        obj: impl Reference + AsRef<JObject<'any_local>>,
    ) -> Result<Global<To::GlobalKind>>
    where
        To: Reference,
    {
        if obj.is_null() {
            return Ok(Default::default());
        }

        if self.is_instance_of_cast_type::<To>(obj.as_ref())? {
            let new = self.new_global_ref(obj)?;
            // Safety:
            // - we have just checked that `new` is an instance of `To`
            unsafe {
                let cast = To::global_kind_from_raw(new.into_raw());
                Ok(Global::new(self, cast))
            }
        } else {
            Err(Error::WrongObjectType)
        }
    }

    /// Attempts to cast a global reference to a different type.
    ///
    /// This performs a runtime type check using `IsInstanceOf` and consumes the input reference.
    ///
    /// For upcasting (converting to a more general type), consider using the `AsRef` trait
    /// implementations instead, which don't require runtime checks.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let local_obj: JObject = env.new_object(c"java/lang/String", c"()V", &[])?;
    /// let global_obj: Global<JObject<'static>> = env.new_global_ref(&local_obj)?;
    /// let global_string = env.cast_global::<JString>(global_obj)?;
    ///
    /// // For upcasting, the `AsRef` trait is more efficient:
    /// let as_obj_again: &JObject = global_string.as_ref(); // No runtime check needed
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::WrongObjectType`] if the object is not an instance of the target type.
    /// Returns [`Error::ClassNotFound`] if the target class cannot be found.
    pub fn cast_global<To>(
        &self,
        obj: Global<
            impl Into<JObject<'static>>
                + AsRef<JObject<'static>>
                + Default
                + Reference
                + Send
                + Sync
                + 'static,
        >,
    ) -> Result<Global<To::GlobalKind>>
    where
        To: Reference,
    {
        if obj.is_null() {
            return Ok(Default::default());
        }

        if self.is_instance_of_cast_type::<To>(obj.as_ref())? {
            // Safety:
            // - we have just checked that `obj` is an instance of `T`
            // - there won't be multiple wrappers since we are creating one from the other
            unsafe {
                let cast = To::global_kind_from_raw(obj.into_raw());
                Ok(Global::new(self, cast))
            }
        } else {
            Err(Error::WrongObjectType)
        }
    }

    /// Creates a new weak global reference.
    ///
    /// Weak global references are a special kind of Java object reference that
    /// doesn't prevent the Java object from being garbage collected. See
    /// [`Weak`] for more information.
    ///
    /// If you use this API to create a [`Weak`] from another [`Weak`]
    /// then it may return [`Error::ObjectFreed`] if the object has been garbage
    /// collected.
    ///
    /// Attempting to create a [`Weak`] for a `null` reference will return an
    /// [`Error::ObjectFreed`] error.
    pub fn new_weak_ref<O>(&self, obj: O) -> Result<Weak<O::GlobalKind>>
    where
        O: Reference,
    {
        // Check if the pointer is null *before* calling `NewWeakGlobalRef`.
        //
        // This avoids a bug in some JVM implementations which, contrary to the JNI specification,
        // will throw `java.lang.OutOfMemoryError: C heap space` from `NewWeakGlobalRef` if it is
        // passed a null pointer. (The specification says it will return a null pointer in that
        // situation, not throw an exception.)
        if obj.is_null() {
            return Err(Error::ObjectFreed);
        }

        // Safety:
        // - the minimum supported JNI version is 1.4
        // - we can assume that `obj.raw()` is a valid reference
        // - we know there's no other wrapper for the reference passed to from_global_raw
        //   since we have just created it.
        let weak_ref = unsafe {
            let weak = O::global_kind_from_raw(jni_call_check_ex!(
                self,
                v1_2,
                NewWeakGlobalRef,
                obj.as_raw()
            )?);
            Weak::new(self, weak)
        };

        // Unlike for NewLocalRef and NewGlobalRef, the JNI spec doesn't seem to
        // give the same guarantee that it will return null if the object has
        // already been freed, but it seems reasonable to assume it can.

        if weak_ref.is_null() {
            // Unlike for NewLocalRef and NewGlobalRef we can assume that NewWeakGlobalRef
            // will throw an out-of-memory exception (that we catch) instead of returning null
            Err(Error::ObjectFreed)
        } else {
            Ok(weak_ref)
        }
    }

    /// Create a new local reference to an object.
    ///
    /// Specifically, this calls the JNI function [`NewLocalRef`], which creates a reference in the
    /// current local reference frame, regardless of whether the original reference belongs to the
    /// same local reference frame, a different one, or is a [global reference][Global]. In Rust
    /// terms, this method accepts a JNI reference with any valid lifetime and produces a clone of
    /// that reference with the lifetime of this `Env`. The returned reference can outlive the
    /// original.
    ///
    /// This method is useful when you have a strong global reference and you can't prevent it from
    /// being dropped before you're finished with it. In that case, you can use this method to
    /// create a new local reference that's guaranteed to remain valid for the duration of the
    /// current local reference frame, regardless of what later happens to the original global
    /// reference.
    ///
    /// # Lifetimes
    ///
    /// `'local` is the lifetime of the local reference frame that this `Env` belongs to. This
    /// method creates a new local reference in that frame, with lifetime `'local`.
    ///
    /// `'other_local` is the lifetime of the original reference's frame. It can be any valid
    /// lifetime, even one that `'local` outlives or vice versa.
    ///
    /// Think of `'local` as meaning `'new` and `'other_local` as meaning `'original`. (It is
    /// unfortunately not possible to actually give these names to the two lifetimes because
    /// `'local` is a parameter to the `Env` type, not a parameter to this method.)
    ///
    /// # Example
    ///
    /// In the following example, the `ExampleError::extract_throwable` method uses
    /// `Env::new_local_ref` to create a new local reference that outlives the original global
    /// reference:
    ///
    /// ```no_run
    /// # use jni::{Env, objects::*, strings::*};
    /// # use std::fmt::Display;
    /// #
    /// # type SomeOtherErrorType = Box<dyn Display>;
    /// #
    /// /// An error that may be caused by either a Java exception or something going wrong in Rust
    /// /// code.
    /// enum ExampleError {
    ///     /// This variant represents a Java exception.
    ///     ///
    ///     /// The enclosed `Global` points to a Java object of class `java.lang.Throwable`
    ///     /// (or one of its many subclasses).
    ///     Exception(Global<JObject<'static>>),
    ///
    ///     /// This variant represents an error in Rust code, not a Java exception.
    ///     Other(SomeOtherErrorType),
    /// }
    ///
    /// impl ExampleError {
    ///     /// Consumes this `ExampleError` and produces a `JThrowable`, suitable for throwing
    ///     /// back to Java code.
    ///     ///
    ///     /// If this is an `ExampleError::Exception`, then this extracts the enclosed Java
    ///     /// exception object. Otherwise, a new exception object is created to represent this
    ///     /// error.
    ///     fn extract_throwable<'local>(self, env: &mut Env<'local>) -> jni::errors::Result<JThrowable<'local>> {
    ///         let throwable: JObject = match self {
    ///             ExampleError::Exception(exception) => {
    ///                 // The error was caused by a Java exception.
    ///
    ///                 // Here, `exception` is a `Global` pointing to a Java `Throwable`. It
    ///                 // will be dropped at the end of this `match` arm. We'll use
    ///                 // `new_local_ref` to create a local reference that will outlive the
    ///                 // `Global`.
    ///
    ///                 env.new_local_ref(&exception)?
    ///             }
    ///
    ///             ExampleError::Other(error) => {
    ///                 // The error was caused by something that happened in Rust code. Create a
    ///                 // new `java.lang.Error` to represent it.
    ///
    ///                 let error_string = env.new_string(JNIString::from(error.to_string()))?;
    ///
    ///                 env.new_object(
    ///                     c"java/lang/Error",
    ///                     c"(Ljava/lang/String;)V",
    ///                     &[
    ///                         (&error_string).into(),
    ///                     ],
    ///                 )?
    ///             }
    ///         };
    ///         let throwable = env.cast_local::<JThrowable>(throwable)?;
    ///         Ok(throwable)
    ///     }
    /// }
    /// ```
    ///
    /// [`NewLocalRef`]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/functions.html#newlocalref
    pub fn new_local_ref<'any_local, O>(&mut self, obj: O) -> Result<O::Kind<'local>>
    where
        O: Reference + AsRef<JObject<'any_local>>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();

        //let obj = obj.as_ref();

        // By checking for `null` before calling `NewLocalRef` we can recognise
        // that a `null` returned from `NewLocalRef` is from being out of memory.
        if obj.is_null() {
            return Ok(O::null());
        }

        // Safety:
        // - the minimum supported JNI version is 1.4
        // - we can assume that `obj.raw()` is a valid reference, or null
        // - we know there's no other wrapper for the reference passed to from_local_raw
        //   since we have just created it.
        let local =
            unsafe { O::kind_from_raw(jni_call_unchecked!(self, v1_2, NewLocalRef, obj.as_raw())) };

        // Per JNI spec, `NewLocalRef` will return a null pointer if the object was GC'd
        // (which could happen if `obj` is a `Weak`):
        //
        //  > it is recommended that a (strong) local or global reference to the
        //  > underlying object be acquired using one of the JNI functions
        //  > NewLocalRef or NewGlobalRef. These functions will return NULL if
        //  > the object has been freed.
        //
        if local.is_null() {
            // In this case it's ambiguous whether there has been an out-of-memory error or
            // the object has been garbage collected and so we now _explicitly_ check
            // whether the object has been garbage collected.
            if self.is_same_object(obj, JObject::null()) {
                Err(Error::ObjectFreed)
            } else {
                Err(Error::JniCall(JniError::NoMemory))
            }
        } else {
            Ok(local)
        }
    }

    /// Creates a new local reference and casts it to a different type.
    ///
    /// This is a convenience method that combines [`Self::cast_local`] and [`Self::new_local_ref`].
    ///
    /// This performs a runtime type check using `IsInstanceOf` and then creates a new local
    /// reference.
    ///
    /// `obj` can be a local reference or a global reference.
    ///
    /// For upcasting (converting to a more general type), consider using the `From` trait
    /// implementations instead, which don't require runtime checks.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let local_obj: JObject = env.new_object(c"java/lang/String", c"()V", &[])?;
    /// let local_string = env.new_cast_local_ref::<JString>(&local_obj)?;
    /// // local_string is now a JString<'local> in the current frame
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::WrongObjectType`] if the object is not an instance of the target type.
    /// Returns [`Error::ClassNotFound`] if the target class cannot be found.
    pub fn new_cast_local_ref<'any_local, To>(
        &mut self,
        obj: impl Reference + AsRef<JObject<'any_local>>,
    ) -> Result<To::Kind<'local>>
    where
        To: Reference,
    {
        if obj.is_null() {
            return Ok(To::null());
        }

        if self.is_instance_of_cast_type::<To>(obj.as_ref())? {
            let new = self.new_local_ref(obj.as_ref())?;
            // Safety:
            // - we have just checked that `new` is an instance of `To`
            // - as it's a new reference, it's assigned the `'local` Env lifetime
            unsafe { Ok(To::kind_from_raw::<'local>(new.into_raw())) }
        } else {
            Err(Error::WrongObjectType)
        }
    }

    /// Attempts to cast a local reference to a different type.
    ///
    /// This performs a runtime type check using `IsInstanceOf` and consumes the input reference.
    /// For upcasting (converting to a more general type), consider using the `From` trait
    /// implementations instead, which don't require runtime checks.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let obj: JObject = env.new_object(c"java/lang/String", c"()V", &[])?;
    /// let string: JString = env.cast_local::<JString>(obj)?;
    ///
    /// // For upcasting, From trait is more efficient:
    /// let obj_again: JObject = string.into(); // No runtime check needed
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::WrongObjectType`] if the object is not an instance of the target type.
    /// Returns [`Error::ClassNotFound`] if the target class cannot be found.
    pub fn cast_local<'any_local, To>(
        &self,
        obj: impl Reference + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
    ) -> Result<To::Kind<'any_local>>
    where
        To: Reference,
    {
        if obj.is_null() {
            return Ok(To::null::<'any_local>());
        }

        if self.is_instance_of_cast_type::<To>(obj.as_ref())? {
            let obj: JObject = obj.into();
            // Safety:
            // - we have just checked that `obj` is an instance of `T`
            // - it is associated with the same lifetime that it was created with
            unsafe { Ok(To::kind_from_raw::<'any_local>(obj.into_raw())) }
        } else {
            Err(Error::WrongObjectType)
        }
    }

    /// Attempts to cast a reference (local or global) to a different type
    /// without consuming it.
    ///
    /// This method borrows the input reference and returns a wrapper that
    /// derefs to the target type. The original reference remains valid and can
    /// be used after the cast operation.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let obj: JObject = env.new_object(c"java/lang/String", c"()V", &[])?;
    /// let string_ref = env.as_cast::<JString>(&obj)?;
    /// // obj is still valid here
    /// let empty_string_contents = string_ref.to_string();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::WrongObjectType`] if the object is not an instance of
    /// the target type. Returns [`Error::ClassNotFound`] if the target class
    /// cannot be found.
    pub fn as_cast<'from, 'any, To>(
        &self,
        obj: &'from (impl Reference + AsRef<JObject<'any>>),
    ) -> Result<Cast<'from, 'any, To>>
    where
        To: Reference,
        'any: 'from,
    {
        Cast::new(self, obj)
    }

    /// Attempts to cast a raw [`jobject`] reference without taking ownership.
    ///
    /// This method borrows the input reference and returns a wrapper that
    /// derefs to the target type. The original reference remains valid and can
    /// continue to be used.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::*, sys::jobject};
    /// #
    /// # fn example(env: &mut Env, raw_global: jobject) -> Result<()> {
    /// // SAFETY: we know that raw_global is a valid java.lang.String reference
    /// let string_ref = unsafe { env.as_cast_raw::<Global<JString>>(&raw_global)? };
    /// let string_contents = string_ref.to_string();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::WrongObjectType`] if the object is not an instance of
    /// the target type. Returns [`Error::ClassNotFound`] if the target class
    /// cannot be found.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `from` is a valid reference (local or
    /// global) - which may be `null`.
    ///
    /// The caller must ensure the `from` reference will not be deleted while
    /// the `Cast` exists.
    ///
    /// Note: even though this API is `unsafe`, it will still do a runtime check
    /// that `from` is a valid instance of `To`, so you are not required to know
    /// this.
    ///
    /// Note: this API is agnostic about whether the reference is local or
    /// global because the returned [`Cast`] wrapper doesn't give ownership over
    /// the reference and so you can't accidentally attempt to delete it using
    /// the wrong JNI API.
    pub unsafe fn as_cast_raw<'from, To>(
        &self,
        from: &'from jobject,
    ) -> Result<Cast<'from, 'from, To>>
    where
        To: Reference,
    {
        Cast::from_raw(self, from)
    }

    /// Creates a new auto-deleted local reference.
    ///
    /// See also [`with_local_frame`](struct.Env.html#method.with_local_frame) method that
    /// can be more convenient when you create a _bounded_ number of local references
    /// but cannot rely on automatic de-allocation (e.g., in case of recursion, deep call stacks,
    /// [permanently-attached](struct.JavaVM.html#attaching-native-threads) native threads, etc.).
    #[deprecated = "Use '.auto()' from IntoAuto trait"]
    pub fn auto_local<O>(&self, obj: O) -> Auto<'local, O>
    where
        O: Into<JObject<'local>>,
    {
        Auto::new(obj)
    }

    /// Deletes a local reference early, before its JNI stack frame unwinds.
    ///
    /// Local references exist within a JNI stack frame, which would typically be created by the
    /// Java VM before making a native method call, and then unwound when your native method
    /// returns.
    ///
    /// New JNI stack frames may also be created via [`Self::with_local_frame`].
    ///
    /// Typically you don't have to worry about deleting local references since they are
    /// automatically freed when the JNI stack frame they were created in unwinds.
    ///
    /// But, each local reference takes memory and so you need to make sure to not excessively
    /// allocate local references.
    ///
    /// If you find that you are allocating a large number of local references in a single native
    /// method call, (e.g. while looping over a large collection), this API can be used to
    /// explicitly delete local references before the JNI stack frame unwinds.
    ///
    /// In most cases it is better to use [`Auto`] (see [`IntoAuto::auto`] method) or
    /// [`Self::with_local_frame`] instead of directly calling [`Self::delete_local_ref`].
    pub fn delete_local_ref<'other_local, O>(&self, obj: O)
    where
        O: Into<JObject<'other_local>>,
    {
        let obj = obj.into();
        let raw = obj.into_raw();

        // Safety: `raw` may be `null`
        unsafe {
            jni_call_unchecked!(self, v1_1, DeleteLocalRef, raw);
        }
    }

    /// Creates a new local reference frame, in which at least a given number
    /// of local references can be created.
    ///
    /// Returns `Err` on failure, with a pending `OutOfMemoryError`.
    ///
    /// Prefer to use
    /// [`with_local_frame`](struct.Env.html#method.with_local_frame)
    /// instead of direct `push_local_frame`/`pop_local_frame` calls.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that there will be a corresponding call to
    /// `pop_local_frame`
    ///
    /// The caller must ensure that a new `AttachGuard` is created before
    /// creating a new local frame and the the local frame may only access
    /// a `Env` that is borrowed from this new guard (so that local
    /// references will be tied to lifetime of the new guard)
    unsafe fn push_local_frame(&self, capacity: i32) -> Result<()> {
        // Safety:
        // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
        // We check for JNI > 1.2 in `from_raw`
        let res = unsafe { jni_call_unchecked!(self, v1_2, PushLocalFrame, capacity) };
        jni_error_code_to_result(res)
    }

    /// Pops off the current local reference frame, frees all the local
    /// references allocated on the current stack frame, except the `result`,
    /// which is returned from this function and remains valid.
    ///
    /// The resulting `JObject` will be `NULL` iff `result` is `NULL`.
    ///
    /// This method allows direct control of local frames, but it can cause
    /// undefined behavior and is therefore unsafe. Prefer
    /// [`Env::with_local_frame`] instead.
    ///
    /// # Safety
    ///
    /// Any local references created after the most recent call to
    /// [`Env::push_local_frame`] (or the underlying JNI function) must not
    /// be used after calling this method.
    ///
    /// The `AttachGuard` created before calling `push_local_frame` must be
    /// dropped after calling `pop_local_frame`.
    unsafe fn pop_local_frame<'frame_local, T: Reference>(
        &self,
        result: T::Kind<'frame_local>,
    ) -> Result<T::Kind<'local>> {
        let result: JObject<'frame_local> = result.into();
        // Safety:
        // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
        // We check for JNI > 1.2 in `from_raw`
        let raw = jni_call_unchecked!(self, v1_2, PopLocalFrame, result.into_raw());
        Ok(T::kind_from_raw(raw))
    }

    /// Executes the given function in a new local reference frame, in which at least a given number
    /// of references can be created. Once this method returns, all references allocated in the
    /// frame are freed.
    ///
    /// If a frame can't be allocated with the requested capacity for local references, returns
    /// `Err` with a pending `OutOfMemoryError`.
    ///
    /// Since local references created within this frame won't be accessible to the calling frame
    /// then if you need to pass an object back to the caller then you can do that via
    /// [`Self::with_local_frame_returning_local`] or else return a [`Global`] with
    /// [`Self::new_global_ref`].
    pub fn with_local_frame<F, T, E>(&mut self, capacity: usize, f: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut Env) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        // Runtime check that the new local frame is being pushed on top of the
        // the current top JNI stack frame
        self.assert_top();

        let capacity: jni_sys::jint = capacity
            .try_into()
            .map_err(|_| Error::JniCall(JniError::InvalidArguments))?;

        unsafe {
            // Safety: by creating a new AttachGuard we ensure that the attach guard level
            // will be incremented in sync with the creation of a new JNI stack frame
            let mut guard = AttachGuard::from_unowned(self.get_raw());
            let env = guard.borrow_env_mut();
            self.push_local_frame(capacity)?;
            let ret = catch_unwind(AssertUnwindSafe(|| f(env)));
            self.pop_local_frame::<JObject>(JObject::null())?;
            drop(guard);

            match ret {
                Ok(ret) => ret,
                Err(payload) => {
                    resume_unwind(payload);
                }
            }
        }
    }

    /// Executes the given function in a new local reference frame, in which at least a given number
    /// of references can be created. Once this method returns, all references allocated
    /// in the frame are freed, except the one that the function returns, which remains valid.
    ///
    /// If a frame can't be allocated with the requested capacity for local
    /// references, returns `Err` with a pending `OutOfMemoryError`.
    ///
    /// Since the low-level JNI interface has support for passing back a single local reference
    /// from a local frame as special-case optimization, this alternative to `with_local_frame`
    /// exposes that capability to return a local reference without needing to create a
    /// temporary [`Global`].
    pub fn with_local_frame_returning_local<F, T, E>(
        &mut self,
        capacity: usize,
        f: F,
    ) -> std::result::Result<T::Kind<'local>, E>
    where
        F: for<'new_local> FnOnce(
            &mut Env<'new_local>,
        ) -> std::result::Result<T::Kind<'new_local>, E>,
        T: Reference,
        E: From<Error>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();

        let capacity: jni_sys::jint = capacity
            .try_into()
            .map_err(|_| Error::JniCall(JniError::InvalidArguments))?;

        unsafe {
            // Inner scope to ensure drop order: `Result` -> `env` -> `guard`, before we potentially call `resume_unwind`.
            let panic_payload = {
                // Safety: by creating a new AttachGuard we ensure that the attach guard level
                // will be incremented in sync with the creation of a new JNI stack frame
                let mut guard = AttachGuard::from_unowned(self.get_raw());
                let env = guard.borrow_env_mut();

                self.push_local_frame(capacity)?;

                let payload = match catch_unwind(AssertUnwindSafe(|| f(env))) {
                    Ok(Ok(obj)) => {
                        let obj = self.pop_local_frame::<T>(obj)?;
                        return Ok(obj);
                    }
                    Ok(Err(err)) => {
                        self.pop_local_frame::<T>(T::null())?;
                        return Err(err);
                    }
                    Err(payload) => {
                        self.pop_local_frame::<T>(T::null())?;
                        payload
                    }
                };
                payload
            };

            resume_unwind(panic_payload);
        }
    }

    /// Allocates a new object from a class descriptor without running a
    /// constructor.
    pub fn alloc_object<'other_local, T>(&mut self, class: T) -> Result<JObject<'local>>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let class = class.lookup(self)?;
        let obj = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, AllocObject, class.as_ref().as_raw())?
        };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        Ok(unsafe { JObject::from_raw(obj) })
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Common functionality for finding methods.
    #[allow(clippy::redundant_closure_call)]
    fn get_method_id_base<'other_local_1, T, U, V, C, R>(
        &mut self,
        class: T,
        name: U,
        sig: V,
        get_method: C,
    ) -> Result<R>
    where
        T: Desc<'local, JClass<'other_local_1>>,
        U: AsRef<JNIStr>,
        V: AsRef<JNIStr>,
        C: for<'other_local_2> Fn(
            &mut Self,
            &JClass<'other_local_2>,
            &JNIStr,
            &JNIStr,
        ) -> Result<R>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.as_ref();
        let sig = sig.as_ref();

        let res: Result<R> = get_method(self, class.as_ref(), ffi_name, sig);

        match res {
            Ok(m) => Ok(m),
            Err(e) => match e {
                Error::NullPtr(_) => {
                    let name: String = ffi_name.to_str().into();
                    let sig: String = sig.to_str().into();
                    Err(Error::MethodNotFound { name, sig })
                }
                _ => Err(e),
            },
        }
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Look up a method by class descriptor, name, and
    /// signature.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::JMethodID};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let method_id: JMethodID =
    ///     env.get_method_id(c"java/lang/String", c"substring", c"(II)Ljava/lang/String;")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_method_id<'other_local, T, U, V>(
        &mut self,
        class: T,
        name: U,
        sig: V,
    ) -> Result<JMethodID>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: AsRef<JNIStr>,
        V: AsRef<JNIStr>,
    {
        self.get_method_id_base(class, name, sig, |env, class, name, sig| unsafe {
            jni_call_check_ex_and_null_ret!(
                env,
                v1_1,
                GetMethodID,
                class.as_raw(),
                name.as_ptr(),
                sig.as_ptr()
            )
            .map(|method_id| JMethodID::from_raw(method_id))
        })
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Look up a static method by class descriptor, name, and
    /// signature.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::JStaticMethodID};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let method_id: JStaticMethodID =
    ///     env.get_static_method_id(c"java/lang/String", c"valueOf", c"(I)Ljava/lang/String;")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_static_method_id<'other_local, T, U, V>(
        &mut self,
        class: T,
        name: U,
        sig: V,
    ) -> Result<JStaticMethodID>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: AsRef<JNIStr>,
        V: AsRef<JNIStr>,
    {
        self.get_method_id_base(class, name, sig, |env, class, name, sig| unsafe {
            jni_call_check_ex_and_null_ret!(
                env,
                v1_1,
                GetStaticMethodID,
                class.as_raw(),
                name.as_ptr(),
                sig.as_ptr()
            )
            .map(|method_id| JStaticMethodID::from_raw(method_id))
        })
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Look up the field ID for a class/name/type combination.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::JFieldID};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let field_id: JFieldID = env.get_field_id(c"com/my/Class", c"intField", c"I")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_field_id<'other_local, T, U, V>(
        &mut self,
        class: T,
        name: U,
        sig: V,
    ) -> Result<JFieldID>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: AsRef<JNIStr>,
        V: AsRef<JNIStr>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.as_ref();
        let ffi_sig = sig.as_ref();

        let res = unsafe {
            jni_call_check_ex_and_null_ret!(
                self,
                v1_1,
                GetFieldID,
                class.as_ref().as_raw(),
                ffi_name.as_ptr(),
                ffi_sig.as_ptr()
            )
            .map(|field_id| JFieldID::from_raw(field_id))
        };

        match res {
            Ok(m) => Ok(m),
            Err(e) => match e {
                Error::NullPtr(_) => {
                    let name: String = ffi_name.to_str().into();
                    let sig: String = ffi_sig.to_str().into();
                    Err(Error::FieldNotFound { name, sig })
                }
                _ => Err(e),
            },
        }
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Look up the static field ID for a class/name/type combination.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, Env, objects::JStaticFieldID};
    /// #
    /// # fn example(env: &mut Env) -> Result<()> {
    /// let field_id: JStaticFieldID = env.get_static_field_id(c"com/my/Class", c"intField", c"I")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_static_field_id<'other_local, T, U, V>(
        &mut self,
        class: T,
        name: U,
        sig: V,
    ) -> Result<JStaticFieldID>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: AsRef<JNIStr>,
        V: AsRef<JNIStr>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.as_ref();
        let ffi_sig = sig.as_ref();

        let res = unsafe {
            jni_call_check_ex_and_null_ret!(
                self,
                v1_1,
                GetStaticFieldID,
                class.as_ref().as_raw(),
                ffi_name.as_ptr(),
                ffi_sig.as_ptr()
            )
            .map(|field_id| JStaticFieldID::from_raw(field_id))
        };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        match res {
            Ok(m) => Ok(m),
            Err(e) => match e {
                Error::NullPtr(_) => {
                    let name: String = ffi_name.to_str().into();
                    let sig: String = ffi_sig.to_str().into();
                    Err(Error::FieldNotFound { name, sig })
                }
                _ => Err(e),
            },
        }
    }

    /// Get the class for an object.
    pub fn get_object_class<'other_local, O>(&mut self, obj: O) -> Result<JClass<'local>>
    where
        O: AsRef<JObject<'other_local>>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let obj = obj.as_ref();
        let obj = null_check!(obj, "get_object_class")?;
        unsafe {
            Ok(JClass::from_raw(jni_call_unchecked!(
                self,
                v1_1,
                GetObjectClass,
                obj.as_raw()
            )))
        }
    }

    /// Call a static method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the class, whether the return
    /// type is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `CallStatic<Type>MethodA` method
    /// with the provided arguments.
    ///
    /// # Safety
    ///
    /// The provided JMethodID must be valid, and match the types and number of arguments, and return type.
    /// If these are incorrect, the JVM may crash. The JMethodID must also match the passed type.
    pub unsafe fn call_static_method_unchecked<'other_local, T, U>(
        &mut self,
        class: T,
        method_id: U,
        ret: ReturnType,
        args: &[jvalue],
    ) -> Result<JValueOwned<'local>>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: Desc<'local, JStaticMethodID>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use JavaType::{Array, Object, Primitive};

        let class = class.lookup(self)?;

        let method_id = method_id.lookup(self)?.as_ref().into_raw();

        let class_raw = class.as_ref().as_raw();
        let jni_args = args.as_ptr();

        macro_rules! invoke {
            ($call:ident -> $ret:ty) => {{
                let o: $ret =
                    jni_call_check_ex!(self, v1_1, $call, class_raw, method_id, jni_args)?;
                o
            }};
        }

        let ret = match ret {
            Object | Array => {
                let obj = invoke!(CallStaticObjectMethodA -> jobject);
                let obj = unsafe { JObject::from_raw(obj) };
                JValueOwned::from(obj)
            }
            Primitive(Boolean) => invoke!(CallStaticBooleanMethodA -> bool).into(),
            Primitive(Char) => invoke!(CallStaticCharMethodA -> u16).into(),
            Primitive(Byte) => invoke!(CallStaticByteMethodA -> i8).into(),
            Primitive(Short) => invoke!(CallStaticShortMethodA -> i16).into(),
            Primitive(Int) => invoke!(CallStaticIntMethodA -> i32).into(),
            Primitive(Long) => invoke!(CallStaticLongMethodA -> i64).into(),
            Primitive(Float) => invoke!(CallStaticFloatMethodA -> f32).into(),
            Primitive(Double) => invoke!(CallStaticDoubleMethodA -> f64).into(),
            Primitive(Void) => {
                jni_call_check_ex!(
                    self,
                    v1_1,
                    CallStaticVoidMethodA,
                    class_raw,
                    method_id,
                    jni_args
                )?;
                JValueOwned::Void
            }
        };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        Ok(ret)
    }

    /// Call an object method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the object, whether the return
    /// type is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `Call<Type>MethodA` method with
    /// the provided arguments.
    ///
    /// # Safety
    ///
    /// The provided JMethodID must be valid, and match the types and number of arguments, and return type.
    /// If these are incorrect, the JVM may crash. The JMethodID must also match the passed type.
    pub unsafe fn call_method_unchecked<'other_local, O, T>(
        &mut self,
        obj: O,
        method_id: T,
        ret_ty: ReturnType,
        args: &[jvalue],
    ) -> Result<JValueOwned<'local>>
    where
        O: AsRef<JObject<'other_local>>,
        T: Desc<'local, JMethodID>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use JavaType::{Array, Object, Primitive};

        let method_id = method_id.lookup(self)?.as_ref().into_raw();

        let obj = obj.as_ref().as_raw();

        let jni_args = args.as_ptr();

        macro_rules! invoke {
            ($call:ident -> $ret:ty) => {{
                let o: $ret = jni_call_check_ex!(self, v1_1, $call, obj, method_id, jni_args)?;
                o
            }};
        }

        let ret = match ret_ty {
            Object | Array => {
                let obj = invoke!(CallObjectMethodA -> jobject);
                let obj = unsafe { JObject::from_raw(obj) };
                JValueOwned::from(obj)
            }
            Primitive(Boolean) => invoke!(CallBooleanMethodA -> bool).into(),
            Primitive(Char) => invoke!(CallCharMethodA -> u16).into(),
            Primitive(Byte) => invoke!(CallByteMethodA -> i8).into(),
            Primitive(Short) => invoke!(CallShortMethodA -> i16).into(),
            Primitive(Int) => invoke!(CallIntMethodA -> i32).into(),
            Primitive(Long) => invoke!(CallLongMethodA -> i64).into(),
            Primitive(Float) => invoke!(CallFloatMethodA -> f32).into(),
            Primitive(Double) => invoke!(CallDoubleMethodA -> f64).into(),
            Primitive(Void) => {
                jni_call_check_ex!(self, v1_1, CallVoidMethodA, obj, method_id, jni_args)?;
                JValueOwned::Void
            }
        };

        Ok(ret)
    }

    /// Call an non-virtual object method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the object, whether the return
    /// type is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `CallNonvirtual<Type>MethodA` method with
    /// the provided arguments.
    ///
    /// # Safety
    ///
    /// The provided JClass, JMethodID must be valid, and match the types and number of arguments, and return type.
    /// If these are incorrect, the JVM may crash. The JMethodID must also match the passed type.
    pub unsafe fn call_nonvirtual_method_unchecked<'other_local, O, T, U>(
        &mut self,
        obj: O,
        class: T,
        method_id: U,
        ret_ty: ReturnType,
        args: &[jvalue],
    ) -> Result<JValueOwned<'local>>
    where
        O: AsRef<JObject<'other_local>>,
        T: Desc<'local, JClass<'other_local>>,
        U: Desc<'local, JMethodID>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use JavaType::{Array, Object, Primitive};

        let method_id = method_id.lookup(self)?.as_ref().into_raw();
        let class = class.lookup(self)?;

        let obj = obj.as_ref().as_raw();
        let class_raw = class.as_ref().as_raw();

        let jni_args = args.as_ptr();

        macro_rules! invoke {
            ($call:ident -> $ret:ty) => {{
                let o: $ret =
                    jni_call_check_ex!(self, v1_1, $call, obj, class_raw, method_id, jni_args)?;
                o
            }};
        }

        let ret = match ret_ty {
            Object | Array => {
                let obj = invoke!(CallNonvirtualObjectMethodA -> jobject);
                let obj = unsafe { JObject::from_raw(obj) };
                JValueOwned::from(obj)
            }
            Primitive(Boolean) => invoke!(CallNonvirtualBooleanMethodA -> bool).into(),
            Primitive(Char) => invoke!(CallNonvirtualCharMethodA -> u16).into(),
            Primitive(Byte) => invoke!(CallNonvirtualByteMethodA -> i8).into(),
            Primitive(Short) => invoke!(CallNonvirtualShortMethodA -> i16).into(),
            Primitive(Int) => invoke!(CallNonvirtualIntMethodA -> i32).into(),
            Primitive(Long) => invoke!(CallNonvirtualLongMethodA -> i64).into(),
            Primitive(Float) => invoke!(CallNonvirtualFloatMethodA -> f32).into(),
            Primitive(Double) => invoke!(CallNonvirtualDoubleMethodA -> f64).into(),
            Primitive(Void) => {
                jni_call_check_ex!(
                    self,
                    v1_1,
                    CallNonvirtualVoidMethodA,
                    obj,
                    class_raw,
                    method_id,
                    jni_args
                )?;
                JValueOwned::Void
            }
        };

        Ok(ret)
    }

    /// Calls an object method safely. This comes with a number of
    /// lookups/checks. It
    ///
    /// * Parses the type signature to find the number of arguments and return
    ///   type
    /// * Looks up the JClass for the given object.
    /// * Looks up the JMethodID for the class/name/signature combination
    /// * Ensures that the number/types of args matches the signature
    ///   * Cannot check an object's type - but primitive types are matched against each other (including Object)
    /// * Calls `call_method_unchecked` with the verified safe arguments.
    ///
    /// Note: this may cause a Java exception if the arguments are the wrong
    /// type, in addition to if the method itself throws.
    pub fn call_method<'other_local, O, S, T>(
        &mut self,
        obj: O,
        name: S,
        sig: T,
        args: &[JValue],
    ) -> Result<JValueOwned<'local>>
    where
        O: AsRef<JObject<'other_local>>,
        S: AsRef<JNIStr>,
        T: AsRef<JNIStr>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let obj = obj.as_ref();
        let obj = null_check!(obj, "call_method obj argument")?;
        let sig = sig.as_ref();

        // parse the signature
        let parsed = TypeSignature::from_str(sig.to_str())?;
        if parsed.args.len() != args.len() {
            return Err(Error::InvalidArgList(parsed));
        }

        // check arguments types
        let base_types_match = parsed
            .args
            .iter()
            .zip(args.iter())
            .all(|(exp, act)| match exp {
                JavaType::Primitive(p) => act.primitive_type() == Some(*p),
                JavaType::Object | JavaType::Array => act.primitive_type().is_none(),
            });
        if !base_types_match {
            return Err(Error::InvalidArgList(parsed));
        }

        let class = self.get_object_class(obj)?.auto();

        let args: Vec<jvalue> = args.iter().map(|v| v.as_jni()).collect();

        // SAFETY: We've obtained the method_id above, so it is valid for this class.
        // We've also validated the argument counts and types using the same type signature
        // we fetched the original method ID from.
        unsafe { self.call_method_unchecked(obj, (&class, name, sig), parsed.ret, &args) }
    }

    /// Calls a static method safely. This comes with a number of
    /// lookups/checks. It
    ///
    /// * Parses the type signature to find the number of arguments and return
    ///   type
    /// * Looks up the JMethodID for the class/name/signature combination
    /// * Ensures that the number/types of args matches the signature
    ///   * Cannot check an object's type - but primitive types are matched against each other (including Object)
    /// * Calls `call_static_method_unchecked` with the verified safe arguments.
    ///
    /// Note: this may cause a Java exception if the arguments are the wrong
    /// type, in addition to if the method itself throws.
    pub fn call_static_method<'other_local, T, U, V>(
        &mut self,
        class: T,
        name: U,
        sig: V,
        args: &[JValue],
    ) -> Result<JValueOwned<'local>>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: AsRef<JNIStr>,
        V: AsRef<JNIStr>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let sig = sig.as_ref();
        let parsed = TypeSignature::from_str(sig.to_str())?;
        if parsed.args.len() != args.len() {
            return Err(Error::InvalidArgList(parsed));
        }

        // check arguments types
        let base_types_match = parsed
            .args
            .iter()
            .zip(args.iter())
            .all(|(exp, act)| match exp {
                JavaType::Primitive(p) => act.primitive_type() == Some(*p),
                JavaType::Object | JavaType::Array => act.primitive_type().is_none(),
            });
        if !base_types_match {
            return Err(Error::InvalidArgList(parsed));
        }

        // go ahead and look up the class since we'll need that for the next call.
        let class = class.lookup(self)?;
        let class = class.as_ref();

        let args: Vec<jvalue> = args.iter().map(|v| v.as_jni()).collect();

        // SAFETY: We've obtained the method_id above, so it is valid for this class.
        // We've also validated the argument counts and types using the same type signature
        // we fetched the original method ID from.
        unsafe { self.call_static_method_unchecked(class, (class, name, sig), parsed.ret, &args) }
    }

    /// Calls a non-virtual method safely. This comes with a number of
    /// lookups/checks. It
    ///
    /// * Parses the type signature to find the number of arguments and return
    ///   type
    /// * Looks up the JMethodID for the class/name/signature combination
    /// * Ensures that the number/types of args matches the signature
    ///   * Cannot check an object's type - but primitive types are matched against each other (including Object)
    /// * Calls `call_nonvirtual_method_unchecked` with the verified safe arguments.
    ///
    /// Note: this may cause a Java exception if the arguments are the wrong
    /// type, in addition to if the method itself throws.
    pub fn call_nonvirtual_method<'other_local, O, T, U, V>(
        &mut self,
        obj: O,
        class: T,
        name: U,
        sig: V,
        args: &[JValue],
    ) -> Result<JValueOwned<'local>>
    where
        O: AsRef<JObject<'other_local>>,
        T: Desc<'local, JClass<'other_local>>,
        U: AsRef<JNIStr>,
        V: AsRef<JNIStr>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let obj = obj.as_ref();
        let obj = null_check!(obj, "call_method obj argument")?;
        let sig = sig.as_ref();

        let parsed = TypeSignature::from_str(sig.to_str())?;
        if parsed.args.len() != args.len() {
            return Err(Error::InvalidArgList(parsed));
        }

        // check arguments types
        let base_types_match = parsed
            .args
            .iter()
            .zip(args.iter())
            .all(|(exp, act)| match exp {
                JavaType::Primitive(p) => act.primitive_type() == Some(*p),
                JavaType::Object | JavaType::Array => act.primitive_type().is_none(),
            });
        if !base_types_match {
            return Err(Error::InvalidArgList(parsed));
        }

        // go ahead and look up the class since we'll need that for the next call.
        let class = class.lookup(self)?;
        let class = class.as_ref();

        let args: Vec<jvalue> = args.iter().map(|v| v.as_jni()).collect();

        // SAFETY: We've obtained the method_id above, so it is valid for this class.
        // We've also validated the argument counts and types using the same type signature
        // we fetched the original method ID from.
        unsafe {
            self.call_nonvirtual_method_unchecked(obj, class, (class, name, sig), parsed.ret, &args)
        }
    }

    /// Create a new object using a constructor. This is done safely using
    /// checks similar to those in `call_static_method`.
    pub fn new_object<'other_local, T, U>(
        &mut self,
        class: T,
        ctor_sig: U,
        ctor_args: &[JValue],
    ) -> Result<JObject<'local>>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: AsRef<JNIStr>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let ctor_sig = ctor_sig.as_ref();
        // parse the signature
        let parsed = TypeSignature::from_str(ctor_sig.to_str())?;

        // check arguments length
        if parsed.args.len() != ctor_args.len() {
            return Err(Error::InvalidArgList(parsed));
        }

        // check arguments types
        let base_types_match =
            parsed
                .args
                .iter()
                .zip(ctor_args.iter())
                .all(|(exp, act)| match exp {
                    JavaType::Primitive(p) => act.primitive_type() == Some(*p),
                    JavaType::Object | JavaType::Array => act.primitive_type().is_none(),
                });
        if !base_types_match {
            return Err(Error::InvalidArgList(parsed));
        }

        // check return value
        if parsed.ret != ReturnType::Primitive(Primitive::Void) {
            return Err(Error::InvalidCtorReturn);
        }

        // build strings
        let class = class.lookup(self)?;
        let class = class.as_ref();

        let method_id: JMethodID = Desc::<JMethodID>::lookup((class, ctor_sig), self)?;

        let ctor_args: Vec<jvalue> = ctor_args.iter().map(|v| v.as_jni()).collect();
        // SAFETY: We've obtained the method_id above, so it is valid for this class.
        // We've also validated the argument counts and types using the same type signature
        // we fetched the original method ID from.
        unsafe { self.new_object_unchecked(class, method_id, &ctor_args) }
    }

    /// Create a new object using a constructor. Arguments aren't checked
    /// because of the `JMethodID` usage.
    ///
    /// # Safety
    ///
    /// The provided JMethodID must be valid, and match the types and number of arguments, as well as return type
    /// (always an Object for a constructor). If these are incorrect, the JVM may crash.  The JMethodID must also match
    /// the passed type.
    pub unsafe fn new_object_unchecked<'other_local, C, M>(
        &mut self,
        class: C,
        ctor_id: M,
        ctor_args: &[jvalue],
    ) -> Result<JObject<'local>>
    where
        C: Desc<'local, JClass<'other_local>>,
        M: Desc<'local, JMethodID>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let class = class.lookup(self)?;

        let ctor_id: JMethodID = *ctor_id.lookup(self)?.as_ref();

        let jni_args = ctor_args.as_ptr();

        let obj = unsafe {
            jni_call_check_ex_and_null_ret!(
                self,
                v1_1,
                NewObjectA,
                class.as_ref().as_raw(),
                ctor_id.into_raw(),
                jni_args
            )
            .map(|obj| JObject::from_raw(obj))
        }?;

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        Ok(obj)
    }

    /// Cast a [JObject] to a [JList].
    ///
    /// Returns `Error::WrongObjectType` if the object is not a `java.util.List`.
    #[deprecated(
        since = "0.22.0",
        note = "use JList::cast_local instead or Env::new_cast_local_ref/cast_local/as_cast_local or Env::new_cast_global_ref/cast_global/as_cast_global"
    )]
    pub fn get_list<'any_local>(
        &mut self,
        obj: impl Reference + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
    ) -> Result<JList<'any_local>> {
        JList::cast_local(obj, self)
    }

    /// Cast a [JObject] to a [JMap].
    ///
    /// Returns `Error::WrongObjectType` if the object is not a `java.util.Map`.
    #[deprecated(
        since = "0.22.0",
        note = "use JMap::cast_local instead or Env::new_cast_local_ref/cast_local/as_cast_local or Env::new_cast_global_ref/cast_global/as_cast_global"
    )]
    pub fn get_map<'any_local>(
        &mut self,
        obj: impl Reference + Into<JObject<'any_local>> + AsRef<JObject<'any_local>>,
    ) -> Result<JMap<'any_local>> {
        JMap::cast_local(obj, self)
    }

    /// Gets the contents of a Java string, in [modified UTF-8] encoding.
    ///
    /// The returned [MUTF8Chars] can be used to access the modified UTF-8 bytes,
    /// or to convert to a Rust string (which uses standard UTF-8 encoding).
    ///
    /// This entails calling the JNI function `GetStringUTFChars`.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    ///
    /// # Errors
    ///
    /// Returns an error if `obj` is `null`.
    #[deprecated(
        since = "0.22.0",
        note = "use JString::mutf8_chars or JString::to_string instead; this method is redundant and does not perform unsafe operations"
    )]
    pub fn get_string_unchecked<'any_local, StringRef>(
        &mut self,
        obj: StringRef,
    ) -> Result<MUTF8Chars<'any_local, StringRef>>
    where
        StringRef: AsRef<JString<'any_local>> + Reference,
    {
        MUTF8Chars::from_get_string_utf_chars(self, obj)
    }

    /// Gets the contents of a Java string, in [modified UTF-8] encoding.
    ///
    /// The returned [MUTF8Chars] can be used to access the modified UTF-8 bytes,
    /// or to convert to a Rust string (which uses standard UTF-8 encoding).
    ///
    /// This entails calling the JNI function `GetStringUTFChars`.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    ///
    /// # Errors
    ///
    /// Returns an error if `obj` is `null`.
    #[deprecated(
        since = "0.22.0",
        note = "use JString::mutf8_chars or JString::to_string instead"
    )]
    pub fn get_string<'any_local, StringRef>(
        &self,
        obj: StringRef,
    ) -> Result<MUTF8Chars<'any_local, StringRef>>
    where
        StringRef: AsRef<JString<'any_local>> + Reference,
    {
        MUTF8Chars::from_get_string_utf_chars(self, obj)
    }

    /// Create a new java string object from a rust string. This requires a
    /// re-encoding of rusts *real* UTF-8 strings to java's modified UTF-8
    /// format.
    pub fn new_string<S: AsRef<JNIStr>>(&mut self, from: S) -> Result<JString<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let ffi_str = from.as_ref();
        unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewStringUTF, ffi_str.as_ptr())
                .map(|s| JString::from_raw(s))
        }
    }

    /// Get the length of a [`JPrimitiveArray`] or [`JObjectArray`].
    #[deprecated(
        since = "0.22.0",
        note = "use JPrimitiveArray::len or JObjectArray::len instead. This method will be removed in a future version"
    )]
    pub fn get_array_length<'other_local, 'array>(
        &self,
        array: &'array impl AsJArrayRaw<'other_local>,
    ) -> Result<jsize> {
        let array = null_check!(array.as_jarray_raw(), "get_array_length array argument")?;
        let len: jsize = unsafe { jni_call_unchecked!(self, v1_1, GetArrayLength, array) };
        Ok(len)
    }

    /// Construct a new array holding objects in class `element_class`.
    ///
    /// All elements are initially set to `initial_element`.
    pub fn new_object_array<'other_local_1, 'other_local_2, T, U>(
        &mut self,
        length: jsize,
        element_class: T,
        initial_element: U,
    ) -> Result<JObjectArray<'local>>
    where
        T: Desc<'local, JClass<'other_local_2>>,
        U: AsRef<JObject<'other_local_1>>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let class = element_class.lookup(self)?;

        let array = unsafe {
            jni_call_check_ex_and_null_ret!(
                self,
                v1_1,
                NewObjectArray,
                length,
                class.as_ref().as_raw(),
                initial_element.as_ref().as_raw()
            )
            .map(|array| JObjectArray::from_raw(array))?
        };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        Ok(array)
    }

    /// Construct a new array holding objects with of type `E`.
    ///
    /// All elements are initially set to `initial_element`.
    pub fn new_object_type_array<'any_local, E>(
        &mut self,
        length: jsize,
        initial_element: impl AsRef<E::Kind<'any_local>>,
    ) -> Result<JObjectArray<'local, E::Kind<'local>>>
    where
        E: Reference,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let class = E::lookup_class(self, LoaderContext::None)?;

        let array = unsafe {
            jni_call_check_ex_and_null_ret!(
                self,
                v1_1,
                NewObjectArray,
                length,
                class.as_raw(),
                initial_element.as_ref().as_raw()
            )
            .map(|array| JObjectArray::from_raw(array))?
        };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        Ok(array)
    }

    /// Returns a local reference to an element of the [`JObjectArray`] `array`.
    #[deprecated(
        since = "0.22.0",
        note = "use JObjectArray::get_element instead. This method will be removed in a future version"
    )]
    pub fn get_object_array_element<'other_local, E: Reference + 'other_local>(
        &mut self,
        array: impl AsRef<JObjectArray<'other_local, E>>,
        index: usize,
    ) -> Result<E::Kind<'local>> {
        array.as_ref().get_element(index, self)
    }

    /// Sets an element of the [`JObjectArray`] `array`.
    #[deprecated(
        since = "0.22.0",
        note = "use JObjectArray::set_element instead. This method will be removed in a future version"
    )]
    pub fn set_object_array_element<'any_local_1, 'any_local_2, E: Reference + 'any_local_1>(
        &self,
        array: impl AsRef<JObjectArray<'any_local_1, E>>,
        index: usize,
        value: impl AsRef<E::Kind<'any_local_2>>,
    ) -> Result<()> {
        array.as_ref().set_element(index, value, self)
    }

    /// Create a new java byte array from a rust byte slice.
    pub fn byte_array_from_slice(&self, buf: &[u8]) -> Result<JByteArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let length = buf.len() as i32;
        let bytes = self.new_byte_array(length)?;
        unsafe {
            jni_call_unchecked!(
                self,
                v1_1,
                SetByteArrayRegion,
                bytes.as_raw(),
                0,
                length,
                buf.as_ptr() as *const i8
            );
        }
        Ok(bytes)
    }

    /// Converts a java byte array to a rust vector of bytes.
    pub fn convert_byte_array<'other_local>(
        &self,
        array: impl AsRef<JByteArray<'other_local>>,
    ) -> Result<Vec<u8>> {
        let array = array.as_ref().as_raw();
        let array = null_check!(array, "convert_byte_array array argument")?;
        let length = unsafe { jni_call_check_ex!(self, v1_1, GetArrayLength, array)? };
        let mut vec = vec![0u8; length as usize];
        unsafe {
            jni_call_unchecked!(
                self,
                v1_1,
                GetByteArrayRegion,
                array,
                0,
                length,
                vec.as_mut_ptr() as *mut i8
            );
        }
        Ok(vec)
    }

    /// Create a new java boolean array of supplied length.
    pub fn new_boolean_array(&self, length: jsize) -> Result<JBooleanArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewBooleanArray, length)
                .map(|array| JBooleanArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java byte array of supplied length.
    pub fn new_byte_array(&self, length: jsize) -> Result<JByteArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewByteArray, length)
                .map(|array| JByteArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java char array of supplied length.
    pub fn new_char_array(&self, length: jsize) -> Result<JCharArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewCharArray, length)
                .map(|array| JCharArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java short array of supplied length.
    pub fn new_short_array(&self, length: jsize) -> Result<JShortArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewShortArray, length)
                .map(|array| JShortArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java int array of supplied length.
    pub fn new_int_array(&self, length: jsize) -> Result<JIntArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewIntArray, length)
                .map(|array| JIntArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java long array of supplied length.
    pub fn new_long_array(&self, length: jsize) -> Result<JLongArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewLongArray, length)
                .map(|array| JLongArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java float array of supplied length.
    pub fn new_float_array(&self, length: jsize) -> Result<JFloatArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewFloatArray, length)
                .map(|array| JFloatArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java double array of supplied length.
    pub fn new_double_array(&self, length: jsize) -> Result<JDoubleArray<'local>> {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewDoubleArray, length)
                .map(|array| JDoubleArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Copy elements of the java boolean array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.Env.html#method.get_array_length
    #[deprecated(
        since = "0.22.0",
        note = "use JBooleanArray::get_region instead; this method will be removed in a future version"
    )]
    pub fn get_boolean_array_region<'other_local>(
        &self,
        array: impl AsRef<JBooleanArray<'other_local>>,
        start: jsize,
        buf: &mut [jboolean],
    ) -> Result<()> {
        unsafe {
            <jboolean as TypeArraySealed>::get_region(self, array.as_ref().as_raw(), start, buf)
        }
    }

    /// Copy elements of the java byte array from the `start` index to the `buf`
    /// slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.Env.html#method.get_array_length
    #[deprecated(
        since = "0.22.0",
        note = "use JByteArray::get_region instead; this method will be removed in a future version"
    )]
    pub fn get_byte_array_region<'other_local>(
        &self,
        array: impl AsRef<JByteArray<'other_local>>,
        start: jsize,
        buf: &mut [jbyte],
    ) -> Result<()> {
        unsafe { <jbyte as TypeArraySealed>::get_region(self, array.as_ref().as_raw(), start, buf) }
    }

    /// Copy elements of the java char array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.Env.html#method.get_array_length
    #[deprecated(
        since = "0.22.0",
        note = "use JCharArray::get_region instead; this method will be removed in a future version"
    )]
    pub fn get_char_array_region<'other_local>(
        &self,
        array: impl AsRef<JCharArray<'other_local>>,
        start: jsize,
        buf: &mut [jchar],
    ) -> Result<()> {
        unsafe { <jchar as TypeArraySealed>::get_region(self, array.as_ref().as_raw(), start, buf) }
    }

    /// Copy elements of the java short array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.Env.html#method.get_array_length
    #[deprecated(
        since = "0.22.0",
        note = "use JShortArray::get_region instead; this method will be removed in a future version"
    )]
    pub fn get_short_array_region<'other_local>(
        &self,
        array: impl AsRef<JShortArray<'other_local>>,
        start: jsize,
        buf: &mut [jshort],
    ) -> Result<()> {
        unsafe {
            <jshort as TypeArraySealed>::get_region(self, array.as_ref().as_raw(), start, buf)
        }
    }

    /// Copy elements of the java int array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.Env.html#method.get_array_length
    #[deprecated(
        since = "0.22.0",
        note = "use JIntArray::get_region instead; this method will be removed in a future version"
    )]
    pub fn get_int_array_region<'other_local>(
        &self,
        array: impl AsRef<JIntArray<'other_local>>,
        start: jsize,
        buf: &mut [jint],
    ) -> Result<()> {
        unsafe { <jint as TypeArraySealed>::get_region(self, array.as_ref().as_raw(), start, buf) }
    }

    /// Copy elements of the java long array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.Env.html#method.get_array_length
    #[deprecated(
        since = "0.22.0",
        note = "use JLongArray::get_region instead; this method will be removed in a future version"
    )]
    pub fn get_long_array_region<'other_local>(
        &self,
        array: impl AsRef<JLongArray<'other_local>>,
        start: jsize,
        buf: &mut [jlong],
    ) -> Result<()> {
        unsafe { <jlong as TypeArraySealed>::get_region(self, array.as_ref().as_raw(), start, buf) }
    }

    /// Copy elements of the java float array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.Env.html#method.get_array_length
    #[deprecated(
        since = "0.22.0",
        note = "use JFloatArray::get_region instead; this method will be removed in a future version"
    )]
    pub fn get_float_array_region<'other_local>(
        &self,
        array: impl AsRef<JFloatArray<'other_local>>,
        start: jsize,
        buf: &mut [jfloat],
    ) -> Result<()> {
        unsafe {
            <jfloat as TypeArraySealed>::get_region(self, array.as_ref().as_raw(), start, buf)
        }
    }

    /// Copy elements of the java double array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.Env.html#method.get_array_length
    #[deprecated(
        since = "0.22.0",
        note = "use JDoubleArray::get_region instead; this method will be removed in a future version"
    )]
    pub fn get_double_array_region<'other_local>(
        &self,
        array: impl AsRef<JDoubleArray<'other_local>>,
        start: jsize,
        buf: &mut [jdouble],
    ) -> Result<()> {
        unsafe {
            <jdouble as TypeArraySealed>::get_region(self, array.as_ref().as_raw(), start, buf)
        }
    }

    /// Copy the contents of the `buf` slice to the java boolean array at the
    /// `start` index.
    #[deprecated(
        since = "0.22.0",
        note = "use JBooleanArray::set_region instead; this method will be removed in a future version"
    )]
    pub fn set_boolean_array_region<'other_local>(
        &self,
        array: impl AsRef<JBooleanArray<'other_local>>,
        start: jsize,
        buf: &[jboolean],
    ) -> Result<()> {
        unsafe {
            <jboolean as TypeArraySealed>::set_region(self, array.as_ref().as_raw(), start, buf)
        }
    }

    /// Copy the contents of the `buf` slice to the java byte array at the
    /// `start` index.
    #[deprecated(
        since = "0.22.0",
        note = "use JByteArray::set_region instead; this method will be removed in a future version"
    )]
    pub fn set_byte_array_region<'other_local>(
        &self,
        array: impl AsRef<JByteArray<'other_local>>,
        start: jsize,
        buf: &[jbyte],
    ) -> Result<()> {
        unsafe { <jbyte as TypeArraySealed>::set_region(self, array.as_ref().as_raw(), start, buf) }
    }

    /// Copy the contents of the `buf` slice to the java char array at the
    /// `start` index.
    #[deprecated(
        since = "0.22.0",
        note = "use JCharArray::set_region instead; this method will be removed in a future version"
    )]
    pub fn set_char_array_region<'other_local>(
        &self,
        array: impl AsRef<JCharArray<'other_local>>,
        start: jsize,
        buf: &[jchar],
    ) -> Result<()> {
        unsafe { <jchar as TypeArraySealed>::set_region(self, array.as_ref().as_raw(), start, buf) }
    }

    /// Copy the contents of the `buf` slice to the java short array at the
    /// `start` index.
    #[deprecated(
        since = "0.22.0",
        note = "use JShortArray::set_region instead; this method will be removed in a future version"
    )]
    pub fn set_short_array_region<'other_local>(
        &self,
        array: impl AsRef<JShortArray<'other_local>>,
        start: jsize,
        buf: &[jshort],
    ) -> Result<()> {
        unsafe {
            <jshort as TypeArraySealed>::set_region(self, array.as_ref().as_raw(), start, buf)
        }
    }

    /// Copy the contents of the `buf` slice to the java int array at the
    /// `start` index.
    #[deprecated(
        since = "0.22.0",
        note = "use JIntArray::set_region instead; this method will be removed in a future version"
    )]
    pub fn set_int_array_region<'other_local>(
        &self,
        array: impl AsRef<JIntArray<'other_local>>,
        start: jsize,
        buf: &[jint],
    ) -> Result<()> {
        unsafe { <jint as TypeArraySealed>::set_region(self, array.as_ref().as_raw(), start, buf) }
    }

    /// Copy the contents of the `buf` slice to the java long array at the
    /// `start` index.
    #[deprecated(
        since = "0.22.0",
        note = "use JLongArray::set_region instead; this method will be removed in a future version"
    )]
    pub fn set_long_array_region<'other_local>(
        &self,
        array: impl AsRef<JLongArray<'other_local>>,
        start: jsize,
        buf: &[jlong],
    ) -> Result<()> {
        unsafe { <jlong as TypeArraySealed>::set_region(self, array.as_ref().as_raw(), start, buf) }
    }

    /// Copy the contents of the `buf` slice to the java float array at the
    /// `start` index.
    #[deprecated(
        since = "0.22.0",
        note = "use JFloatArray::set_region instead; this method will be removed in a future version"
    )]
    pub fn set_float_array_region<'other_local>(
        &self,
        array: impl AsRef<JFloatArray<'other_local>>,
        start: jsize,
        buf: &[jfloat],
    ) -> Result<()> {
        unsafe {
            <jfloat as TypeArraySealed>::set_region(self, array.as_ref().as_raw(), start, buf)
        }
    }

    /// Copy the contents of the `buf` slice to the java double array at the
    /// `start` index.
    #[deprecated(
        since = "0.22.0",
        note = "use JDoubleArray::set_region instead; this method will be removed in a future version"
    )]
    pub fn set_double_array_region<'other_local>(
        &self,
        array: impl AsRef<JDoubleArray<'other_local>>,
        start: jsize,
        buf: &[jdouble],
    ) -> Result<()> {
        unsafe {
            <jdouble as TypeArraySealed>::set_region(self, array.as_ref().as_raw(), start, buf)
        }
    }

    /// Convert a [`JMethodID`] into a [`JObject`] with the corresponding
    /// `java.lang.reflect.Method` or `java.lang.reflect.Constructor` instance.
    pub fn to_reflected_method<'other_local>(
        &mut self,
        class: impl Desc<'local, JClass<'other_local>>,
        method_id: impl Desc<'local, JMethodID>,
    ) -> Result<JObject<'local>> {
        // Safety: Rust type safety ensures that method_id is a JMethodID, while is_static is false
        unsafe { self.to_reflected_method_base(class, method_id, JMethodID::into_raw, false) }
    }

    /// Convert a [`JStaticMethodID`] into a [`JObject`] with the corresponding
    /// `java.lang.reflect.Method` instance.
    pub fn to_reflected_static_method<'other_local>(
        &mut self,
        class: impl Desc<'local, JClass<'other_local>>,
        method_id: impl Desc<'local, JStaticMethodID>,
    ) -> Result<JObject<'local>> {
        // Safety: Rust type safety ensures that method_id is a JStaticMethodID, while is_static is true
        unsafe { self.to_reflected_method_base(class, method_id, JStaticMethodID::into_raw, true) }
    }

    /// Convert a [`JMethodID`] or [`JStaticMethodID`] into a [`JObject`] with the
    /// corresponding `java.lang.reflect.Method` or `java.lang.reflect.Constructor`
    /// instance.
    ///
    /// The `to_jmethodid` function is used to convert the method ID type into
    /// a raw [`sys::jmethodID`].
    ///
    /// # Safety
    ///
    /// `is_static` must correctly indicate whether the method ID is for a static method. (The JNI
    /// spec does not define what happens if this is incorrect.)
    #[allow(clippy::wrong_self_convention)]
    unsafe fn to_reflected_method_base<'other_local, M>(
        &mut self,
        class: impl Desc<'local, JClass<'other_local>>,
        method_id: impl Desc<'local, M>,
        to_jmethodid: impl FnOnce(M) -> crate::sys::jmethodID,
        is_static: bool,
    ) -> Result<JObject<'local>>
    where
        M: Copy,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let class = class.lookup(self)?;

        let method_id = to_jmethodid(*method_id.lookup(self)?.as_ref());

        unsafe {
            jni_call_check_ex_and_null_ret!(
                self,
                v1_2,
                ToReflectedMethod,
                class.as_ref().as_raw(),
                method_id,
                is_static
            )
            .map(|jobject| JObject::from_raw(jobject))
        }
    }

    /// Get a field without checking the provided type against the actual field.
    ///
    /// # Safety
    ///
    /// - The `obj` must not be `null`
    /// - The `field` must be associated with the given `obj` (got from passing the `obj` to [Env::get_field_id])
    /// - The field must have the specified `ty` type.
    pub unsafe fn get_field_unchecked<'other_local, O, F>(
        &mut self,
        obj: O,
        field: F,
        ty: ReturnType,
    ) -> Result<JValueOwned<'local>>
    where
        O: AsRef<JObject<'other_local>>,
        F: Desc<'local, JFieldID>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use JavaType::{Array, Object, Primitive};

        let obj = obj.as_ref();
        let obj = null_check!(obj, "get_field_typed obj argument")?;

        let field = field.lookup(self)?.as_ref().into_raw();
        let obj = obj.as_raw();

        macro_rules! field {
            ($get_field:ident) => {{
                // Safety: No exceptions are defined for Get*Field and we assume
                // the caller knows that the field is valid
                unsafe {
                    JValueOwned::from(jni_call_unchecked!(self, v1_1, $get_field, obj, field))
                }
            }};
        }

        match ty {
            Object | Array => {
                let obj = unsafe {
                    jni_call_check_ex!(self, v1_1, GetObjectField, obj, field)
                        .map(|obj| JObject::from_raw(obj))?
                };
                Ok(obj.into())
            }
            Primitive(Char) => Ok(field!(GetCharField)),
            Primitive(Boolean) => Ok(field!(GetBooleanField)),
            Primitive(Short) => Ok(field!(GetShortField)),
            Primitive(Int) => Ok(field!(GetIntField)),
            Primitive(Long) => Ok(field!(GetLongField)),
            Primitive(Float) => Ok(field!(GetFloatField)),
            Primitive(Double) => Ok(field!(GetDoubleField)),
            Primitive(Byte) => Ok(field!(GetByteField)),
            Primitive(Void) => Err(Error::WrongJValueType("void", "see java field")),
        }
    }

    /// Set a field without any type checking.
    ///
    /// # Safety
    ///
    /// - The `obj` must not be `null`
    /// - The `field` must be associated with the given `obj` (got from passing the `obj` to [Env::get_field_id])
    /// - The field type must match the given `value` type.
    pub unsafe fn set_field_unchecked<'other_local, O, F>(
        &mut self,
        obj: O,
        field: F,
        value: JValue,
    ) -> Result<()>
    where
        O: AsRef<JObject<'other_local>>,
        F: Desc<'local, JFieldID>,
    {
        if let JValue::Void = value {
            return Err(Error::WrongJValueType("void", "see java field"));
        }

        let obj = obj.as_ref();
        let obj = null_check!(obj, "set_field_typed obj argument")?;

        let field = field.lookup(self)?.as_ref().into_raw();
        let obj = obj.as_raw();

        macro_rules! set_field {
            ($set_field:ident($val:expr)) => {{
                unsafe { jni_call_unchecked!(self, v1_1, $set_field, obj, field, $val) };
            }};
        }

        match value {
            JValue::Object(o) => set_field!(SetObjectField(o.as_raw())),
            JValue::Bool(b) => set_field!(SetBooleanField(b)),
            JValue::Char(c) => set_field!(SetCharField(c)),
            JValue::Short(s) => set_field!(SetShortField(s)),
            JValue::Int(i) => set_field!(SetIntField(i)),
            JValue::Long(l) => set_field!(SetLongField(l)),
            JValue::Float(f) => set_field!(SetFloatField(f)),
            JValue::Double(d) => set_field!(SetDoubleField(d)),
            JValue::Byte(b) => set_field!(SetByteField(b)),
            _ => (),
        };

        Ok(())
    }

    /// Get a field. Requires an object class lookup and a field id lookup
    /// internally.
    pub fn get_field<'other_local, O, N, S>(
        &mut self,
        obj: O,
        name: N,
        sig: S,
    ) -> Result<JValueOwned<'local>>
    where
        O: AsRef<JObject<'other_local>>,
        N: AsRef<JNIStr>,
        S: AsRef<JNIStr>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let obj = obj.as_ref();
        let class = self.get_object_class(obj)?.auto();

        let sig = sig.as_ref();
        let field_ty = JavaType::from_str(&sig.to_str())?;
        let field_id: JFieldID = Desc::<JFieldID>::lookup((&class, name, sig), self)?;

        // Safety: Since we have explicitly looked up the field ID based on the given
        // return type we have already validate that they match
        unsafe { self.get_field_unchecked(obj, field_id, field_ty) }
    }

    /// Set a field. Does the same lookups as `get_field` and ensures that the
    /// type matches the given value.
    pub fn set_field<'other_local, O, N, S>(
        &mut self,
        obj: O,
        name: N,
        sig: S,
        value: JValue,
    ) -> Result<()>
    where
        O: AsRef<JObject<'other_local>>,
        N: AsRef<JNIStr>,
        S: AsRef<JNIStr>,
    {
        let obj = obj.as_ref();
        let sig = sig.as_ref();
        let field_ty = JavaType::from_str(&sig.to_str())?;

        if value.java_type() != field_ty {
            return Err(Error::WrongJValueType(value.type_name(), "see java field"));
        }

        let class = self.get_object_class(obj)?.auto();

        // Safety: We have explicitly checked that the field type matches
        // the value type and the field ID is going to be looked up dynamically
        // based on the class, name and signature (so it's safe to use)
        unsafe { self.set_field_unchecked(obj, (&class, name, sig), value) }
    }

    /// Get a static field without checking the provided type against the actual
    /// field.
    ///
    /// # Safety
    ///
    /// - The `class` must not be null
    /// - The `field` must be associated with the given `class` (got from passing the `class` to [Env::get_static_field_id])
    /// - The field must have the specified `ty` type.
    pub unsafe fn get_static_field_unchecked<'other_local, C, F>(
        &mut self,
        class: C,
        field: F,
        ty: ReturnType,
    ) -> Result<JValueOwned<'local>>
    where
        C: Desc<'local, JClass<'other_local>>,
        F: Desc<'local, JStaticFieldID>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use JavaType::{Array, Object, Primitive};

        let class = class.lookup(self)?;
        let field = field.lookup(self)?;

        macro_rules! field {
            ($get_field:ident) => {{
                unsafe {
                    jni_call_check_ex!(
                        self,
                        v1_1,
                        $get_field,
                        class.as_ref().as_raw(),
                        field.as_ref().into_raw()
                    )?
                }
            }};
        }

        let ret = match ty {
            Primitive(Void) => Err(Error::WrongJValueType("void", "see java field")),
            Object | Array => {
                let obj = field!(GetStaticObjectField);
                let obj = unsafe { JObject::from_raw(obj) };
                Ok(JValueOwned::from(obj))
            }
            Primitive(Boolean) => Ok(field!(GetStaticBooleanField).into()),
            Primitive(Char) => Ok(field!(GetStaticCharField).into()),
            Primitive(Short) => Ok(field!(GetStaticShortField).into()),
            Primitive(Int) => Ok(field!(GetStaticIntField).into()),
            Primitive(Long) => Ok(field!(GetStaticLongField).into()),
            Primitive(Float) => Ok(field!(GetStaticFloatField).into()),
            Primitive(Double) => Ok(field!(GetStaticDoubleField).into()),
            Primitive(Byte) => Ok(field!(GetStaticByteField).into()),
        };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        ret
    }

    /// Set a static field. Requires a class lookup and a field id lookup internally.
    ///
    /// # Safety
    ///
    /// - The `class` must not be null
    /// - The `field` must be associated with the given `class` (got from passing the `class` to [Env::get_static_field_id])
    /// - The field type must match the given `value` type.
    pub unsafe fn set_static_field_unchecked<'other_local, C, F>(
        &mut self,
        class: C,
        field: F,
        value: JValue,
    ) -> Result<()>
    where
        C: Desc<'local, JClass<'other_local>>,
        F: Desc<'local, JStaticFieldID>,
    {
        let class = class.lookup(self)?;
        let field = field.lookup(self)?;

        macro_rules! set_field {
            ($set_field:ident($val:expr)) => {{
                unsafe {
                    jni_call_unchecked!(
                        self,
                        v1_1,
                        $set_field,
                        class.as_ref().as_raw(),
                        field.as_ref().into_raw(),
                        $val
                    );
                }
            }};
        }

        match value {
            JValue::Object(v) => set_field!(SetStaticObjectField(v.as_raw())),
            JValue::Byte(v) => set_field!(SetStaticByteField(v)),
            JValue::Char(v) => set_field!(SetStaticCharField(v)),
            JValue::Short(v) => set_field!(SetStaticShortField(v)),
            JValue::Int(v) => set_field!(SetStaticIntField(v)),
            JValue::Long(v) => set_field!(SetStaticLongField(v)),
            JValue::Bool(v) => set_field!(SetStaticBooleanField(v)),
            JValue::Float(v) => set_field!(SetStaticFloatField(v)),
            JValue::Double(v) => set_field!(SetStaticDoubleField(v)),
            _ => (),
        }

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        Ok(())
    }

    /// Get a static field. Requires a class lookup and a field id lookup
    /// internally.
    pub fn get_static_field<'other_local, C, N, S>(
        &mut self,
        class: C,
        name: N,
        sig: S,
    ) -> Result<JValueOwned<'local>>
    where
        C: Desc<'local, JClass<'other_local>>,
        N: AsRef<JNIStr>,
        S: AsRef<JNIStr>,
    {
        // Runtime check that the 'local reference lifetime will be tied to
        // Env lifetime for the top JNI stack frame
        self.assert_top();
        let sig = sig.as_ref();
        let field_ty = JavaType::from_str(&sig.to_str())?;

        // go ahead and look up the class since we'll need that for the next call.
        let class = class.lookup(self)?;

        // SAFETY: We have verified that `class`, `field`, `sig` and `ty` are valid
        unsafe {
            self.get_static_field_unchecked(class.as_ref(), (class.as_ref(), name, sig), field_ty)
        }
    }

    /// Set a static field. Requires a class lookup and a field id lookup internally.
    pub fn set_static_field<'other_local, C, N, S>(
        &mut self,
        class: C,
        name: N,
        sig: S,
        value: JValue,
    ) -> Result<()>
    where
        C: Desc<'local, JClass<'other_local>>,
        N: AsRef<JNIStr>,
        S: AsRef<JNIStr>,
    {
        let sig = sig.as_ref();
        let field_ty = JavaType::from_str(&sig.to_str())?;

        if value.java_type() != field_ty {
            return Err(Error::WrongJValueType(value.type_name(), "see java field"));
        }

        let class = class.lookup(self)?;

        // Safety: We have explicitly checked that the field type matches the value type.
        unsafe {
            self.set_static_field_unchecked(class.as_ref(), (class.as_ref(), name, sig), value)
        }
    }

    /// Looks up the field ID for the given field name and takes the monitor
    /// lock on the given object so the field can be updated without racing
    /// with other Java threads
    fn lock_rust_field<'other_local, O, S>(
        &'_ self,
        obj: O,
        field: S,
    ) -> Result<(MonitorGuard<'_>, JFieldID)>
    where
        O: AsRef<JObject<'other_local>>,
        S: AsRef<JNIStr>,
    {
        // Note: although the returned Monitor is associated with a lifetime, this API doesn't need
        // a `&mut self` reference because we don't need to create and return a new local reference
        // (Monitors aren't JNI types that are owned by a JNI stack frame).

        // We also don't assert that `self.level == JavaVM::thread_attach_guard_level()` since the
        // returned monitor is associated with the current thread and there's no reason you can't
        // lock with a reference that's not from the top stack frame.

        // Since `Desc::lookup` may need to create a temporary local reference for the object class
        // (which we don't want to leak), we push a new stack frame that we can get a mutable
        // reference for.

        // Panic: The `&self` reference is enough to prove that `JavaVM::singleton` must have been
        // initialized and won't panic.
        JavaVM::singleton()?.with_env(|env| {
            let obj = obj.as_ref();
            let class = env.get_object_class(obj)?;
            let field_id: JFieldID = Desc::<JFieldID>::lookup((&class, &field, c"J"), env)?;
            let guard = self.lock_obj(obj)?;
            Ok((guard, field_id))
        })
    }

    /// Surrenders ownership of a Rust value to Java.
    ///
    /// This requires an object with a `long` field to store the pointer.
    ///
    /// In Java the property may look like:
    /// ```java
    /// private long myRustValueHandle = 0;
    /// ```
    ///
    /// Or, in Kotlin the property may look like:
    /// ```java
    /// private var myRustValueHandle: Long = 0
    /// ```
    ///
    /// _Note that `private` properties are accessible to JNI which may be
    /// preferable to avoid exposing the handles to more code than necessary
    /// (since the handles are usually only meaningful to Rust code)_.
    ///
    /// The Rust value will be implicitly wrapped in a `Box<Mutex<T>>`.
    ///
    /// The Java object will be locked while changing the field value.
    ///
    /// # Safety
    ///
    /// This will lead to undefined behaviour if the the specified field
    /// doesn't have a type of `long`.
    ///
    /// It's important to note that using this API will leak memory if
    /// [`Self::take_rust_field`] is never called so that the Rust type may be
    /// dropped.
    ///
    /// One suggestion that may help ensure that a set Rust field will be
    /// cleaned up later is for the Java object to implement `Closeable` and let
    /// people use a `use` block (Kotlin) or `try-with-resources` (Java).
    ///
    /// **DO NOT** make a copy of the handle stored in one of these fields
    /// since that could lead to a use-after-free error if the Rust type is
    /// taken and dropped multiple times from Rust. If you need to copy an
    /// object with one of these fields then the field should be zero
    /// initialized in the copy.
    pub unsafe fn set_rust_field<'other_local, O, S, T>(
        &self,
        obj: O,
        field: S,
        rust_object: T,
    ) -> Result<()>
    where
        O: AsRef<JObject<'other_local>>,
        S: AsRef<JNIStr>,
        T: Send + 'static,
    {
        let (_guard, field_id) = self.lock_rust_field(&obj, &field)?;

        // It's OK that we don't push a new stack frame here since we know we are dealing with a
        // `jlong` field and since we have already looked up the field ID then we also know that
        // get_field_unchecked and set_field_unchecked don't need to create any local references.

        // Panic: The `&self` reference is enough to prove that `JavaVM::singleton` must have been
        // initialized and won't panic.
        JavaVM::singleton()?.with_env_current_frame(|env| {
            // Safety: the requirement that the given field must be a `long` is
            // documented in the 'Safety' section of this function
            unsafe {
                let field_ptr = env
                    .get_field_unchecked(&obj, field_id, ReturnType::Primitive(Primitive::Long))?
                    .j()? as *mut Mutex<T>;
                if !field_ptr.is_null() {
                    return Err(Error::FieldAlreadySet(field.as_ref().to_str().into()));
                }
            }

            let mbox = Box::new(::std::sync::Mutex::new(rust_object));
            let ptr: *mut Mutex<T> = Box::into_raw(mbox);

            // Safety: the requirement that the given field must be a `long` is
            // documented in the 'Safety' section of this function
            unsafe { env.set_field_unchecked(obj, field_id, (ptr as crate::sys::jlong).into()) }
        })
    }

    /// Gets a lock on a Rust value that's been given to a Java object.
    ///
    /// Java still retains ownership and [`Self::take_rust_field`] will still
    /// need to be called at some point.
    ///
    /// The Java object will be locked before reading the field value but the
    /// Java object lock will be released after the Rust `Mutex` lock for the
    /// field value has been taken (i.e the Java object won't be locked once
    /// this function returns).
    ///
    /// # Safety
    ///
    /// This will lead to undefined behaviour if the the specified field
    /// doesn't have a type of `long`.
    ///
    /// If the field contains a non-zero value then it is assumed to be a valid
    /// pointer that was set via `set_rust_field` and will lead to undefined
    /// behaviour if that is not true.
    pub unsafe fn get_rust_field<'other_local, O, S, T>(
        &'_ self,
        obj: O,
        field: S,
    ) -> Result<MutexGuard<'_, T>>
    where
        O: AsRef<JObject<'other_local>>,
        S: AsRef<JNIStr>,
        T: Send + 'static,
    {
        let (_guard, field_id) = self.lock_rust_field(&obj, &field)?;

        // Reference Leaks:
        //
        // It's ok that we don't push a new stack frame here since we know we are dealing with a
        // `jlong` field and since we have already looked up the field ID then we also know that
        // get_field_unchecked doesn't need to create any local references.

        // Panic: The `&self` reference is enough to prove that `JavaVM::singleton` must have been
        // initialized and won't panic.
        JavaVM::singleton()?.with_env_current_frame(|env| {
            // Safety: the requirement that the given field must be a `long` is
            // documented in the 'Safety' section of this function
            unsafe {
                let field_ptr = env
                    .get_field_unchecked(obj, field_id, ReturnType::Primitive(Primitive::Long))?
                    .j()? as *mut Mutex<T>;
                null_check!(field_ptr, "rust value from Java")?;
                // dereferencing is safe, because we checked it for null
                Ok((*field_ptr).lock().unwrap())
            }
        })
    }

    /// Take a Rust field back from Java.
    ///
    /// It sets the field to a null pointer to signal that it's empty.
    ///
    /// The Java object will be locked before taking the field value.
    ///
    /// # Safety
    ///
    /// This will lead to undefined behaviour if the the specified field
    /// doesn't have a type of `long`.
    ///
    /// If the field contains a non-zero value then it is assumed to be a valid
    /// pointer that was set via `set_rust_field` and will lead to undefined
    /// behaviour if that is not true.
    pub unsafe fn take_rust_field<'other_local, O, S, T>(&self, obj: O, field: S) -> Result<T>
    where
        O: AsRef<JObject<'other_local>>,
        S: AsRef<JNIStr>,
        T: Send + 'static,
    {
        let (_guard, field_id) = self.lock_rust_field(&obj, &field)?;

        // Reference Leaks:
        //
        // It's ok that we don't push a new stack frame here since we know we are dealing with a
        // `jlong` field and since we have already looked up the field ID then we also know that
        // get_field_unchecked doesn't need to create any local references.

        // Panic: The `&self` reference is enough to prove that `JavaVM::singleton` must have been
        // initialized and won't panic.
        JavaVM::singleton()?.with_env_current_frame(|env| {
            // Safety: the requirement that the given field must be a `long` is
            // documented in the 'Safety' section of this function
            let mbox = unsafe {
                let ptr = env
                    .get_field_unchecked(&obj, field_id, ReturnType::Primitive(Primitive::Long))?
                    .j()? as *mut Mutex<T>;

                null_check!(ptr, "rust value from Java")?;
                Box::from_raw(ptr)
            };

            // attempt to acquire the lock. This prevents us from consuming the
            // mutex if there's an outstanding lock. No one else will be able to
            // get a new one as long as we're in the guarded scope.
            drop(mbox.try_lock()?);

            // Safety: the requirement that the given field must be a `long` is
            // documented in the 'Safety' section of this function
            unsafe {
                env.set_field_unchecked(obj, field_id, (0 as sys::jlong).into())?;
            }

            Ok(mbox.into_inner().unwrap())
        })
    }

    /// Lock a Java object. The MonitorGuard that this returns is responsible
    /// for ensuring that it gets unlocked.
    pub fn lock_obj<'other_local, O>(&self, obj: O) -> Result<MonitorGuard<'local>>
    where
        O: AsRef<JObject<'other_local>>,
    {
        // Note: although the returned Monitor is associated with a lifetime, we
        // don't need a `&mut self` reference and we don't assert that
        // `self.level == JavaVM::thread_attach_guard_level()` since the
        // returned monitor is associated with the current thread and is not a
        // local reference.

        let inner = obj.as_ref().as_raw();
        let res = unsafe { jni_call_unchecked!(self, v1_1, MonitorEnter, inner) };
        jni_error_code_to_result(res)?;

        Ok(MonitorGuard {
            obj: inner,
            life: Default::default(),
        })
    }

    /// Returns the Java VM interface.
    pub fn get_java_vm(&self) -> JavaVM {
        // This avoids calling JNI if we already know the VM pointer
        JavaVM::from_env(self)
    }

    /// Ensures that at least a given number of local references can be created
    /// in the current thread.
    pub fn ensure_local_capacity(&self, capacity: usize) -> Result<()> {
        let capacity: jint = capacity
            .try_into()
            .map_err(|_| Error::JniCall(JniError::InvalidArguments))?;
        // Safety:
        // - jni-rs required JNI_VERSION > 1.2
        // - we have ensured capacity is >= 0
        // - EnsureLocalCapacity has no documented exceptions that it throws
        let res = unsafe { jni_call_unchecked!(self, v1_2, EnsureLocalCapacity, capacity) };
        jni_error_code_to_result(res)?;
        Ok(())
    }

    // FIXME: this API shouldn't need a `&mut self` reference since it doesn't return a local reference
    // (currently it just needs the `&mut self` for the sake of `Desc<JClass>::lookup`)
    //
    /// Bind function pointers to native methods of class according to method
    /// name and signature.
    ///
    /// For details see
    /// [documentation](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#RegisterNatives).
    ///
    /// # Safety
    ///
    /// All the native method pointers must be valid, non-null pointers to
    /// functions that match the signature of the corresponding Java method.
    ///
    /// All of the pointers must remain valid for the lifetime of the class unless
    /// they are unregistered, via [`Self::unregister_native_methods`].
    pub unsafe fn register_native_methods<'other_local, T>(
        &mut self,
        class: T,
        methods: &[NativeMethod],
    ) -> Result<()>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let jni_native_methods: Vec<JNINativeMethod> = methods
            .iter()
            .map(|nm| JNINativeMethod {
                name: nm.name.as_ptr() as *mut c_char,
                signature: nm.sig.as_ptr() as *mut c_char,
                fnPtr: nm.fn_ptr,
            })
            .collect();
        let res = unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                RegisterNatives,
                class.as_ref().as_raw(),
                jni_native_methods.as_ptr(),
                jni_native_methods.len() as jint
            )?
        };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        jni_error_code_to_result(res)
    }

    /// Unbind all native methods of class.
    pub fn unregister_native_methods<'other_local, T>(&mut self, class: T) -> Result<()>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let res =
            unsafe { jni_call_check_ex!(self, v1_1, UnregisterNatives, class.as_ref().as_raw())? };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        jni_error_code_to_result(res)
    }

    /// Returns an [`AutoElements`] to access the elements of the given Java `array`.
    ///
    /// # Safety
    ///
    /// See: [JPrimitiveArray::get_elements] for more details
    #[deprecated(
        since = "0.22.0",
        note = "use JPrimitiveArray::get_elements instead. This API will be removed in a future version"
    )]
    pub unsafe fn get_array_elements<'array_local, T, TArrayRef>(
        &self,
        array: TArrayRef,
        mode: ReleaseMode,
    ) -> Result<AutoElements<'array_local, T, TArrayRef>>
    where
        T: TypeArray,
        TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + Reference,
    {
        AutoElements::new(self, array, mode)
    }

    /// Returns an [`AutoElementsCritical`] to access the elements of the given Java `array`.
    ///
    /// # Safety
    ///
    /// See: [JPrimitiveArray::get_elements_critical] for more details
    #[deprecated(
        since = "0.22.0",
        note = "use JPrimitiveArray::get_elements_critical instead. This API will be removed in a future version"
    )]
    pub unsafe fn get_array_elements_critical<'array_local, T, TArrayRef>(
        &self,
        array: TArrayRef,
        mode: ReleaseMode,
    ) -> Result<AutoElementsCritical<'array_local, T, TArrayRef>>
    where
        T: TypeArray,
        TArrayRef: AsRef<JPrimitiveArray<'array_local, T>> + Reference,
    {
        AutoElementsCritical::new(self, array, mode)
    }
}

/// The outcome of an operation that may succeed, fail with an error, or panic.
///
/// This enum is used to encapsulate the result of operations within native methods
/// where extra care is needed to handle errors and panics gracefully before
/// returning a value to the Java environment.
#[derive(Debug)]
pub enum Outcome<T, E> {
    /// Contains the success value
    Ok(T),
    /// Contains the error value
    Err(E),
    /// Contains the panic value
    Panic(Box<dyn std::any::Any + Send + 'static>),
}

/// An opaque wrapper around an [`Outcome<T, Error>`] that supports mapping
/// errors within native methods (via [`ErrorPolicy`]), with access to an
/// [`Env`] reference.
///
/// This is returned by [`EnvUnowned::with_env`] and is designed for use within
/// native method implementations where unwinding can't be caught by the JVM and
/// will abort the process.
///
/// Once you have an [`EnvOutcome`] you must resolve it to a value that can be
/// returned to the Java environment.
///
/// An [`EnvOutcome`] is resolved into a return value by calling
/// [`EnvOutcome::resolve`] or [`EnvOutcome::resolve_with`] with the help of an
/// [`ErrorPolicy`] that can customize how errors and panics are handled before
/// returning a value.
///
/// There are several built-in error policies in the `jni::errors` module, and
/// you can also implement your own by implementing the [`ErrorPolicy`] trait.
/// See:
///
/// - [`ThrowRuntimeExAndDefault`]: an `ErrorPolicy` that throws any error as a
///   `java.lang.RuntimeException` and returns a default value.
/// - [`LogErrorAndDefault`]: an `ErrorPolicy` that logs errors and returns a
///   default value.
/// - [`LogContextErrorAndDefault`]: an `ErrorPolicy` that logs errors, with a
///   given context string, and returns a default value.
///
/// # Examples
///
/// ## Map Rust errors and panics to Java exceptions:
/// ```rust,no_run
/// # use jni::objects::{JObject, JString};
/// # use jni::errors::ThrowRuntimeExAndDefault;
/// #[no_mangle]
/// pub extern "system" fn Java_com_example_MyClass_myNativeMethod<'caller>(
///     mut unowned_env: jni::EnvUnowned<'caller>,
///     _this: JObject<'caller>,
///     arg: JString<'caller>,
/// ) -> JObject<'caller> {
///     unowned_env.with_env(|env| -> jni::errors::Result<_> {
///         // Use `env` to call Java methods or access fields.
///         Ok(JObject::null())
///     }).resolve::<ThrowRuntimeExAndDefault>()
/// }
/// ```
///
/// ## Log errors with a context string and return a default value:
/// ```rust,no_run
/// # use jni::objects::{JObject, JString};
/// # use jni::errors::LogContextErrorAndDefault;
/// #[no_mangle]
/// pub extern "system" fn Java_com_example_MyClass_myNativeMethod<'caller>(
///    mut unowned_env: jni::EnvUnowned<'caller>,
///    _this: JObject<'caller>,
///    arg: JString<'caller>,
/// ) -> JObject<'caller> {
///    unowned_env.with_env(|env| -> jni::errors::Result<_> {
///       // Use `env` to call Java methods or access fields.
///       Ok(JObject::null())
///   }).resolve_with::<LogContextErrorAndDefault, _>(|| {
///     format!("in myNativeMethod with arg: {arg}")
///   })
/// }
/// ```
#[must_use = "The outcome must be resolved to a value that can be returned to Java. See ::resolve or ::resolve_with"]
#[derive(Debug)]
pub struct EnvOutcome<'local, T, E> {
    raw_env: *mut crate::sys::JNIEnv,
    outcome: Outcome<T, E>,
    _invariant: std::marker::PhantomData<&'local mut ()>, // !Send/!Sync + tie to frame
}

impl<'local, T, E> EnvOutcome<'local, T, E> {
    pub(crate) fn new(raw_env: *mut crate::sys::JNIEnv, outcome: Outcome<T, E>) -> Self {
        Self {
            raw_env,
            outcome,
            _invariant: Default::default(),
        }
    }

    /// No captures (fast path).
    pub fn resolve<'native_method, P>(self) -> T
    where
        P: ErrorPolicy<T, E, Captures<'local, 'native_method> = ()>,
        T: Default + 'native_method,
        'local: 'native_method,
    {
        self.resolve_with::<P, _>(|| ())
    }

    /// Builder can borrow locals (via 'm) and use the temporary Env<'cf>.
    pub fn resolve_with<'native_method, P, F>(self, capture: F) -> T
    where
        P: ErrorPolicy<T, E>,
        T: Default + 'native_method,
        F: FnOnce() -> <P as ErrorPolicy<T, E>>::Captures<'local, 'native_method>,
        'local: 'native_method,
    {
        // Rebuild Env<'cf> (AttachGuard hidden), then build captures and map once.
        // All calls are guarded with catch_unwind; fall back to last_resort on failure.
        self.resolve_inner::<P, F>(capture)
    }

    fn resolve_inner<'native_method, P, F>(self, capture: F) -> T
    where
        P: ErrorPolicy<T, E>,
        T: Default + 'native_method,
        F: FnOnce() -> <P as ErrorPolicy<T, E>>::Captures<'local, 'native_method>,
        'local: 'native_method,
    {
        unsafe {
            let mut guard: AttachGuard<'local> = AttachGuard::from_unowned(self.raw_env);
            let env = guard.borrow_env_mut();

            match self.outcome {
                Outcome::Ok(t) => t,
                Outcome::Err(e) => {
                    let mut cap = capture();
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        match P::on_error(env, &mut cap, e) {
                            Ok(t) => t,
                            Err(err) => P::on_internal_jni_error(&mut cap, err),
                        }
                    }))
                    .unwrap_or_else(|payload| P::on_internal_panic(&mut cap, payload))
                }
                Outcome::Panic(p) => {
                    let mut cap = capture();
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        match P::on_panic(env, &mut cap, p) {
                            Ok(t) => t,
                            Err(err) => P::on_internal_jni_error(&mut cap, err),
                        }
                    }))
                    .unwrap_or_else(|payload| P::on_internal_panic(&mut cap, payload))
                }
            }
        }
    }

    /// Consumes the `EnvOutcome`, returning the underlying `Outcome<T, E>`.
    pub fn into_outcome(self) -> Outcome<T, E> {
        self.outcome
    }
}

/// Represents an external (unowned) JNI stack frame and thread attachment that
/// was passed to a native method call.
///
/// This is an FFI safe wrapper around a [`crate::sys::JNIEnv`] pointer that has
/// been passed as the first argument to a native method call, and represents
/// an implicit JNI thread attachment.
///
/// For example, you can use it with a native method implementation like this:
/// ```rust,no_run
/// # use jni::objects::{JObject, JString};
/// # use jni::errors::ThrowRuntimeExAndDefault;
/// #[no_mangle]
/// pub extern "system" fn Java_com_example_MyClass_myNativeMethod<'caller>(
///     mut unowned_env: jni::EnvUnowned<'caller>,
///     _this: JObject<'caller>,
///     arg: JString<'caller>,
/// ) -> JObject<'caller> {
///     unowned_env.with_env(|env| -> jni::errors::Result<_> {
///         // Use `env` to call Java methods or access fields.
///         Ok(JObject::null())
///     }).resolve::<ThrowRuntimeExAndDefault>()
/// }
/// ```
#[repr(transparent)]
#[derive(Debug)]
pub struct EnvUnowned<'local> {
    ptr: *mut jni_sys::JNIEnv,
    _lifetime: std::marker::PhantomData<&'local ()>,
}

impl<'local> EnvUnowned<'local> {
    /// Runs a closure with a [`Env`] based on an unowned JNI thread attachment
    /// associated with an external JNI stack frame.
    ///
    /// This API is specifically intended to be used within native/foreign Java
    /// method implementations in cases where you have named the lifetime for
    /// the caller's JNI stack frame.
    ///
    /// It returns an [`EnvOutcome`] that supports mapping errors with access to
    /// an [`Env`], so you may choose to throw errors as exceptions.
    ///
    /// To avoid the risk of unwinding into the JVM (which will abort the
    /// process) this API wraps the closure in a [`catch_unwind`] to catch any
    /// panics. Any panic can be handled via the [`ErrorPolicy`] given to
    /// [`EnvOutcome::resolve`] or handled directly via [`EnvOutcome::into_outcome`].
    ///
    /// Note: This API does not create a new JNI stack frame, since the JVM will
    /// clean up the JNI stack frame when the native method returns.
    ///
    /// Note: This API returns an [`EnvOutcome`]
    pub fn with_env<F, T, E>(&mut self, f: F) -> EnvOutcome<'local, T, E>
    where
        F: FnOnce(&mut Env<'local>) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        // Safety: we trust that self.ptr a valid, non-null pointer
        let mut guard: AttachGuard<'local> = unsafe { AttachGuard::from_unowned(self.ptr) };
        let env = guard.borrow_env_mut();
        let result = catch_unwind(AssertUnwindSafe(|| f(env)));
        let outcome = match result {
            Ok(ret) => match ret {
                Ok(t) => Outcome::Ok(t),
                Err(e) => Outcome::Err(e),
            },
            Err(payload) => Outcome::Panic(payload),
        };
        EnvOutcome::new(self.ptr, outcome)
    }

    /// Runs a closure with a [`Env`] based on an unowned JNI thread attachment
    /// associated with an external JNI stack frame.
    ///
    /// This API is specifically intended to be used within native/foreign Java
    /// method implementations in cases where you have named the lifetime for
    /// the caller's JNI stack frame.
    ///
    /// Since it would lead to undefined behaviour to allow Rust code to unwind
    /// across a native method call boundary, you probably want to use
    /// [`EnvUnowned::with_env`] instead, which will wrap the closure
    /// in a `catch_unwind` to catch any panics.
    ///
    /// Note: This API does not create a new JNI stack frame, which is normally
    /// what you want when implementing a native method, since the JVM will
    /// clean up the JNI stack frame when the native method returns.
    pub fn with_env_no_catch<F, T, E>(&mut self, f: F) -> EnvOutcome<'local, T, E>
    where
        F: FnOnce(&mut Env<'local>) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        // Safety: we trust that self.ptr a valid, non-null pointer
        let mut guard: AttachGuard<'local> = unsafe { AttachGuard::from_unowned(self.ptr) };
        let result = f(guard.borrow_env_mut());
        match result {
            Ok(t) => EnvOutcome::new(self.ptr, Outcome::Ok(t)),
            Err(e) => EnvOutcome::new(self.ptr, Outcome::Err(e)),
        }
    }

    /// Creates a new `EnvUnowned` from a raw [`crate::sys::JNIEnv`] pointer.
    ///
    /// It should be very uncommon to use this method directly, but could be
    /// useful if you are given a raw [`crate::sys::JNIEnv`] pointer that you
    /// know represents a valid JNI attachment for the current thread.
    ///
    /// If you are implementing a native method in Rust though, you should
    /// prefer to use the [`EnvUnowned`] type as the first argument to your
    /// native method and avoid the need to use a raw pointer.
    ///
    /// If you have a raw [`crate::sys::JNIEnv`] pointer, this API should be
    /// marginally safer than using [`crate::AttachGuard`] manually
    /// since since the attach guard management will be hidden within the
    /// [`Self::with_env`] and [`Self::with_env_no_catch`] methods.
    ///
    /// Beware that [`Self::with_env`] and [`Self::with_env_no_catch`] will not
    /// create a new JNI stack frame, so if you are not implementing a native
    /// method with a JNI stack frame that will be cleaned up on return, you may
    /// need to consider the risk of leaking local references into the current
    /// stack frame (JNI references are only cleaned up when the stack frame is
    /// popped)
    ///
    /// # Safety
    ///
    /// The pointer must be a valid, non-null pointer to a `jni_sys::JNIEnv`
    /// that represents an attachment of the current thread to a Java VM.
    ///
    /// The assigned lifetime must not outlive the JNI stack frame that owns the
    /// `Env` pointer. For example it would _never_ be safe to use `'static`.
    pub unsafe fn from_raw(ptr: *mut jni_sys::JNIEnv) -> Self {
        assert!(!ptr.is_null(), "EnvUnowned pointer must not be null");
        Self {
            ptr,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Returns the raw pointer to the underlying [`crate::sys::JNIEnv`].
    pub fn as_raw(&self) -> *mut jni_sys::JNIEnv {
        self.ptr
    }

    /// Consumes the [`EnvUnowned`] and returns the raw pointer to the underlying [`crate::sys::JNIEnv`].
    pub fn into_raw(self) -> *mut jni_sys::JNIEnv {
        self.ptr
    }
}

#[derive(Debug)]
/// Native method descriptor.
pub struct NativeMethod {
    /// Name of method.
    pub name: JNIString,
    /// Method signature.
    pub sig: JNIString,
    /// Pointer to native function with signature
    /// `fn(env: Env, class: JClass, ...arguments according to sig) -> RetType`
    /// for static methods or
    /// `fn(env: Env, object: JObject, ...arguments according to sig) -> RetType`
    /// for instance methods.
    pub fn_ptr: *mut c_void,
}

/// Guard for a lock on a java object. This gets returned from the `lock_obj`
/// method.
#[derive(Debug)]
pub struct MonitorGuard<'local> {
    obj: sys::jobject,
    life: PhantomData<&'local ()>,
}

impl Drop for MonitorGuard<'_> {
    fn drop(&mut self) {
        // Panics:
        //
        // The first `.expect()` is OK because the `&self` reference is enough to prove that
        // `JavaVM::singleton` must have been initialized and won't panic. (A MonitorGuard can only
        // be created with a Env reference)
        //
        // The second `.expect()` is OK because the guard is associated with a Env lifetime, so
        // it logically shouldn't be possible for the thread to become detached before the monitor
        // is dropped.
        JavaVM::singleton()
            .expect("JavaVM singleton must be initialized")
            .with_env_current_frame(|env| -> crate::errors::Result<()> {
                // Safety:
                //
                // This relies on `MonitorGuard` not being `Send` to maintain the
                // invariant that "The current thread must be the owner of the monitor
                // associated with the underlying Java object referred to by obj"
                //
                // This also means we can assume the `IllegalMonitorStateException`
                // exception can't be thrown due to the current thread not owning
                // the monitor.
                let res = unsafe { jni_call_unchecked!(env, v1_1, MonitorExit, self.obj) };
                if let Err(err) = jni_error_code_to_result(res) {
                    log::error!("error releasing java monitor: {err}");
                }

                Ok(())
            })
            .expect("MonitorGuard dropped on detached thread");
    }
}

#[cfg(test)]
static_assertions::assert_not_impl_any!(MonitorGuard: Send);
