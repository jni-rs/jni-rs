use std::{
    convert::TryInto,
    marker::PhantomData,
    os::raw::{c_char, c_void},
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    ptr, str,
    str::FromStr,
    sync::{Mutex, MutexGuard},
};

use jni_sys::jobject;
use once_cell::sync::OnceCell;

use crate::{
    descriptors::Desc,
    errors::*,
    objects::{
        AutoElements, AutoElementsCritical, AutoLocal, GlobalRef, JByteBuffer, JClass, JFieldID,
        JList, JMap, JMethodID, JObject, JStaticFieldID, JStaticMethodID, JString, JThrowable,
        JValue, JValueOwned, ReleaseMode, TypeArray, WeakRef,
    },
    signature::{JavaType, Primitive, TypeSignature},
    strings::{JNIStr, JNIString, JavaStr},
    sys::{
        self, jboolean, jbyte, jchar, jdouble, jfloat, jint, jlong, jshort, jsize, jvalue,
        JNINativeMethod,
    },
    JNIVersion, JavaVM,
};
use crate::{
    errors::Error::JniCall,
    objects::{
        JBooleanArray, JByteArray, JCharArray, JDoubleArray, JFloatArray, JIntArray, JLongArray,
        JObjectArray, JPrimitiveArray, JShortArray,
    },
};
use crate::{objects::AsJArrayRaw, signature::ReturnType};

/// FFI-compatible JNIEnv struct. You can safely use this as the JNIEnv argument
/// to exported methods that will be called by java. This is where most of the
/// magic happens. All methods on this object are wrappers around JNI functions,
/// so the documentation on their behavior is still pretty applicable.
///
/// # Exception handling
///
/// Since we're calling into the JVM with this, many methods also have the
/// potential to cause an exception to get thrown. If this is the case, an `Err`
/// result will be returned with the error kind `JavaException`. Note that this
/// will _not_ clear the exception - it's up to the caller to decide whether to
/// do so or to let it continue being thrown.
///
/// # References and Lifetimes
///
/// As in C JNI, interactions with Java objects happen through <dfn>references</dfn>, either local
/// or global, represented by [`JObject`] and [`GlobalRef`] respectively. So long as there is at
/// least one such reference to a Java object, the JVM garbage collector will not reclaim it.
///
/// <dfn>Global references</dfn> exist until deleted. Deletion occurs when the `GlobalRef` is
/// dropped.
///
/// <dfn>Local references</dfn> belong to a local reference frame, and exist until
/// [deleted][JNIEnv::delete_local_ref] or until the local reference frame is exited. A <dfn>local
/// reference frame</dfn> is entered when a native method is called from Java, or when Rust code
/// does so explicitly using [`JNIEnv::with_local_frame`]. That local reference frame is exited
/// when the native method or `with_local_frame` returns. When a local reference frame is exited,
/// all local references created inside it are deleted.
///
/// Unlike C JNI, this crate creates a separate `JNIEnv` for each local reference frame. The
/// associated Rust lifetime `'local` represents that local reference frame. Rust's borrow checker
/// will ensure that local references are not used after their local reference frame exits (which
/// would cause undefined behavior).
///
/// Unlike global references, local references are not deleted when dropped by default. This is for
/// performance: it is faster for the JVM to delete all of the local references in a frame all at
/// once, than to delete each local reference one at a time. However, this can cause a memory leak
/// if the local reference frame remains entered for a long time, such as a long-lasting loop, in
/// which case local references should be deleted explicitly. Local references can be deleted when
/// dropped if desired; use [`JNIEnv::auto_local`] to arrange that.
///
/// ## Lifetime Names
///
/// This crate uses the following convention for lifetime names:
///
/// * `'local` is the lifetime of a local reference frame, as described above.
///
/// * `'other_local`, `'other_local_1`, and `'other_local_2` are the lifetimes of some other local
///   reference frame, which may be but doesn't have to be the same as `'local`. For example,
///   [`JNIEnv::new_local_ref`] accepts a local reference in any local reference frame
///   `'other_local` and creates a new local reference to the same object in `'local`.
///
/// * `'obj_ref` is the lifetime of a borrow of a JNI reference, like <code>&amp;[JObject]</code>
///   or <code>&amp;[GlobalRef]</code>. For example, [`JNIEnv::get_list`] constructs a new
///   [`JList`] that borrows a `&'obj_ref JObject`.
///
/// ## `null` Java references
/// `null` Java references are handled by the following rules:
///   - If a `null` Java reference is passed to a method that expects a non-`null`
///   argument, an `Err` result with the kind `NullPtr` is returned.
///   - If a JNI function returns `null` to indicate an error (e.g. `new_int_array`),
///     it is converted to `Err`/`NullPtr` or, where possible, to a more applicable
///     error type, such as `MethodNotFound`. If the JNI function also throws
///     an exception, the `JavaException` error kind will be preferred.
///   - If a JNI function may return `null` Java reference as one of possible reference
///     values (e.g., `get_object_array_element` or `get_field_unchecked`),
///     it is converted to `JObject::null()`.
///
/// # `&self` and `&mut self`
///
/// Most of the methods on this type take a `&mut self` reference, specifically all methods that
/// can enter a new local reference frame. This includes anything that might invoke user-defined
/// Java code, which can indirectly enter a new local reference frame by calling a native method.
///
/// The reason for this restriction is to ensure that a `JNIEnv` instance can only be used in the
/// local reference frame that it belongs to. This, in turn, ensures that it is not possible to
/// create [`JObject`]s with the lifetime of a different local reference frame, which would lead to
/// undefined behavior. (See [issue #392] for background discussion.)
///
/// [issue #392]: https://github.com/jni-rs/jni-rs/issues/392
///
/// ## `cannot borrow as mutable`
///
/// If a function takes two or more parameters, one of them is `JNIEnv`, and another is something
/// returned by a `JNIEnv` method (like [`JObject`]), then calls to that function may not compile:
///
/// ```rust,compile_fail
/// # use jni::{errors::Result, JNIEnv, objects::*};
/// #
/// # fn f(env: &mut JNIEnv) -> Result<()> {
/// fn example_function(
///     env: &mut JNIEnv,
///     obj: &JObject,
/// ) {
///     // …
/// }
///
/// example_function(
///     env,
///     // ERROR: cannot borrow `*env` as mutable more than once at a time
///     &env.new_object(
///         "com/example/SomeClass",
///         "()V",
///         &[],
///     )?,
/// )
/// # ; Ok(())
/// # }
/// ```
///
/// To fix this, the `JNIEnv` parameter needs to come *last*:
///
/// ```rust,no_run
/// # use jni::{errors::Result, JNIEnv, objects::*};
/// #
/// # fn f(env: &mut JNIEnv) -> Result<()> {
/// fn example_function(
///     obj: &JObject,
///     env: &mut JNIEnv,
/// ) {
///     // …
/// }
///
/// example_function(
///     &env.new_object(
///         "com/example/SomeClass",
///         "()V",
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
/// perform some checks to ensure the validity of provided signatures, names
/// and arguments, and then call the corresponding unchecked method.
///
/// Checked methods are more flexible as they allow passing class names
/// and method/field descriptors as strings and may perform lookups
/// of class objects and method/field ids for you, also performing
/// all the needed precondition checks. However, these lookup operations
/// are expensive, so if you need to call the same method (or access
/// the same field) multiple times, it is
/// [recommended](https://docs.oracle.com/en/java/javase/11/docs/specs/jni/design.html#accessing-fields-and-methods)
/// to cache the instance of the class and the method/field id, e.g.
///   - in loops
///   - when calling the same Java callback repeatedly.
///
/// If you do not cache references to classes and method/field ids,
/// you will *not* benefit from the unchecked methods.
///
/// Calling unchecked methods with invalid arguments and/or invalid class and
/// method descriptors may lead to segmentation fault.
#[repr(transparent)]
#[derive(Debug)]
pub struct JNIEnv<'local> {
    /// A non-null JNIEnv pointer
    internal: *mut sys::JNIEnv,
    lifetime: PhantomData<&'local ()>,
}

impl<'local> JNIEnv<'local> {
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

    /// Create a JNIEnv from a raw pointer.
    ///
    /// This does a null check, and checks that the JNI version is >= 1.4
    ///
    /// # Safety
    ///
    /// Expects a valid pointer retrieved from the `GetEnv` JNI function or [Self::get_raw] function.
    pub unsafe fn from_raw(ptr: *mut sys::JNIEnv) -> Result<Self> {
        let ptr = null_check!(ptr, "from_raw ptr argument")?;
        let env = JNIEnv {
            internal: ptr,
            lifetime: PhantomData,
        };
        if env.version() < JNIVersion::V1_4 {
            Err(Error::UnsupportedVersion)
        } else {
            Ok(env)
        }
    }

    /// Create a JNIEnv from a raw pointer.
    ///
    /// Doesn't check for `null` or check the JNI version
    ///
    /// # Safety
    ///
    /// Expects a valid, non-null pointer retrieved from the `GetEnv` JNI function or [`Self::get_raw`] function.
    /// Requires a JNI version >= 1.4
    pub unsafe fn from_raw_unchecked(ptr: *mut sys::JNIEnv) -> Self {
        JNIEnv {
            internal: ptr,
            lifetime: PhantomData,
        }
    }

    /// Get the raw JNIEnv pointer
    pub fn get_raw(&self) -> *mut sys::JNIEnv {
        self.internal
    }

    /// Duplicates this `JNIEnv`.
    ///
    /// # Safety
    ///
    /// The duplicate `JNIEnv` must not be used to create any local references, unless they are
    /// discarded before the current [local reference frame] is exited. Otherwise, they may have a
    /// lifetime longer than they are actually valid for, resulting in a use-after-free bug and
    /// undefined behavior.
    ///
    /// See [issue #392] for background.
    ///
    /// [local reference frame]: JNIEnv::with_local_frame
    /// [issue #392]: https://github.com/jni-rs/jni-rs/issues/392
    pub unsafe fn unsafe_clone(&self) -> Self {
        Self {
            internal: self.internal,
            lifetime: self.lifetime,
        }
    }

    /// Get the JNI version that this [`JNIEnv`] supports.
    pub fn version(&self) -> JNIVersion {
        // Safety: GetVersion is 1.1 API that must be valid
        JNIVersion::from(unsafe { jni_call_unchecked!(self, v1_1, GetVersion) })
    }

    /// Load a class from a buffer of raw class data. The name of the class must match the name
    /// encoded within the class file data.
    pub fn define_class<S>(
        &mut self,
        name: S,
        loader: &JObject,
        buf: &[u8],
    ) -> Result<JClass<'local>>
    where
        S: Into<JNIString>,
    {
        let name = name.into();
        self.define_class_impl(name.as_ptr(), loader, buf)
    }

    /// Load a class from a buffer of raw class data. The name of the class is inferred from the
    /// buffer.
    pub fn define_unnamed_class(&mut self, loader: &JObject, buf: &[u8]) -> Result<JClass<'local>> {
        self.define_class_impl(ptr::null(), loader, buf)
    }

    // Note: This requires `&mut` because it might invoke a method on a user-defined `ClassLoader`.
    fn define_class_impl(
        &mut self,
        name: *const c_char,
        loader: &JObject,
        buf: &[u8],
    ) -> Result<JClass<'local>> {
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
                buf.as_ptr() as *const jbyte,
                buf.len() as jsize
            )
            .map(|class| JClass::from_raw(class))
        }
    }

    /// Load a class from a buffer of raw class data. The name of the class must match the name
    /// encoded within the class file data.
    pub fn define_class_bytearray<S>(
        &mut self,
        name: S,
        loader: &JObject,
        buf: &AutoElements<'_, '_, '_, jbyte>,
    ) -> Result<JClass<'local>>
    where
        S: Into<JNIString>,
    {
        let name = name.into();
        // Safety:
        // DefineClass is 1.1 API that must be valid
        // It is valid to potentially pass a `null` `name` to `DefineClass`, since the
        // name can bre read from the bytecode.
        unsafe {
            jni_call_check_ex_and_null_ret!(
                self,
                v1_1,
                DefineClass,
                name.as_ptr(),
                loader.as_raw(),
                buf.as_ptr(),
                buf.len() as _
            )
            .map(|class| JClass::from_raw(class))
        }
    }

    /// Look up a class by name.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, JNIEnv, objects::JClass};
    /// #
    /// # fn example<'local>(env: &mut JNIEnv<'local>) -> Result<()> {
    /// let class: JClass<'local> = env.find_class("java/lang/String")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn find_class<S>(&mut self, name: S) -> Result<JClass<'local>>
    where
        S: Into<JNIString>,
    {
        let name = name.into();
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

    /// Raise an exception from an existing object. This will continue being
    /// thrown in java unless `exception_clear` is called.
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use jni::{errors::Result, JNIEnv};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
    /// env.throw(("java/lang/Exception", "something bad happened"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Defaulting to "java/lang/Exception":
    ///
    /// ```rust,no_run
    /// # use jni::{errors::Result, JNIEnv};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
    /// env.throw("something bad happened")?;
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

    /// Create and throw a new exception from a class descriptor and an error
    /// message.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, JNIEnv};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
    /// env.throw_new("java/lang/Exception", "something bad happened")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn throw_new<'other_local, S, T>(&mut self, class: T, msg: S) -> Result<()>
    where
        S: Into<JNIString>,
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let msg = msg.into();

        // Safety:
        // ThrowNew is 1.1 API that must be valid
        //
        // We are careful to ensure that we don't drop the reference
        // to `class` or `msg` after converting to raw pointers.
        let res: i32 = unsafe {
            jni_call_unchecked!(self, v1_1, ThrowNew, class.as_ref().as_raw(), msg.as_ptr())
        };

        // Ensure that `class` and msg aren't dropped before the JNI call returns.
        drop(class);
        drop(msg);

        if res == 0 {
            Ok(())
        } else {
            Err(Error::ThrowFailed(res))
        }
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
    /// # use jni::{JNIEnv, strings::JNIString};
    /// # let env: JNIEnv = unimplemented!();
    /// env.fatal_error(&JNIString::from("Game over, man! Game over!"))
    /// ```
    ///
    /// This can also be used in a way that's completely guaranteed to be
    /// panic- and allocation-free, but it is somewhat complicated and
    /// `unsafe`:
    ///
    /// ```no_run
    /// # use jni::{JNIEnv, strings::JNIStr};
    /// # use std::ffi::CStr;
    /// const MESSAGE: &JNIStr = unsafe {
    ///     JNIStr::from_cstr_unchecked(
    ///         CStr::from_bytes_with_nul_unchecked(
    ///             b"Game over, man! Game over!\0"
    ///         )
    ///     )
    /// };
    ///
    /// # let env: JNIEnv = unimplemented!();
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
    /// # use jni::{errors::Result, JNIEnv};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
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
    /// of this `JNIEnv`.
    pub unsafe fn new_direct_byte_buffer(
        &mut self,
        data: *mut u8,
        len: usize,
    ) -> Result<JByteBuffer<'local>> {
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
    /// in certain situations. See [`GlobalRef`] for more information.
    pub fn new_global_ref<'other_local, O>(&self, obj: O) -> Result<GlobalRef>
    where
        O: AsRef<JObject<'other_local>>,
    {
        let jvm = self.get_java_vm()?;
        unsafe {
            let new_ref = jni_call_unchecked!(self, v1_1, NewGlobalRef, obj.as_ref().as_raw());
            let global = GlobalRef::from_raw(jvm, new_ref);
            Ok(global)
        }
    }

    /// Creates a new weak global reference.
    ///
    /// Weak global references are a special kind of Java object reference that
    /// doesn't prevent the Java object from being garbage collected. See
    /// [`WeakRef`] for more information.
    ///
    /// If the provided object is null, this method returns `None`. Otherwise, it returns `Some`
    /// containing the new weak global reference.
    pub fn new_weak_ref<'other_local, O>(&self, obj: O) -> Result<Option<WeakRef>>
    where
        O: AsRef<JObject<'other_local>>,
    {
        // We need the `JavaVM` in order to construct a `WeakRef` below. But because `get_java_vm`
        // is fallible, we need to call it before doing anything else, so that we don't leak
        // memory if it fails.
        let vm = self.get_java_vm()?;

        let obj = obj.as_ref().as_raw();

        // Check if the pointer is null *before* calling `NewWeakGlobalRef`.
        //
        // This avoids a bug in some JVM implementations which, contrary to the JNI specification,
        // will throw `java.lang.OutOfMemoryError: C heap space` from `NewWeakGlobalRef` if it is
        // passed a null pointer. (The specification says it will return a null pointer in that
        // situation, not throw an exception.)
        if obj.is_null() {
            return Ok(None);
        }

        unsafe {
            // Safety: jni-rs requires JNI_VERSION > 1.2
            let weak: sys::jweak = jni_call_check_ex!(self, v1_2, NewWeakGlobalRef, obj)?;

            // Check if the pointer returned by `NewWeakGlobalRef` is null. This can happen if `obj` is
            // itself a weak reference that was already garbage collected.
            if weak.is_null() {
                return Ok(None);
            }

            let weak = WeakRef::from_raw(vm, weak);

            Ok(Some(weak))
        }
    }

    /// Create a new local reference to an object.
    ///
    /// Specifically, this calls the JNI function [`NewLocalRef`], which creates a reference in the
    /// current local reference frame, regardless of whether the original reference belongs to the
    /// same local reference frame, a different one, or is a [global reference][GlobalRef]. In Rust
    /// terms, this method accepts a JNI reference with any valid lifetime and produces a clone of
    /// that reference with the lifetime of this `JNIEnv`. The returned reference can outlive the
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
    /// `'local` is the lifetime of the local reference frame that this `JNIEnv` belongs to. This
    /// method creates a new local reference in that frame, with lifetime `'local`.
    ///
    /// `'other_local` is the lifetime of the original reference's frame. It can be any valid
    /// lifetime, even one that `'local` outlives or vice versa.
    ///
    /// Think of `'local` as meaning `'new` and `'other_local` as meaning `'original`. (It is
    /// unfortunately not possible to actually give these names to the two lifetimes because
    /// `'local` is a parameter to the `JNIEnv` type, not a parameter to this method.)
    ///
    /// # Example
    ///
    /// In the following example, the `ExampleError::extract_throwable` method uses
    /// `JNIEnv::new_local_ref` to create a new local reference that outlives the original global
    /// reference:
    ///
    /// ```no_run
    /// # use jni::{JNIEnv, objects::*};
    /// # use std::fmt::Display;
    /// #
    /// # type SomeOtherErrorType = Box<dyn Display>;
    /// #
    /// /// An error that may be caused by either a Java exception or something going wrong in Rust
    /// /// code.
    /// enum ExampleError {
    ///     /// This variant represents a Java exception.
    ///     ///
    ///     /// The enclosed `GlobalRef` points to a Java object of class `java.lang.Throwable`
    ///     /// (or one of its many subclasses).
    ///     Exception(GlobalRef),
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
    ///     fn extract_throwable<'local>(self, env: &mut JNIEnv<'local>) -> jni::errors::Result<JThrowable<'local>> {
    ///         let throwable: JObject = match self {
    ///             ExampleError::Exception(exception) => {
    ///                 // The error was caused by a Java exception.
    ///
    ///                 // Here, `exception` is a `GlobalRef` pointing to a Java `Throwable`. It
    ///                 // will be dropped at the end of this `match` arm. We'll use
    ///                 // `new_local_ref` to create a local reference that will outlive the
    ///                 // `GlobalRef`.
    ///
    ///                 env.new_local_ref(&exception)?
    ///             }
    ///
    ///             ExampleError::Other(error) => {
    ///                 // The error was caused by something that happened in Rust code. Create a
    ///                 // new `java.lang.Error` to represent it.
    ///
    ///                 let error_string = env.new_string(error.to_string())?;
    ///
    ///                 env.new_object(
    ///                     "java/lang/Error",
    ///                     "(Ljava/lang/String;)V",
    ///                     &[
    ///                         (&error_string).into(),
    ///                     ],
    ///                 )?
    ///             }
    ///         };
    ///
    ///         Ok(JThrowable::from(throwable))
    ///     }
    /// }
    /// ```
    ///
    /// [`NewLocalRef`]: https://docs.oracle.com/en/java/javase/11/docs/specs/jni/functions.html#newlocalref
    pub fn new_local_ref<'other_local, O>(&self, obj: O) -> Result<JObject<'local>>
    where
        O: AsRef<JObject<'other_local>>,
    {
        let obj = obj.as_ref();

        // By checking for `null` before calling `NewLocalRef` we can recognise
        // that a `null` returned from `NewLocalRef` is from being out of memory.
        if obj.is_null() {
            return Ok(JObject::null());
        }

        // Safety: we check the JNI version is > 1.2 in `from_raw`
        let local = unsafe {
            JObject::from_raw(jni_call_unchecked!(self, v1_2, NewLocalRef, obj.as_raw()))
        };

        // Since we know we didn't pass a `null` `obj` reference to `NewLocalRef` then
        // a `null` implies an out-of-memory error.
        //
        // (We assume it's not a `null` from failing to upgrade a weak reference because
        //  that would be done via `WeakRef::upgrade_local`)
        //
        if local.is_null() {
            return Err(Error::JniCall(JniError::NoMemory));
        }

        Ok(local)
    }

    /// Creates a new auto-deleted local reference.
    ///
    /// See also [`with_local_frame`](struct.JNIEnv.html#method.with_local_frame) method that
    /// can be more convenient when you create a _bounded_ number of local references
    /// but cannot rely on automatic de-allocation (e.g., in case of recursion, deep call stacks,
    /// [permanently-attached](struct.JavaVM.html#attaching-native-threads) native threads, etc.).
    pub fn auto_local<O>(&self, obj: O) -> AutoLocal<'local, O>
    where
        O: Into<JObject<'local>>,
    {
        AutoLocal::new(obj, self)
    }

    /// Deletes the local reference.
    ///
    /// Local references are valid for the duration of a native method call.
    /// They are freed automatically after the native method returns. Each local
    /// reference costs some amount of Java Virtual Machine resource.
    /// Programmers need to make sure that native methods do not excessively
    /// allocate local references. Although local references are automatically
    /// freed after the native method returns to Java, excessive allocation of
    /// local references may cause the VM to run out of memory during the
    /// execution of a native method.
    ///
    /// In most cases it is better to use `AutoLocal` (see `auto_local` method)
    /// or `with_local_frame` instead of direct `delete_local_ref` calls.
    ///
    /// `obj` can be a mutable borrow of a local reference (such as
    /// `&mut JObject`) instead of the local reference itself (such as
    /// `JObject`). In this case, the local reference will still exist after
    /// this method returns, but it will be null.
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
    /// [`with_local_frame`](struct.JNIEnv.html#method.with_local_frame)
    /// instead of direct `push_local_frame`/`pop_local_frame` calls.
    ///
    /// See also [`auto_local`](struct.JNIEnv.html#method.auto_local) method
    /// and `AutoLocal` type — that approach can be more convenient in loops.
    pub fn push_local_frame(&self, capacity: i32) -> Result<()> {
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
    /// [`JNIEnv::with_local_frame`] instead.
    ///
    /// # Safety
    ///
    /// Any local references created after the most recent call to
    /// [`JNIEnv::push_local_frame`] (or the underlying JNI function) must not
    /// be used after calling this method.
    pub unsafe fn pop_local_frame(&self, result: &JObject) -> Result<JObject<'local>> {
        // Safety:
        // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
        // We check for JNI > 1.2 in `from_raw`
        Ok(JObject::from_raw(jni_call_unchecked!(
            self,
            v1_2,
            PopLocalFrame,
            result.as_raw()
        )))
    }

    /// Executes the given function in a new local reference frame, in which at least a given number
    /// of references can be created. Once this method returns, all references allocated
    /// in the frame are freed.
    ///
    /// If a frame can't be allocated with the requested capacity for local
    /// references, returns `Err` with a pending `OutOfMemoryError`.
    ///
    /// Since local references created within this frame won't be accessible to the calling
    /// frame then if you need to pass an object back to the caller then you can do that via a
    /// [`GlobalRef`] / [`Self::make_global`].
    pub fn with_local_frame<F, T, E>(&mut self, capacity: i32, f: F) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut JNIEnv) -> std::result::Result<T, E>,
        E: From<Error>,
    {
        unsafe {
            self.push_local_frame(capacity)?;
            let ret = catch_unwind(AssertUnwindSafe(|| f(self)));
            self.pop_local_frame(&JObject::null())?;

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
    /// temporary [`GlobalRef`].
    pub fn with_local_frame_returning_local<F, E>(
        &mut self,
        capacity: i32,
        f: F,
    ) -> std::result::Result<JObject<'local>, E>
    where
        F: for<'new_local> FnOnce(
            &mut JNIEnv<'new_local>,
        ) -> std::result::Result<JObject<'new_local>, E>,
        E: From<Error>,
    {
        unsafe {
            self.push_local_frame(capacity)?;
            let ret = catch_unwind(AssertUnwindSafe(|| f(self)));
            match ret {
                Ok(ret) => match ret {
                    Ok(obj) => {
                        let obj = self.pop_local_frame(&obj)?;
                        Ok(obj)
                    }
                    Err(err) => {
                        self.pop_local_frame(&JObject::null())?;
                        Err(err)
                    }
                },
                Err(payload) => {
                    self.pop_local_frame(&JObject::null())?;
                    resume_unwind(payload);
                }
            }
        }
    }

    /// Allocates a new object from a class descriptor without running a
    /// constructor.
    pub fn alloc_object<'other_local, T>(&mut self, class: T) -> Result<JObject<'local>>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;
        let obj = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, AllocObject, class.as_ref().as_raw())?
        };

        // Ensure that `class` isn't dropped before the JNI call returns.
        drop(class);

        Ok(unsafe { JObject::from_raw(obj) })
    }

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
        U: Into<JNIString>,
        V: Into<JNIString>,
        C: for<'other_local_2> Fn(
            &mut Self,
            &JClass<'other_local_2>,
            &JNIString,
            &JNIString,
        ) -> Result<R>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.into();
        let sig = sig.into();

        let res: Result<R> = get_method(self, class.as_ref(), &ffi_name, &sig);

        match res {
            Ok(m) => Ok(m),
            Err(e) => match e {
                Error::NullPtr(_) => {
                    let name: String = ffi_name.into();
                    let sig: String = sig.into();
                    Err(Error::MethodNotFound { name, sig })
                }
                _ => Err(e),
            },
        }
    }

    /// Look up a method by class descriptor, name, and
    /// signature.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, JNIEnv, objects::JMethodID};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
    /// let method_id: JMethodID =
    ///     env.get_method_id("java/lang/String", "substring", "(II)Ljava/lang/String;")?;
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
        U: Into<JNIString>,
        V: Into<JNIString>,
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

    /// Look up a static method by class descriptor, name, and
    /// signature.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, JNIEnv, objects::JStaticMethodID};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
    /// let method_id: JStaticMethodID =
    ///     env.get_static_method_id("java/lang/String", "valueOf", "(I)Ljava/lang/String;")?;
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
        U: Into<JNIString>,
        V: Into<JNIString>,
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

    /// Look up the field ID for a class/name/type combination.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, JNIEnv, objects::JFieldID};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
    /// let field_id: JFieldID = env.get_field_id("com/my/Class", "intField", "I")?;
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
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.into();
        let ffi_sig = sig.into();

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
                    let name: String = ffi_name.into();
                    let sig: String = ffi_sig.into();
                    Err(Error::FieldNotFound { name, sig })
                }
                _ => Err(e),
            },
        }
    }

    /// Look up the static field ID for a class/name/type combination.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use jni::{errors::Result, JNIEnv, objects::JStaticFieldID};
    /// #
    /// # fn example(env: &mut JNIEnv) -> Result<()> {
    /// let field_id: JStaticFieldID = env.get_static_field_id("com/my/Class", "intField", "I")?;
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
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.into();
        let ffi_sig = sig.into();

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
                    let name: String = ffi_name.into();
                    let sig: String = ffi_sig.into();
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
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use ReturnType::{Array, Object, Primitive};

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
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use ReturnType::{Array, Object, Primitive};

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
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use ReturnType::{Array, Object, Primitive};

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
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let obj = obj.as_ref();
        let obj = null_check!(obj, "call_method obj argument")?;

        // parse the signature
        let parsed = TypeSignature::from_str(sig.as_ref())?;
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
                JavaType::Object(_) | JavaType::Array(_) => act.primitive_type().is_none(),
                JavaType::Method(_) => {
                    unreachable!("JavaType::Method(_) should not come from parsing a method sig")
                }
            });
        if !base_types_match {
            return Err(Error::InvalidArgList(parsed));
        }

        let class = self.get_object_class(obj)?;
        let class = self.auto_local(class);

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
        U: Into<JNIString>,
        V: Into<JNIString> + AsRef<str>,
    {
        let parsed = TypeSignature::from_str(&sig)?;
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
                JavaType::Object(_) | JavaType::Array(_) => act.primitive_type().is_none(),
                JavaType::Method(_) => {
                    unreachable!("JavaType::Method(_) should not come from parsing a method sig")
                }
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
        U: Into<JNIString>,
        V: Into<JNIString> + AsRef<str>,
    {
        let obj = obj.as_ref();
        let obj = null_check!(obj, "call_method obj argument")?;

        let parsed = TypeSignature::from_str(&sig)?;
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
                JavaType::Object(_) | JavaType::Array(_) => act.primitive_type().is_none(),
                JavaType::Method(_) => {
                    unreachable!("JavaType::Method(_) should not come from parsing a method sig")
                }
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
        U: Into<JNIString> + AsRef<str>,
    {
        // parse the signature
        let parsed = TypeSignature::from_str(&ctor_sig)?;

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
                    JavaType::Object(_) | JavaType::Array(_) => act.primitive_type().is_none(),
                    JavaType::Method(_) => {
                        unreachable!("JavaType::Method(_) should not come from parsing a ctor sig")
                    }
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
    pub unsafe fn new_object_unchecked<'other_local, T>(
        &mut self,
        class: T,
        ctor_id: JMethodID,
        ctor_args: &[jvalue],
    ) -> Result<JObject<'local>>
    where
        T: Desc<'local, JClass<'other_local>>,
    {
        let class = class.lookup(self)?;

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

    /// Cast a JObject to a `JList`. This won't throw exceptions or return errors
    /// in the event that the object isn't actually a list, but the methods on
    /// the resulting map object will.
    pub fn get_list<'other_local_1, 'obj_ref>(
        &mut self,
        obj: &'obj_ref JObject<'other_local_1>,
    ) -> Result<JList<'local, 'other_local_1, 'obj_ref>>
    where
        'other_local_1: 'obj_ref,
    {
        let obj = null_check!(obj, "get_list obj argument")?;
        JList::from_env(self, obj)
    }

    /// Cast a JObject to a JMap. This won't throw exceptions or return errors
    /// in the event that the object isn't actually a map, but the methods on
    /// the resulting map object will.
    pub fn get_map<'other_local_1, 'obj_ref>(
        &mut self,
        obj: &'obj_ref JObject<'other_local_1>,
    ) -> Result<JMap<'local, 'other_local_1, 'obj_ref>>
    where
        'other_local_1: 'obj_ref,
    {
        let obj = null_check!(obj, "get_map obj argument")?;
        JMap::from_env(self, obj)
    }

    /// Gets the bytes of a Java string, in [modified UTF-8] encoding.
    ///
    /// The returned `JavaStr` can be used to access the modified UTF-8 bytes,
    /// or to convert to a Rust string (which uses standard UTF-8 encoding).
    ///
    /// This only entails calling the JNI function `GetStringUTFChars`.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the Object passed in is an instance of `java.lang.String`,
    /// passing in anything else will lead to undefined behaviour (The JNI implementation
    /// is likely to crash or abort the process).
    ///
    /// If this cannot be guaranteed, use the [`get_string`][Self::get_string]
    /// method instead.
    ///
    /// # Errors
    ///
    /// Returns an error if `obj` is `null`.
    pub unsafe fn get_string_unchecked<'other_local: 'obj_ref, 'obj_ref>(
        &self,
        obj: &'obj_ref JString<'other_local>,
    ) -> Result<JavaStr<'local, 'other_local, 'obj_ref>> {
        let obj = null_check!(obj, "get_string obj argument")?;
        JavaStr::from_env_totally_unchecked(self, obj)
    }

    /// Gets the bytes of a Java string, in [modified UTF-8] encoding.
    ///
    /// The returned `JavaStr` can be used to access the modified UTF-8 bytes,
    /// or to convert to a Rust string (which uses standard UTF-8 encoding).
    ///
    /// This entails checking that the given object is a `java.lang.String`,
    /// then calling the JNI function `GetStringUTFChars`.
    ///
    /// [modified UTF-8]: https://en.wikipedia.org/wiki/UTF-8#Modified_UTF-8
    ///
    /// # Performance
    ///
    /// This function has some relative performance impact compared to
    /// [`get_string_unchecked`][Self::get_string_unchecked].
    /// This performance penalty comes from the extra validation
    /// performed by this function. If and only if you can guarantee that your
    /// `obj` is of the class `java.lang.String`, use `get_string_unchecked` to
    /// skip this extra validation.
    ///
    /// # Errors
    ///
    /// Returns an error if `obj` is `null` or is not an instance of `java.lang.String`.
    pub fn get_string<'other_local: 'obj_ref, 'obj_ref>(
        &mut self,
        obj: &'obj_ref JString<'other_local>,
    ) -> Result<JavaStr<'local, 'other_local, 'obj_ref>> {
        static STRING_CLASS: OnceCell<GlobalRef> = OnceCell::new();
        let string_class = STRING_CLASS.get_or_try_init(|| {
            let string_class_local = self.find_class("java/lang/String")?;
            self.new_global_ref(string_class_local)
        })?;

        if !self.is_instance_of(obj, string_class)? {
            return Err(JniCall(JniError::InvalidArguments));
        }

        // SAFETY: We check that the passed in Object is actually a java.lang.String
        unsafe { self.get_string_unchecked(obj) }
    }

    /// Create a new java string object from a rust string. This requires a
    /// re-encoding of rusts *real* UTF-8 strings to java's modified UTF-8
    /// format.
    pub fn new_string<S: Into<JNIString>>(&self, from: S) -> Result<JString<'local>> {
        let ffi_str = from.into();
        unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewStringUTF, ffi_str.as_ptr())
                .map(|s| JString::from_raw(s))
        }
    }

    /// Get the length of a [`JPrimitiveArray`] or [`JObjectArray`].
    pub fn get_array_length<'other_local, 'array>(
        &self,
        array: &'array impl AsJArrayRaw<'other_local>,
    ) -> Result<jsize> {
        let array = null_check!(array.as_jarray_raw(), "get_array_length array argument")?;
        let len: jsize = unsafe { jni_call_unchecked!(self, v1_1, GetArrayLength, array) };
        Ok(len)
    }

    /// Construct a new array holding objects in class `element_class`.
    /// All elements are initially set to `initial_element`.
    ///
    /// This function returns a local reference, that must not be allocated
    /// excessively.
    /// See [Java documentation][1] for details.
    ///
    /// [1]: https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/design.html#global_and_local_references
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

    /// Returns a local reference to an element of the [`JObjectArray`] `array`.
    pub fn get_object_array_element<'other_local>(
        &mut self,
        array: impl AsRef<JObjectArray<'other_local>>,
        index: jsize,
    ) -> Result<JObject<'local>> {
        let array = null_check!(array.as_ref(), "get_object_array_element array argument")?;
        unsafe {
            jni_call_check_ex!(self, v1_1, GetObjectArrayElement, array.as_raw(), index)
                .map(|obj| JObject::from_raw(obj))
        }
    }

    /// Sets an element of the [`JObjectArray`] `array`.
    pub fn set_object_array_element<'other_local_1, 'other_local_2>(
        &self,
        array: impl AsRef<JObjectArray<'other_local_1>>,
        index: jsize,
        value: impl AsRef<JObject<'other_local_2>>,
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_object_array_element array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetObjectArrayElement,
                array.as_raw(),
                index,
                value.as_ref().as_raw()
            )?;
        }
        Ok(())
    }

    /// Create a new java byte array from a rust byte slice.
    pub fn byte_array_from_slice(&self, buf: &[u8]) -> Result<JByteArray<'local>> {
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
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewBooleanArray, length)
                .map(|array| JBooleanArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java byte array of supplied length.
    pub fn new_byte_array(&self, length: jsize) -> Result<JByteArray<'local>> {
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewByteArray, length)
                .map(|array| JByteArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java char array of supplied length.
    pub fn new_char_array(&self, length: jsize) -> Result<JCharArray<'local>> {
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewCharArray, length)
                .map(|array| JCharArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java short array of supplied length.
    pub fn new_short_array(&self, length: jsize) -> Result<JShortArray<'local>> {
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewShortArray, length)
                .map(|array| JShortArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java int array of supplied length.
    pub fn new_int_array(&self, length: jsize) -> Result<JIntArray<'local>> {
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewIntArray, length)
                .map(|array| JIntArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java long array of supplied length.
    pub fn new_long_array(&self, length: jsize) -> Result<JLongArray<'local>> {
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewLongArray, length)
                .map(|array| JLongArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java float array of supplied length.
    pub fn new_float_array(&self, length: jsize) -> Result<JFloatArray<'local>> {
        let array = unsafe {
            jni_call_check_ex_and_null_ret!(self, v1_1, NewFloatArray, length)
                .map(|array| JFloatArray::from_raw(array))?
        };
        Ok(array)
    }

    /// Create a new java double array of supplied length.
    pub fn new_double_array(&self, length: jsize) -> Result<JDoubleArray<'local>> {
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
    /// [`array.length`]: struct.JNIEnv.html#method.get_array_length
    pub fn get_boolean_array_region<'other_local>(
        &self,
        array: impl AsRef<JBooleanArray<'other_local>>,
        start: jsize,
        buf: &mut [jboolean],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "get_boolean_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                GetBooleanArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_mut_ptr()
            )?;
        }
        Ok(())
    }

    /// Copy elements of the java byte array from the `start` index to the `buf`
    /// slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.JNIEnv.html#method.get_array_length
    pub fn get_byte_array_region<'other_local>(
        &self,
        array: impl AsRef<JByteArray<'other_local>>,
        start: jsize,
        buf: &mut [jbyte],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "get_byte_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                GetByteArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_mut_ptr()
            )
        }
    }

    /// Copy elements of the java char array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.JNIEnv.html#method.get_array_length
    pub fn get_char_array_region<'other_local>(
        &self,
        array: impl AsRef<JCharArray<'other_local>>,
        start: jsize,
        buf: &mut [jchar],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "get_char_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                GetCharArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_mut_ptr()
            )
        }
    }

    /// Copy elements of the java short array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.JNIEnv.html#method.get_array_length
    pub fn get_short_array_region<'other_local>(
        &self,
        array: impl AsRef<JShortArray<'other_local>>,
        start: jsize,
        buf: &mut [jshort],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "get_short_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                GetShortArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_mut_ptr()
            )
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
    /// [`array.length`]: struct.JNIEnv.html#method.get_array_length
    pub fn get_int_array_region<'other_local>(
        &self,
        array: impl AsRef<JIntArray<'other_local>>,
        start: jsize,
        buf: &mut [jint],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "get_int_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                GetIntArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_mut_ptr()
            )
        }
    }

    /// Copy elements of the java long array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.JNIEnv.html#method.get_array_length
    pub fn get_long_array_region<'other_local>(
        &self,
        array: impl AsRef<JLongArray<'other_local>>,
        start: jsize,
        buf: &mut [jlong],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "get_long_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                GetLongArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_mut_ptr()
            )
        }
    }

    /// Copy elements of the java float array from the `start` index to the
    /// `buf` slice. The number of copied elements is equal to the `buf` length.
    ///
    /// # Errors
    /// If `start` is negative _or_ `start + buf.len()` is greater than [`array.length`]
    /// then no elements are copied, an `ArrayIndexOutOfBoundsException` is thrown,
    /// and `Err` is returned.
    ///
    /// [`array.length`]: struct.JNIEnv.html#method.get_array_length
    pub fn get_float_array_region<'other_local>(
        &self,
        array: impl AsRef<JFloatArray<'other_local>>,
        start: jsize,
        buf: &mut [jfloat],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "get_float_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                GetFloatArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_mut_ptr()
            )
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
    /// [`array.length`]: struct.JNIEnv.html#method.get_array_length
    pub fn get_double_array_region<'other_local>(
        &self,
        array: impl AsRef<JDoubleArray<'other_local>>,
        start: jsize,
        buf: &mut [jdouble],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "get_double_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                GetDoubleArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_mut_ptr()
            )
        }
    }

    /// Copy the contents of the `buf` slice to the java boolean array at the
    /// `start` index.
    pub fn set_boolean_array_region<'other_local>(
        &self,
        array: impl AsRef<JBooleanArray<'other_local>>,
        start: jsize,
        buf: &[jboolean],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_boolean_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetBooleanArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_ptr()
            )
        }
    }

    /// Copy the contents of the `buf` slice to the java byte array at the
    /// `start` index.
    pub fn set_byte_array_region<'other_local>(
        &self,
        array: impl AsRef<JByteArray<'other_local>>,
        start: jsize,
        buf: &[jbyte],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_byte_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetByteArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_ptr()
            )
        }
    }

    /// Copy the contents of the `buf` slice to the java char array at the
    /// `start` index.
    pub fn set_char_array_region<'other_local>(
        &self,
        array: impl AsRef<JCharArray<'other_local>>,
        start: jsize,
        buf: &[jchar],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_char_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetCharArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_ptr()
            )
        }
    }

    /// Copy the contents of the `buf` slice to the java short array at the
    /// `start` index.
    pub fn set_short_array_region<'other_local>(
        &self,
        array: impl AsRef<JShortArray<'other_local>>,
        start: jsize,
        buf: &[jshort],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_short_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetShortArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_ptr()
            )
        }
    }

    /// Copy the contents of the `buf` slice to the java int array at the
    /// `start` index.
    pub fn set_int_array_region<'other_local>(
        &self,
        array: impl AsRef<JIntArray<'other_local>>,
        start: jsize,
        buf: &[jint],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_int_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetIntArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_ptr()
            )
        }
    }

    /// Copy the contents of the `buf` slice to the java long array at the
    /// `start` index.
    pub fn set_long_array_region<'other_local>(
        &self,
        array: impl AsRef<JLongArray<'other_local>>,
        start: jsize,
        buf: &[jlong],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_long_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetLongArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_ptr()
            )
        }
    }

    /// Copy the contents of the `buf` slice to the java float array at the
    /// `start` index.
    pub fn set_float_array_region<'other_local>(
        &self,
        array: impl AsRef<JFloatArray<'other_local>>,
        start: jsize,
        buf: &[jfloat],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_float_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetFloatArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_ptr()
            )
        }
    }

    /// Copy the contents of the `buf` slice to the java double array at the
    /// `start` index.
    pub fn set_double_array_region<'other_local>(
        &self,
        array: impl AsRef<JDoubleArray<'other_local>>,
        start: jsize,
        buf: &[jdouble],
    ) -> Result<()> {
        let array = null_check!(array.as_ref(), "set_double_array_region array argument")?;
        unsafe {
            jni_call_check_ex!(
                self,
                v1_1,
                SetDoubleArrayRegion,
                array.as_raw(),
                start,
                buf.len() as jsize,
                buf.as_ptr()
            )
        }
    }

    /// Get a field without checking the provided type against the actual field.
    ///
    /// # Safety
    /// There will be undefined behaviour if the return type `ty` doesn't match
    /// the type for the given `field`
    pub unsafe fn get_field_unchecked<'other_local, O, T>(
        &mut self,
        obj: O,
        field: T,
        ty: ReturnType,
    ) -> Result<JValueOwned<'local>>
    where
        O: AsRef<JObject<'other_local>>,
        T: Desc<'local, JFieldID>,
    {
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use ReturnType::{Array, Object, Primitive};

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
    /// There will be undefined behaviour if the `val` type doesn't match
    /// the type for the given `field`
    pub unsafe fn set_field_unchecked<'other_local, O, T>(
        &mut self,
        obj: O,
        field: T,
        val: JValue,
    ) -> Result<()>
    where
        O: AsRef<JObject<'other_local>>,
        T: Desc<'local, JFieldID>,
    {
        if let JValue::Void = val {
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

        match val {
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
    pub fn get_field<'other_local, O, S, T>(
        &mut self,
        obj: O,
        name: S,
        ty: T,
    ) -> Result<JValueOwned<'local>>
    where
        O: AsRef<JObject<'other_local>>,
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let obj = obj.as_ref();
        let class = self.get_object_class(obj)?;
        let class = self.auto_local(class);

        let parsed = ReturnType::from_str(ty.as_ref())?;

        let field_id: JFieldID = Desc::<JFieldID>::lookup((&class, name, ty), self)?;

        // Safety: Since we have explicitly looked up the field ID based on the given
        // return type we have already validate that they match
        unsafe { self.get_field_unchecked(obj, field_id, parsed) }
    }

    /// Set a field. Does the same lookups as `get_field` and ensures that the
    /// type matches the given value.
    pub fn set_field<'other_local, O, S, T>(
        &mut self,
        obj: O,
        name: S,
        ty: T,
        val: JValue,
    ) -> Result<()>
    where
        O: AsRef<JObject<'other_local>>,
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let obj = obj.as_ref();
        let field_ty = JavaType::from_str(ty.as_ref())?;
        let val_primitive = val.primitive_type();

        let wrong_type = Err(Error::WrongJValueType(val.type_name(), "see java field"));

        match field_ty {
            JavaType::Object(_) | JavaType::Array(_) if val_primitive.is_some() => wrong_type,
            JavaType::Primitive(p) if val_primitive != Some(p) => wrong_type,
            JavaType::Primitive(_) if val_primitive.is_none() => wrong_type,
            JavaType::Method(_) => Err(Error::WrongJValueType(
                val.type_name(),
                "cannot set field with method type",
            )),
            _ => {
                let class = self.get_object_class(obj)?;
                let class = self.auto_local(class);

                // Safety: We have explicitly checked that the field type matches
                // the value type
                unsafe { self.set_field_unchecked(obj, (&class, name, ty), val) }
            }
        }
    }

    /// Get a static field without checking the provided type against the actual
    /// field.
    pub fn get_static_field_unchecked<'other_local, T, U>(
        &mut self,
        class: T,
        field: U,
        ty: JavaType,
    ) -> Result<JValueOwned<'local>>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: Desc<'local, JStaticFieldID>,
    {
        use super::signature::Primitive::{
            Boolean, Byte, Char, Double, Float, Int, Long, Short, Void,
        };
        use JavaType::{Array, Method, Object, Primitive};

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
            Method(_) => Err(Error::WrongJValueType("Method", "see java field")),
            Object(_) | Array(_) => {
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

    /// Get a static field. Requires a class lookup and a field id lookup
    /// internally.
    pub fn get_static_field<'other_local, T, U, V>(
        &mut self,
        class: T,
        field: U,
        sig: V,
    ) -> Result<JValueOwned<'local>>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: Into<JNIString>,
        V: Into<JNIString> + AsRef<str>,
    {
        let ty = JavaType::from_str(sig.as_ref())?;

        // go ahead and look up the class sincewe'll need that for the next
        // call.
        let class = class.lookup(self)?;

        self.get_static_field_unchecked(class.as_ref(), (class.as_ref(), field, sig), ty)
    }

    /// Set a static field. Requires a class lookup and a field id lookup internally.
    pub fn set_static_field<'other_local, T, U>(
        &mut self,
        class: T,
        field: U,
        value: JValue,
    ) -> Result<()>
    where
        T: Desc<'local, JClass<'other_local>>,
        U: Desc<'local, JStaticFieldID>,
    {
        if let JValue::Void = value {
            return Err(Error::WrongJValueType("void", "see java field"));
        }

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

    /// Looks up the field ID for the given field name and takes the monitor
    /// lock on the given object so the field can be updated without racing
    /// with other Java threads
    fn lock_rust_field<'other_local, O, S>(
        &self,
        obj: O,
        field: S,
    ) -> Result<(MonitorGuard, JFieldID)>
    where
        O: AsRef<JObject<'other_local>>,
        S: AsRef<str>,
    {
        // Safety: although we get a local reference from get_object_class, we wrap
        // that in an AutoLocal to make sure that it is deleted before returning
        // to the caller.
        //
        // `Desc::<JFieldID>::lookup` is not allowed to leak references and in
        // this case since we explicitly lookup the object class then the
        // `lookup` just needs to call `GetFieldID` without creating any
        // other local reference for the class.
        let mut env = unsafe { self.unsafe_clone() };
        let obj = obj.as_ref();
        let class = env.get_object_class(obj)?;
        let class = self.auto_local(class);
        let field_id: JFieldID = Desc::<JFieldID>::lookup((&class, &field, "J"), &mut env)?;
        let guard = self.lock_obj(obj)?;
        Ok((guard, field_id))
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
        S: AsRef<str>,
        T: Send + 'static,
    {
        let (_guard, field_id) = self.lock_rust_field(&obj, &field)?;

        // Safety: Since we know we are dealing with a `jlong` field and since
        // we have already looked up the field ID then we also know that
        // get_field_unchecked and set_field_unchecked don't need to create any
        // local references.
        let mut env = unsafe { self.unsafe_clone() };

        // Safety: the requirement that the given field must be a `long` is
        // documented in the 'Safety' section of this function
        unsafe {
            let field_ptr = env
                .get_field_unchecked(&obj, field_id, ReturnType::Primitive(Primitive::Long))?
                .j()? as *mut Mutex<T>;
            if !field_ptr.is_null() {
                return Err(Error::FieldAlreadySet(field.as_ref().to_owned()));
            }
        }

        let mbox = Box::new(::std::sync::Mutex::new(rust_object));
        let ptr: *mut Mutex<T> = Box::into_raw(mbox);

        // Safety: the requirement that the given field must be a `long` is
        // documented in the 'Safety' section of this function
        unsafe { env.set_field_unchecked(obj, field_id, (ptr as crate::sys::jlong).into()) }
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
        &self,
        obj: O,
        field: S,
    ) -> Result<MutexGuard<T>>
    where
        O: AsRef<JObject<'other_local>>,
        S: AsRef<str>,
        T: Send + 'static,
    {
        let (_guard, field_id) = self.lock_rust_field(&obj, &field)?;

        // Safety: Since we know we are dealing with a `jlong` field and since
        // we have already looked up the field ID then we also know that
        // get_field_unchecked doesn't need to create any local references.
        let mut env = self.unsafe_clone();

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
        S: AsRef<str>,
        T: Send + 'static,
    {
        let (_guard, field_id) = self.lock_rust_field(&obj, &field)?;

        // Safety: Since we know we are dealing with a `jlong` field and since
        // we have already looked up the field ID then we also know that
        // get_field_unchecked and set_field_unchecked don't need to create any
        // local references.
        let mut env = self.unsafe_clone();

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
    }

    /// Lock a Java object. The MonitorGuard that this returns is responsible
    /// for ensuring that it gets unlocked.
    pub fn lock_obj<'other_local, O>(&self, obj: O) -> Result<MonitorGuard<'local>>
    where
        O: AsRef<JObject<'other_local>>,
    {
        let inner = obj.as_ref().as_raw();
        let res = unsafe { jni_call_unchecked!(self, v1_1, MonitorEnter, inner) };
        jni_error_code_to_result(res)?;

        Ok(MonitorGuard {
            obj: inner,
            env: self.internal,
            life: Default::default(),
        })
    }

    /// Returns the Java VM interface.
    pub fn get_java_vm(&self) -> Result<JavaVM> {
        let mut raw = ptr::null_mut();
        let res = unsafe { jni_call_unchecked!(self, v1_1, GetJavaVM, &mut raw) };
        jni_error_code_to_result(res)?;
        unsafe { JavaVM::from_raw(raw) }
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

    /// Bind function pointers to native methods of class
    /// according to method name and signature.
    /// For details see [documentation](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#RegisterNatives).
    pub fn register_native_methods<'other_local, T>(
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
    /// The elements are accessible until the returned auto-release guard is dropped.
    ///
    /// The returned array may be a copy of the Java array and changes made to
    /// the returned array will not necessarily be reflected in the original
    /// array until the [`AutoElements`] guard is dropped.
    ///
    /// If you know in advance that you will only be reading from the array then
    /// pass [`ReleaseMode::NoCopyBack`] so that the JNI implementation knows
    /// that it's not necessary to copy any data back to the original Java array
    /// when the [`AutoElements`] guard is dropped.
    ///
    /// Since the returned array may be a copy of the Java array, changes made to the
    /// returned array will not necessarily be reflected in the original array until
    /// the corresponding `Release*ArrayElements` JNI method is called.
    /// [`AutoElements`] has a commit() method, to force a copy back of pending
    /// array changes if needed (and without releasing it).
    ///
    /// # Safety
    ///
    /// ## No data races
    ///
    /// This API has no built-in synchronization that ensures there won't be any data
    /// races while accessing the array elements.
    ///
    /// To avoid undefined behaviour it is the caller's responsibility to ensure there
    /// will be no data races between other Rust or Java threads trying to access the
    /// same array.
    ///
    /// Acquiring a [`MonitorGuard`] lock for the `array` could be one way of ensuring
    /// mutual exclusion between Rust and Java threads, so long as the Java threads
    /// also acquire the same lock via `synchronized(array) {}`.
    ///
    /// ## No aliasing
    ///
    /// Callers must not create more than one [`AutoElements`] or
    /// [`AutoElementsCritical`] per Java array at the same time - even if
    /// there is no risk of a data race.
    ///
    /// The reason for this restriction is that [`AutoElements`] and
    /// [`AutoElementsCritical`] implement `DerefMut` which can provide a
    /// mutable `&mut [T]` slice reference for the elements and it would
    /// constitute undefined behaviour to allow there to be more than one
    /// mutable reference that points to the same memory.
    ///
    /// # jboolean elements
    ///
    /// Keep in mind that arrays of `jboolean` values should only ever hold
    /// values of `0` or `1` because any other value could lead to undefined
    /// behaviour within the JVM.
    ///
    /// Also see
    /// [`get_array_elements_critical`](Self::get_array_elements_critical) which
    /// imposes additional restrictions that make it less likely to incur the
    /// cost of copying the array elements.
    pub unsafe fn get_array_elements<'other_local, 'array, T: TypeArray>(
        &mut self,
        array: &'array JPrimitiveArray<'other_local, T>,
        mode: ReleaseMode,
    ) -> Result<AutoElements<'local, 'other_local, 'array, T>> {
        let array = null_check!(array, "get_array_elements array argument")?;
        AutoElements::new(self, array, mode)
    }

    /// Returns an [`AutoElementsCritical`] to access the elements of the given Java `array`.
    ///
    /// The elements are accessible during the critical section that exists until the
    /// returned auto-release guard is dropped.
    ///
    /// This API imposes some strict restrictions that help the JNI implementation
    /// avoid any need to copy the underlying array elements before making them
    /// accessible to native code:
    ///
    /// 1. No other use of JNI calls are allowed (on the same thread) within the critical
    /// section that exists while holding the [`AutoElementsCritical`] guard.
    /// 2. No system calls can be made (Such as `read`) that may depend on a result
    /// from another Java thread.
    ///
    /// The JNI spec does not specify what will happen if these rules aren't adhered to
    /// but it should be assumed it will lead to undefined behaviour, likely deadlock
    /// and possible program termination.
    ///
    /// Even with these restrictions the returned array may still be a copy of
    /// the Java array and changes made to the returned array will not
    /// necessarily be reflected in the original array until the [`AutoElementsCritical`]
    /// guard is dropped.
    ///
    /// If you know in advance that you will only be reading from the array then
    /// pass [`ReleaseMode::NoCopyBack`] so that the JNI implementation knows
    /// that it's not necessary to copy any data back to the original Java array
    /// when the [`AutoElementsCritical`] guard is dropped.
    ///
    /// A nested scope or explicit use of `std::mem::drop` can be used to
    /// control when the returned [`AutoElementsCritical`] is dropped to
    /// minimize the length of the critical section.
    ///
    /// If the given array is `null`, an `Error::NullPtr` is returned.
    ///
    /// # Safety
    ///
    /// ## Critical Section Restrictions
    ///
    /// Although this API takes a mutable reference to a [`JNIEnv`] which should
    /// ensure that it's not possible to call JNI, this API is still marked as
    /// `unsafe` due to the complex, far-reaching nature of the critical-section
    /// restrictions imposed here that can't be guaranteed simply through Rust's
    /// borrow checker rules.
    ///
    /// The rules above about JNI usage and system calls _must_ be adhered to.
    ///
    /// Using this API implies:
    ///
    /// 1. All garbage collection will likely be paused during the critical section
    /// 2. Any use of JNI in other threads may block if they need to allocate memory
    ///    (due to the garbage collector being paused)
    /// 3. Any use of system calls that will wait for a result from another Java thread
    ///    could deadlock if that other thread is blocked by a paused garbage collector.
    ///
    /// A failure to adhere to the critical section rules could lead to any
    /// undefined behaviour, including aborting the program.
    ///
    /// ## No data races
    ///
    /// This API has no built-in synchronization that ensures there won't be any data
    /// races while accessing the array elements.
    ///
    /// To avoid undefined behaviour it is the caller's responsibility to ensure there
    /// will be no data races between other Rust or Java threads trying to access the
    /// same array.
    ///
    /// Acquiring a [`MonitorGuard`] lock for the `array` could be one way of ensuring
    /// mutual exclusion between Rust and Java threads, so long as the Java threads
    /// also acquire the same lock via `synchronized(array) {}`.
    ///
    /// ## No aliasing
    ///
    /// Callers must not create more than one [`AutoElements`] or
    /// [`AutoElementsCritical`] per Java array at the same time - even if
    /// there is no risk of a data race.
    ///
    /// The reason for this restriction is that [`AutoElements`] and
    /// [`AutoElementsCritical`] implement `DerefMut` which can provide a
    /// mutable `&mut [T]` slice reference for the elements and it would
    /// constitute undefined behaviour to allow there to be more than one
    /// mutable reference that points to the same memory.
    ///
    /// ## jboolean elements
    ///
    /// Keep in mind that arrays of `jboolean` values should only ever hold
    /// values of `0` or `1` because any other value could lead to undefined
    /// behaviour within the JVM.
    ///
    /// Also see [`get_array_elements`](Self::get_array_elements) which has fewer
    /// restrictions, but is is more likely to incur a cost from copying the
    /// array elements.
    pub unsafe fn get_array_elements_critical<'other_local, 'array, 'env, T: TypeArray>(
        &'env mut self,
        array: &'array JPrimitiveArray<'other_local, T>,
        mode: ReleaseMode,
    ) -> Result<AutoElementsCritical<'local, 'other_local, 'array, 'env, T>> {
        let array = null_check!(array, "get_primitive_array_critical array argument")?;
        AutoElementsCritical::new(self, array, mode)
    }
}

/// Native method descriptor.
pub struct NativeMethod {
    /// Name of method.
    pub name: JNIString,
    /// Method signature.
    pub sig: JNIString,
    /// Pointer to native function with signature
    /// `fn(env: JNIEnv, class: JClass, ...arguments according to sig) -> RetType`
    /// for static methods or
    /// `fn(env: JNIEnv, object: JObject, ...arguments according to sig) -> RetType`
    /// for instance methods.
    pub fn_ptr: *mut c_void,
}

/// Guard for a lock on a java object. This gets returned from the `lock_obj`
/// method.
pub struct MonitorGuard<'local> {
    obj: sys::jobject,
    env: *mut sys::JNIEnv,
    life: PhantomData<&'local ()>,
}

static_assertions::assert_not_impl_any!(MonitorGuard: Send);

impl<'local> Drop for MonitorGuard<'local> {
    fn drop(&mut self) {
        // Safety:
        //
        // Calling JNIEnv::from_raw_unchecked is safe since we know self.env is
        // non-null and valid, and implements JNI > 1.2
        //
        // This relies on `MonitorGuard` not being `Send` to maintain the
        // invariant that "The current thread must be the owner of the monitor
        // associated with the underlying Java object referred to by obj"
        //
        // This also means we can assume the `IllegalMonitorStateException`
        // exception can't be thrown due to the current thread not owning
        // the monitor.
        let res = unsafe {
            jni_call_unchecked!(
                &JNIEnv::from_raw_unchecked(self.env),
                v1_1,
                MonitorExit,
                self.obj
            )
        };
        if let Err(err) = jni_error_code_to_result(res) {
            log::error!("error releasing java monitor: {err}");
        }
    }
}
