use std::{
    marker::PhantomData,
    os::raw::{c_char, c_void},
    ptr, str,
    str::FromStr,
    sync::{Mutex, MutexGuard},
};

use log::warn;

use crate::signature::ReturnType;
use crate::{
    descriptors::Desc,
    errors::*,
    objects::{
        AutoArray, AutoLocal, AutoPrimitiveArray, GlobalRef, JByteBuffer, JClass, JFieldID, JList,
        JMap, JMethodID, JObject, JStaticFieldID, JStaticMethodID, JString, JThrowable, JValue,
        ReleaseMode, TypeArray,
    },
    signature::{JavaType, Primitive, TypeSignature},
    strings::{JNIString, JavaStr},
    sys::{
        self, jarray, jboolean, jbooleanArray, jbyte, jbyteArray, jchar, jcharArray, jdouble,
        jdoubleArray, jfloat, jfloatArray, jint, jintArray, jlong, jlongArray, jobjectArray,
        jshort, jshortArray, jsize, jvalue, JNINativeMethod,
    },
    JNIVersion, JavaVM,
};

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
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct JNIEnv<'a> {
    internal: *mut sys::JNIEnv,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> JNIEnv<'a> {
    /// Create a JNIEnv from a raw pointer.
    ///
    /// # Safety
    ///
    /// Expects a valid pointer retrieved from the `GetEnv` JNI function. Only does a null check.
    pub unsafe fn from_raw(ptr: *mut sys::JNIEnv) -> Result<Self> {
        non_null!(ptr, "from_raw ptr argument");
        Ok(JNIEnv {
            internal: ptr,
            lifetime: PhantomData,
        })
    }

    /// Get the java version that we're being executed from.
    pub fn get_version(&self) -> Result<JNIVersion> {
        Ok(jni_unchecked!(self.internal, GetVersion).into())
    }

    /// Load a class from a buffer of raw class data. The name of the class must match the name
    /// encoded within the class file data.
    pub fn define_class<S>(&self, name: S, loader: JObject<'a>, buf: &[u8]) -> Result<JClass<'a>>
    where
        S: Into<JNIString>,
    {
        let name = name.into();
        self.define_class_impl(name.as_ptr(), loader, buf)
    }

    /// Load a class from a buffer of raw class data. The name of the class is inferred from the
    /// buffer.
    pub fn define_unnamed_class<S>(&self, loader: JObject<'a>, buf: &[u8]) -> Result<JClass<'a>>
    where
        S: Into<JNIString>,
    {
        self.define_class_impl(ptr::null(), loader, buf)
    }

    fn define_class_impl(
        &self,
        name: *const c_char,
        loader: JObject<'a>,
        buf: &[u8],
    ) -> Result<JClass<'a>> {
        let class = jni_non_null_call!(
            self.internal,
            DefineClass,
            name,
            loader.into_raw(),
            buf.as_ptr() as *const jbyte,
            buf.len() as jsize
        );
        Ok(unsafe { JClass::from_raw(class) })
    }

    /// Look up a class by name.
    ///
    /// # Example
    /// ```rust,ignore
    /// let class: JClass<'a> = env.find_class("java/lang/String");
    /// ```
    pub fn find_class<S>(&self, name: S) -> Result<JClass<'a>>
    where
        S: Into<JNIString>,
    {
        let name = name.into();
        let class = jni_non_null_call!(self.internal, FindClass, name.as_ptr());
        Ok(unsafe { JClass::from_raw(class) })
    }

    /// Returns the superclass for a particular class OR `JObject::null()` for `java.lang.Object` or
    /// an interface. As with `find_class`, takes a descriptor.
    pub fn get_superclass<'c, T>(&self, class: T) -> Result<JClass<'a>>
    where
        T: Desc<'a, JClass<'c>>,
    {
        let class = class.lookup(self)?;
        Ok(unsafe {
            JClass::from_raw(jni_non_void_call!(
                self.internal,
                GetSuperclass,
                class.into_raw()
            ))
        })
    }

    /// Tests whether class1 is assignable from class2.
    pub fn is_assignable_from<'t, 'u, T, U>(&self, class1: T, class2: U) -> Result<bool>
    where
        T: Desc<'a, JClass<'t>>,
        U: Desc<'a, JClass<'u>>,
    {
        let class1 = class1.lookup(self)?;
        let class2 = class2.lookup(self)?;
        Ok(jni_unchecked!(
            self.internal,
            IsAssignableFrom,
            class1.into_raw(),
            class2.into_raw()
        ) == sys::JNI_TRUE)
    }

    /// Returns true if the object reference can be cast to the given type.
    ///
    /// _NB: Unlike the operator `instanceof`, function `IsInstanceOf` *returns `true`*
    /// for all classes *if `object` is `null`.*_
    ///
    /// See [JNI documentation](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#IsInstanceOf)
    /// for details.
    pub fn is_instance_of<'c, O, T>(&self, object: O, class: T) -> Result<bool>
    where
        O: Into<JObject<'a>>,
        T: Desc<'a, JClass<'c>>,
    {
        let class = class.lookup(self)?;
        Ok(jni_unchecked!(
            self.internal,
            IsInstanceOf,
            object.into().into_raw(),
            class.into_raw()
        ) == sys::JNI_TRUE)
    }

    /// Returns true if ref1 and ref2 refer to the same Java object, or are both `NULL`. Otherwise,
    /// returns false.
    pub fn is_same_object<'b, 'c, O, T>(&self, ref1: O, ref2: T) -> Result<bool>
    where
        O: Into<JObject<'b>>,
        T: Into<JObject<'c>>,
    {
        Ok(jni_unchecked!(
            self.internal,
            IsSameObject,
            ref1.into().into_raw(),
            ref2.into().into_raw()
        ) == sys::JNI_TRUE)
    }

    /// Raise an exception from an existing object. This will continue being
    /// thrown in java unless `exception_clear` is called.
    ///
    /// # Examples
    /// ```rust,ignore
    /// let _ = env.throw(("java/lang/Exception", "something bad happened"));
    /// ```
    ///
    /// Defaulting to "java/lang/Exception":
    ///
    /// ```rust,ignore
    /// let _ = env.throw("something bad happened");
    /// ```
    pub fn throw<'e, E>(&self, obj: E) -> Result<()>
    where
        E: Desc<'a, JThrowable<'e>>,
    {
        let throwable = obj.lookup(self)?;
        let res: i32 = jni_unchecked!(self.internal, Throw, throwable.into_raw());
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
    /// ```rust,ignore
    /// let _ = env.throw_new("java/lang/Exception", "something bad happened");
    /// ```
    pub fn throw_new<'c, S, T>(&self, class: T, msg: S) -> Result<()>
    where
        S: Into<JNIString>,
        T: Desc<'a, JClass<'c>>,
    {
        let class = class.lookup(self)?;
        let msg = msg.into();
        let res: i32 = jni_unchecked!(self.internal, ThrowNew, class.into_raw(), msg.as_ptr());
        if res == 0 {
            Ok(())
        } else {
            Err(Error::ThrowFailed(res))
        }
    }

    /// Check whether or not an exception is currently in the process of being
    /// thrown. An exception is in this state from the time it gets thrown and
    /// not caught in a java function until `exception_clear` is called.
    pub fn exception_occurred(&self) -> Result<JThrowable<'a>> {
        let throwable = jni_unchecked!(self.internal, ExceptionOccurred);
        Ok(unsafe { JThrowable::from_raw(throwable) })
    }

    /// Print exception information to the console.
    pub fn exception_describe(&self) -> Result<()> {
        jni_unchecked!(self.internal, ExceptionDescribe);
        Ok(())
    }

    /// Clear an exception in the process of being thrown. If this is never
    /// called, the exception will continue being thrown when control is
    /// returned to java.
    pub fn exception_clear(&self) -> Result<()> {
        jni_unchecked!(self.internal, ExceptionClear);
        Ok(())
    }

    /// Abort the JVM with an error message.
    #[allow(unused_variables, unreachable_code)]
    pub fn fatal_error<S: Into<JNIString>>(&self, msg: S) -> ! {
        let msg = msg.into();
        let res: Result<()> = catch!({
            jni_unchecked!(self.internal, FatalError, msg.as_ptr());
            unreachable!()
        });

        panic!("{:?}", res.unwrap_err());
    }

    /// Check to see if an exception is being thrown. This only differs from
    /// `exception_occurred` in that it doesn't return the actual thrown
    /// exception.
    pub fn exception_check(&self) -> Result<bool> {
        let check = jni_unchecked!(self.internal, ExceptionCheck) == sys::JNI_TRUE;
        Ok(check)
    }

    /// Create a new instance of a direct java.nio.ByteBuffer
    ///
    /// # Example
    /// ```rust,ignore
    /// let buf = vec![0; 1024 * 1024];
    /// let (addr, len) = { // (use buf.into_raw_parts() on nightly)
    ///     let buf = buf.leak();
    ///     (buf.as_mut_ptr(), buf.len())
    /// };
    /// let direct_buffer = unsafe { env.new_direct_byte_buffer(addr, len) };
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
        &self,
        data: *mut u8,
        len: usize,
    ) -> Result<JByteBuffer<'a>> {
        non_null!(data, "new_direct_byte_buffer data argument");
        let obj = jni_non_null_call!(
            self.internal,
            NewDirectByteBuffer,
            data as *mut c_void,
            len as jlong
        );
        Ok(JByteBuffer::from_raw(obj))
    }

    /// Returns the starting address of the memory of the direct
    /// java.nio.ByteBuffer.
    pub fn get_direct_buffer_address(&self, buf: JByteBuffer) -> Result<*mut u8> {
        non_null!(buf, "get_direct_buffer_address argument");
        let ptr = jni_unchecked!(self.internal, GetDirectBufferAddress, buf.into_raw());
        non_null!(ptr, "get_direct_buffer_address return value");
        Ok(ptr as _)
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
    pub fn get_direct_buffer_capacity(&self, buf: JByteBuffer) -> Result<usize> {
        non_null!(buf, "get_direct_buffer_capacity argument");
        let capacity = jni_unchecked!(self.internal, GetDirectBufferCapacity, buf.into_raw());
        match capacity {
            -1 => Err(Error::JniCall(JniError::Unknown)),
            _ => Ok(capacity as usize),
        }
    }

    /// Turns an object into a global ref. This has the benefit of removing the
    /// lifetime bounds since it's guaranteed to not get GC'd by java. It
    /// releases the GC pin upon being dropped.
    pub fn new_global_ref<O>(&self, obj: O) -> Result<GlobalRef>
    where
        O: Into<JObject<'a>>,
    {
        let new_ref = jni_unchecked!(self.internal, NewGlobalRef, obj.into().into_raw());
        let global = unsafe { GlobalRef::from_raw(self.get_java_vm()?, new_ref) };
        Ok(global)
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
    /// `'a` is the lifetime of this `JNIEnv`. This method creates a new local reference with
    /// lifetime `'a`.
    ///
    /// `'b` is the lifetime of the original reference. It can be any valid lifetime, even one that
    /// `'a` outlives or vice versa.
    ///
    /// Think of `'a` as meaning `'new` and `'b` as meaning `'original`. (It is unfortunately not
    /// possible to actually give these names to the two lifetimes because `'a` is a parameter to
    /// the `JNIEnv` type, not a parameter to this method.)
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
    ///     fn extract_throwable(self, env: JNIEnv) -> jni::errors::Result<JThrowable> {
    ///         let throwable: JObject = match self {
    ///             ExampleError::Exception(exception) => {
    ///                 // The error was caused by a Java exception.
    ///
    ///                 // Here, `exception` is a `GlobalRef` pointing to a Java `Throwable`. It
    ///                 // will be dropped at the end of this `match` arm. We'll use
    ///                 // `new_local_ref` to create a local reference that will outlive the
    ///                 // `GlobalRef`.
    ///
    ///                 env.new_local_ref(exception.as_obj())?
    ///             }
    ///
    ///             ExampleError::Other(error) => {
    ///                 // The error was caused by something that happened in Rust code. Create a
    ///                 // new `java.lang.Error` to represent it.
    ///
    ///                 env.new_object(
    ///                     "java/lang/Error",
    ///                     "(Ljava/lang/String;)V",
    ///                     &[
    ///                         env.new_string(error.to_string())?.into(),
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
    pub fn new_local_ref<'b, O>(&self, obj: O) -> Result<JObject<'a>>
    where
        O: Into<JObject<'b>>,
    {
        let local = jni_unchecked!(self.internal, NewLocalRef, obj.into().into_raw());
        Ok(unsafe { JObject::from_raw(local) })
    }

    /// Creates a new auto-deleted local reference.
    ///
    /// See also [`with_local_frame`](struct.JNIEnv.html#method.with_local_frame) method that
    /// can be more convenient when you create a _bounded_ number of local references
    /// but cannot rely on automatic de-allocation (e.g., in case of recursion, deep call stacks,
    /// [permanently-attached](struct.JavaVM.html#attaching-native-threads) native threads, etc.).
    pub fn auto_local<'b, O>(&'b self, obj: O) -> AutoLocal<'a, 'b>
    where
        O: Into<JObject<'a>>,
    {
        AutoLocal::new(self, obj.into())
    }

    /// Deletes the local reference.
    ///
    /// Local references are valid for the duration of a native method call.
    /// They are
    /// freed automatically after the native method returns. Each local
    /// reference costs
    /// some amount of Java Virtual Machine resource. Programmers need to make
    /// sure that
    /// native methods do not excessively allocate local references. Although
    /// local
    /// references are automatically freed after the native method returns to
    /// Java,
    /// excessive allocation of local references may cause the VM to run out of
    /// memory
    /// during the execution of a native method.
    ///
    /// In most cases it is better to use `AutoLocal` (see `auto_local` method)
    /// or `with_local_frame` instead of direct `delete_local_ref` calls.
    pub fn delete_local_ref(&self, obj: JObject) -> Result<()> {
        jni_unchecked!(self.internal, DeleteLocalRef, obj.into_raw());
        Ok(())
    }

    /// Creates a new local reference frame, in which at least a given number
    /// of local references can be created.
    ///
    /// Returns `Err` on failure, with a pending `OutOfMemoryError`.
    ///
    /// Prefer to use [`with_local_frame`](struct.JNIEnv.html#method.with_local_frame) instead of
    /// direct `push_local_frame`/`pop_local_frame` calls.
    ///
    /// See also [`auto_local`](struct.JNIEnv.html#method.auto_local) method
    /// and `AutoLocal` type — that approach can be more convenient in loops.
    pub fn push_local_frame(&self, capacity: i32) -> Result<()> {
        // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
        let res = jni_unchecked!(self.internal, PushLocalFrame, capacity);
        jni_error_code_to_result(res)
    }

    /// Pops off the current local reference frame, frees all the local
    /// references allocated on the current stack frame, except the `result`,
    /// which is returned from this function and remains valid.
    ///
    /// The resulting `JObject` will be `NULL` iff `result` is `NULL`.
    pub fn pop_local_frame(&self, result: JObject<'a>) -> Result<JObject<'a>> {
        // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
        Ok(unsafe {
            JObject::from_raw(jni_unchecked!(
                self.internal,
                PopLocalFrame,
                result.into_raw()
            ))
        })
    }

    /// Executes the given function in a new local reference frame, in which at least a given number
    /// of references can be created. Once this method returns, all references allocated
    /// in the frame are freed, except the one that the function returns, which remains valid.
    ///
    /// If _no_ new frames can be allocated, returns `Err` with a pending `OutOfMemoryError`.
    ///
    /// See also [`auto_local`](struct.JNIEnv.html#method.auto_local) method
    /// and `AutoLocal` type - that approach can be more convenient in loops.
    pub fn with_local_frame<F>(&self, capacity: i32, f: F) -> Result<JObject<'a>>
    where
        F: FnOnce() -> Result<JObject<'a>>,
    {
        self.push_local_frame(capacity)?;
        let res = f();
        match res {
            Ok(obj) => self.pop_local_frame(obj),
            Err(e) => {
                self.pop_local_frame(JObject::null())?;
                Err(e)
            }
        }
    }

    /// Allocates a new object from a class descriptor without running a
    /// constructor.
    pub fn alloc_object<'c, T>(&self, class: T) -> Result<JObject<'a>>
    where
        T: Desc<'a, JClass<'c>>,
    {
        let class = class.lookup(self)?;
        let obj = jni_non_null_call!(self.internal, AllocObject, class.into_raw());
        Ok(unsafe { JObject::from_raw(obj) })
    }

    /// Common functionality for finding methods.
    #[allow(clippy::redundant_closure_call)]
    fn get_method_id_base<'c, T, U, V, C, R>(
        &self,
        class: T,
        name: U,
        sig: V,
        get_method: C,
    ) -> Result<R>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
        C: for<'d> Fn(&JClass<'d>, &JNIString, &JNIString) -> Result<R>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.into();
        let sig = sig.into();

        let res: Result<R> = catch!({ get_method(&class, &ffi_name, &sig) });

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
    /// ```rust,ignore
    /// let method_id: JMethodID =
    ///     env.get_method_id("java/lang/String", "substring", "(II)Ljava/lang/String;");
    /// ```
    pub fn get_method_id<'c, T, U, V>(&self, class: T, name: U, sig: V) -> Result<JMethodID>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        self.get_method_id_base(class, name, sig, |class, name, sig| {
            let method_id = jni_non_null_call!(
                self.internal,
                GetMethodID,
                class.into_raw(),
                name.as_ptr(),
                sig.as_ptr()
            );
            Ok(unsafe { JMethodID::from_raw(method_id) })
        })
    }

    /// Look up a static method by class descriptor, name, and
    /// signature.
    ///
    /// # Example
    /// ```rust,ignore
    /// let method_id: JMethodID =
    ///     env.get_static_method_id("java/lang/String", "valueOf", "(I)Ljava/lang/String;");
    /// ```
    pub fn get_static_method_id<'c, T, U, V>(
        &self,
        class: T,
        name: U,
        sig: V,
    ) -> Result<JStaticMethodID>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        self.get_method_id_base(class, name, sig, |class, name, sig| {
            let method_id = jni_non_null_call!(
                self.internal,
                GetStaticMethodID,
                class.into_raw(),
                name.as_ptr(),
                sig.as_ptr()
            );
            Ok(unsafe { JStaticMethodID::from_raw(method_id) })
        })
    }

    /// Look up the field ID for a class/name/type combination.
    ///
    /// # Example
    /// ```rust,ignore
    /// let field_id = env.get_field_id("com/my/Class", "intField", "I");
    /// ```
    pub fn get_field_id<'c, T, U, V>(&self, class: T, name: U, sig: V) -> Result<JFieldID>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.into();
        let ffi_sig = sig.into();

        let res: Result<JFieldID> = catch!({
            let field_id = jni_non_null_call!(
                self.internal,
                GetFieldID,
                class.into_raw(),
                ffi_name.as_ptr(),
                ffi_sig.as_ptr()
            );
            Ok(unsafe { JFieldID::from_raw(field_id) })
        });

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
    /// ```rust,ignore
    /// let field_id = env.get_static_field_id("com/my/Class", "intField", "I");
    /// ```
    pub fn get_static_field_id<'c, T, U, V>(
        &self,
        class: T,
        name: U,
        sig: V,
    ) -> Result<JStaticFieldID>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.into();
        let ffi_sig = sig.into();

        let res: Result<JStaticFieldID> = catch!({
            let field_id = jni_non_null_call!(
                self.internal,
                GetStaticFieldID,
                class.into_raw(),
                ffi_name.as_ptr(),
                ffi_sig.as_ptr()
            );
            Ok(unsafe { JStaticFieldID::from_raw(field_id) })
        });

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
    pub fn get_object_class<'b, O>(&self, obj: O) -> Result<JClass<'a>>
    where
        O: Into<JObject<'b>>,
    {
        let obj = obj.into();
        non_null!(obj, "get_object_class");
        Ok(unsafe {
            JClass::from_raw(jni_unchecked!(
                self.internal,
                GetObjectClass,
                obj.into_raw()
            ))
        })
    }

    /// Call a static method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the class, whether the return
    /// type is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `CallStatic<Type>MethodA` method
    /// with the provided arguments.
    pub fn call_static_method_unchecked<'c, T, U>(
        &self,
        class: T,
        method_id: U,
        ret: ReturnType,
        args: &[jvalue],
    ) -> Result<JValue<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Desc<'a, JStaticMethodID>,
    {
        let class = class.lookup(self)?;

        let method_id = method_id.lookup(self)?.into_raw();

        let class = class.into_raw();
        let jni_args = args.as_ptr();

        // TODO clean this up
        Ok(match ret {
            ReturnType::Object | ReturnType::Array => {
                let obj = jni_non_void_call!(
                    self.internal,
                    CallStaticObjectMethodA,
                    class,
                    method_id,
                    jni_args
                );
                let obj = unsafe { JObject::from_raw(obj) };
                obj.into()
            }
            ReturnType::Primitive(p) => match p {
                Primitive::Boolean => jni_non_void_call!(
                    self.internal,
                    CallStaticBooleanMethodA,
                    class,
                    method_id,
                    jni_args
                )
                .into(),
                Primitive::Char => jni_non_void_call!(
                    self.internal,
                    CallStaticCharMethodA,
                    class,
                    method_id,
                    jni_args
                )
                .into(),
                Primitive::Short => jni_non_void_call!(
                    self.internal,
                    CallStaticShortMethodA,
                    class,
                    method_id,
                    jni_args
                )
                .into(),
                Primitive::Int => jni_non_void_call!(
                    self.internal,
                    CallStaticIntMethodA,
                    class,
                    method_id,
                    jni_args
                )
                .into(),
                Primitive::Long => jni_non_void_call!(
                    self.internal,
                    CallStaticLongMethodA,
                    class,
                    method_id,
                    jni_args
                )
                .into(),
                Primitive::Float => jni_non_void_call!(
                    self.internal,
                    CallStaticFloatMethodA,
                    class,
                    method_id,
                    jni_args
                )
                .into(),
                Primitive::Double => jni_non_void_call!(
                    self.internal,
                    CallStaticDoubleMethodA,
                    class,
                    method_id,
                    jni_args
                )
                .into(),
                Primitive::Byte => jni_non_void_call!(
                    self.internal,
                    CallStaticByteMethodA,
                    class,
                    method_id,
                    jni_args
                )
                .into(),
                Primitive::Void => {
                    jni_void_call!(
                        self.internal,
                        CallStaticVoidMethodA,
                        class,
                        method_id,
                        jni_args
                    );
                    return Ok(JValue::Void);
                }
            }, // JavaType::Primitive
        }) // match parsed.ret
    }

    /// Call an object method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the object, whether the return
    /// type is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `Call<Type>MethodA` method with
    /// the provided arguments.
    pub fn call_method_unchecked<O, T>(
        &self,
        obj: O,
        method_id: T,
        ret: ReturnType,
        args: &[jvalue],
    ) -> Result<JValue<'a>>
    where
        O: Into<JObject<'a>>,
        T: Desc<'a, JMethodID>,
    {
        let method_id = method_id.lookup(self)?.into_raw();

        let obj = obj.into().into_raw();

        let jni_args = args.as_ptr();

        // TODO clean this up
        Ok(match ret {
            ReturnType::Object | ReturnType::Array => {
                let obj =
                    jni_non_void_call!(self.internal, CallObjectMethodA, obj, method_id, jni_args);
                let obj = unsafe { JObject::from_raw(obj) };
                obj.into()
            }
            ReturnType::Primitive(p) => match p {
                Primitive::Boolean => {
                    jni_non_void_call!(self.internal, CallBooleanMethodA, obj, method_id, jni_args)
                        .into()
                }
                Primitive::Char => {
                    jni_non_void_call!(self.internal, CallCharMethodA, obj, method_id, jni_args)
                        .into()
                }
                Primitive::Short => {
                    jni_non_void_call!(self.internal, CallShortMethodA, obj, method_id, jni_args)
                        .into()
                }
                Primitive::Int => {
                    jni_non_void_call!(self.internal, CallIntMethodA, obj, method_id, jni_args)
                        .into()
                }
                Primitive::Long => {
                    jni_non_void_call!(self.internal, CallLongMethodA, obj, method_id, jni_args)
                        .into()
                }
                Primitive::Float => {
                    jni_non_void_call!(self.internal, CallFloatMethodA, obj, method_id, jni_args)
                        .into()
                }
                Primitive::Double => {
                    jni_non_void_call!(self.internal, CallDoubleMethodA, obj, method_id, jni_args)
                        .into()
                }
                Primitive::Byte => {
                    jni_non_void_call!(self.internal, CallByteMethodA, obj, method_id, jni_args)
                        .into()
                }
                Primitive::Void => {
                    jni_void_call!(self.internal, CallVoidMethodA, obj, method_id, jni_args);
                    return Ok(JValue::Void);
                }
            }, // JavaType::Primitive
        }) // match parsed.ret
    }

    /// Calls an object method safely. This comes with a number of
    /// lookups/checks. It
    ///
    /// * Parses the type signature to find the number of arguments and return
    ///   type
    /// * Looks up the JClass for the given object.
    /// * Looks up the JMethodID for the class/name/signature combination
    /// * Ensures that the number of args matches the signature
    /// * Calls `call_method_unchecked` with the verified safe arguments.
    ///
    /// Note: this may cause a java exception if the arguments are the wrong
    /// type, in addition to if the method itself throws.
    pub fn call_method<O, S, T>(
        &self,
        obj: O,
        name: S,
        sig: T,
        args: &[JValue],
    ) -> Result<JValue<'a>>
    where
        O: Into<JObject<'a>>,
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let obj = obj.into();
        non_null!(obj, "call_method obj argument");

        // parse the signature
        let parsed = TypeSignature::from_str(sig.as_ref())?;
        if parsed.args.len() != args.len() {
            return Err(Error::InvalidArgList(parsed));
        }

        let class = self.auto_local(self.get_object_class(obj)?);

        let args: Vec<jvalue> = args.iter().map(|v| v.to_jni()).collect();
        self.call_method_unchecked(obj, (&class, name, sig), parsed.ret, &args)
    }

    /// Calls a static method safely. This comes with a number of
    /// lookups/checks. It
    ///
    /// * Parses the type signature to find the number of arguments and return
    ///   type
    /// * Looks up the JMethodID for the class/name/signature combination
    /// * Ensures that the number of args matches the signature
    /// * Calls `call_method_unchecked` with the verified safe arguments.
    ///
    /// Note: this may cause a java exception if the arguments are the wrong
    /// type, in addition to if the method itself throws.
    pub fn call_static_method<'c, T, U, V>(
        &self,
        class: T,
        name: U,
        sig: V,
        args: &[JValue],
    ) -> Result<JValue<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString> + AsRef<str>,
    {
        let parsed = TypeSignature::from_str(&sig)?;
        if parsed.args.len() != args.len() {
            return Err(Error::InvalidArgList(parsed));
        }

        // go ahead and look up the class since it's already Copy,
        // and we'll need that for the next call.
        let class = class.lookup(self)?;

        let args: Vec<jvalue> = args.iter().map(|v| v.to_jni()).collect();
        self.call_static_method_unchecked(class, (class, name, sig), parsed.ret, &args)
    }

    /// Create a new object using a constructor. This is done safely using
    /// checks similar to those in `call_static_method`.
    pub fn new_object<'c, T, U>(
        &self,
        class: T,
        ctor_sig: U,
        ctor_args: &[JValue],
    ) -> Result<JObject<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString> + AsRef<str>,
    {
        // parse the signature
        let parsed = TypeSignature::from_str(&ctor_sig)?;

        if parsed.args.len() != ctor_args.len() {
            return Err(Error::InvalidArgList(parsed));
        }

        if parsed.ret != ReturnType::Primitive(Primitive::Void) {
            return Err(Error::InvalidCtorReturn);
        }

        // build strings
        let class = class.lookup(self)?;

        let method_id: JMethodID = (class, ctor_sig).lookup(self)?;

        self.new_object_unchecked(class, method_id, ctor_args)
    }

    /// Create a new object using a constructor. Arguments aren't checked
    /// because
    /// of the `JMethodID` usage.
    pub fn new_object_unchecked<'c, T>(
        &self,
        class: T,
        ctor_id: JMethodID,
        ctor_args: &[JValue],
    ) -> Result<JObject<'a>>
    where
        T: Desc<'a, JClass<'c>>,
    {
        let class = class.lookup(self)?;

        let jni_args: Vec<jvalue> = ctor_args.iter().map(|v| v.to_jni()).collect();
        let jni_args = jni_args.as_ptr();

        let obj = jni_non_null_call!(
            self.internal,
            NewObjectA,
            class.into_raw(),
            ctor_id.into_raw(),
            jni_args
        );
        Ok(unsafe { JObject::from_raw(obj) })
    }

    /// Cast a JObject to a `JList`. This won't throw exceptions or return errors
    /// in the event that the object isn't actually a list, but the methods on
    /// the resulting map object will.
    pub fn get_list(&self, obj: JObject<'a>) -> Result<JList<'a, '_>> {
        non_null!(obj, "get_list obj argument");
        JList::from_env(self, obj)
    }

    /// Cast a JObject to a JMap. This won't throw exceptions or return errors
    /// in the event that the object isn't actually a map, but the methods on
    /// the resulting map object will.
    pub fn get_map(&self, obj: JObject<'a>) -> Result<JMap<'a, '_>> {
        non_null!(obj, "get_map obj argument");
        JMap::from_env(self, obj)
    }

    /// Get a JavaStr from a JString. This allows conversions from java string
    /// objects to rust strings.
    ///
    /// This entails a call to `GetStringUTFChars` and only decodes java's
    /// modified UTF-8 format on conversion to a rust-compatible string.
    ///
    /// # Panics
    ///
    /// This call panics when given an Object that is not a java.lang.String
    pub fn get_string(&self, obj: JString<'a>) -> Result<JavaStr<'a, '_>> {
        non_null!(obj, "get_string obj argument");
        JavaStr::from_env(self, obj)
    }

    /// Get a pointer to the character array beneath a JString.
    ///
    /// Array contains Java's modified UTF-8.
    ///
    /// # Attention
    /// This will leak memory if `release_string_utf_chars` is never called.
    pub fn get_string_utf_chars(&self, obj: JString) -> Result<*const c_char> {
        non_null!(obj, "get_string_utf_chars obj argument");
        let ptr: *const c_char = jni_non_null_call!(
            self.internal,
            GetStringUTFChars,
            obj.into_raw(),
            ::std::ptr::null::<jboolean>() as *mut jboolean
        );
        Ok(ptr)
    }

    /// Unpin the array returned by `get_string_utf_chars`.
    ///
    /// # Safety
    ///
    /// The behaviour is undefined if the array isn't returned by the `get_string_utf_chars`
    /// function.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # let env = unsafe { jni::JNIEnv::from_raw(std::ptr::null_mut()).unwrap() };
    /// let s = env.new_string("test").unwrap();
    /// let array = env.get_string_utf_chars(s).unwrap();
    /// unsafe { env.release_string_utf_chars(s, array).unwrap() };
    /// ```
    #[allow(unused_unsafe)]
    pub unsafe fn release_string_utf_chars(&self, obj: JString, arr: *const c_char) -> Result<()> {
        non_null!(obj, "release_string_utf_chars obj argument");
        // This method is safe to call in case of pending exceptions (see the chapter 2 of the spec)
        jni_unchecked!(self.internal, ReleaseStringUTFChars, obj.into_raw(), arr);
        Ok(())
    }

    /// Create a new java string object from a rust string. This requires a
    /// re-encoding of rusts *real* UTF-8 strings to java's modified UTF-8
    /// format.
    pub fn new_string<S: Into<JNIString>>(&self, from: S) -> Result<JString<'a>> {
        let ffi_str = from.into();
        let s = jni_non_null_call!(self.internal, NewStringUTF, ffi_str.as_ptr());
        Ok(unsafe { JString::from_raw(s) })
    }

    /// Get the length of a java array
    pub fn get_array_length(&self, array: jarray) -> Result<jsize> {
        non_null!(array, "get_array_length array argument");
        let len: jsize = jni_unchecked!(self.internal, GetArrayLength, array);
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
    pub fn new_object_array<'c, T, U>(
        &self,
        length: jsize,
        element_class: T,
        initial_element: U,
    ) -> Result<jobjectArray>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JObject<'a>>,
    {
        let class = element_class.lookup(self)?;
        Ok(jni_non_null_call!(
            self.internal,
            NewObjectArray,
            length,
            class.into_raw(),
            initial_element.into().into_raw()
        ))
    }

    /// Returns an element of the `jobjectArray` array.
    pub fn get_object_array_element(
        &self,
        array: jobjectArray,
        index: jsize,
    ) -> Result<JObject<'a>> {
        non_null!(array, "get_object_array_element array argument");
        Ok(unsafe {
            JObject::from_raw(jni_non_void_call!(
                self.internal,
                GetObjectArrayElement,
                array,
                index
            ))
        })
    }

    /// Sets an element of the `jobjectArray` array.
    pub fn set_object_array_element<O>(
        &self,
        array: jobjectArray,
        index: jsize,
        value: O,
    ) -> Result<()>
    where
        O: Into<JObject<'a>>,
    {
        non_null!(array, "set_object_array_element array argument");
        jni_void_call!(
            self.internal,
            SetObjectArrayElement,
            array,
            index,
            value.into().into_raw()
        );
        Ok(())
    }

    /// Create a new java byte array from a rust byte slice.
    pub fn byte_array_from_slice(&self, buf: &[u8]) -> Result<jbyteArray> {
        let length = buf.len() as i32;
        let bytes: jbyteArray = self.new_byte_array(length)?;
        jni_unchecked!(
            self.internal,
            SetByteArrayRegion,
            bytes,
            0,
            length,
            buf.as_ptr() as *const i8
        );
        Ok(bytes)
    }

    /// Converts a java byte array to a rust vector of bytes.
    pub fn convert_byte_array(&self, array: jbyteArray) -> Result<Vec<u8>> {
        non_null!(array, "convert_byte_array array argument");
        let length = jni_non_void_call!(self.internal, GetArrayLength, array);
        let mut vec = vec![0u8; length as usize];
        jni_unchecked!(
            self.internal,
            GetByteArrayRegion,
            array,
            0,
            length,
            vec.as_mut_ptr() as *mut i8
        );
        Ok(vec)
    }

    /// Create a new java boolean array of supplied length.
    pub fn new_boolean_array(&self, length: jsize) -> Result<jbooleanArray> {
        let array: jbooleanArray = jni_non_null_call!(self.internal, NewBooleanArray, length);
        Ok(array)
    }

    /// Create a new java byte array of supplied length.
    pub fn new_byte_array(&self, length: jsize) -> Result<jbyteArray> {
        let array: jbyteArray = jni_non_null_call!(self.internal, NewByteArray, length);
        Ok(array)
    }

    /// Create a new java char array of supplied length.
    pub fn new_char_array(&self, length: jsize) -> Result<jcharArray> {
        let array: jcharArray = jni_non_null_call!(self.internal, NewCharArray, length);
        Ok(array)
    }

    /// Create a new java short array of supplied length.
    pub fn new_short_array(&self, length: jsize) -> Result<jshortArray> {
        let array: jshortArray = jni_non_null_call!(self.internal, NewShortArray, length);
        Ok(array)
    }

    /// Create a new java int array of supplied length.
    pub fn new_int_array(&self, length: jsize) -> Result<jintArray> {
        let array: jintArray = jni_non_null_call!(self.internal, NewIntArray, length);
        Ok(array)
    }

    /// Create a new java long array of supplied length.
    pub fn new_long_array(&self, length: jsize) -> Result<jlongArray> {
        let array: jlongArray = jni_non_null_call!(self.internal, NewLongArray, length);
        Ok(array)
    }

    /// Create a new java float array of supplied length.
    pub fn new_float_array(&self, length: jsize) -> Result<jfloatArray> {
        let array: jfloatArray = jni_non_null_call!(self.internal, NewFloatArray, length);
        Ok(array)
    }

    /// Create a new java double array of supplied length.
    pub fn new_double_array(&self, length: jsize) -> Result<jdoubleArray> {
        let array: jdoubleArray = jni_non_null_call!(self.internal, NewDoubleArray, length);
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
    pub fn get_boolean_array_region(
        &self,
        array: jbooleanArray,
        start: jsize,
        buf: &mut [jboolean],
    ) -> Result<()> {
        non_null!(array, "get_boolean_array_region array argument");
        jni_void_call!(
            self.internal,
            GetBooleanArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_mut_ptr()
        );
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
    pub fn get_byte_array_region(
        &self,
        array: jbyteArray,
        start: jsize,
        buf: &mut [jbyte],
    ) -> Result<()> {
        non_null!(array, "get_byte_array_region array argument");
        jni_void_call!(
            self.internal,
            GetByteArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_mut_ptr()
        );
        Ok(())
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
    pub fn get_char_array_region(
        &self,
        array: jcharArray,
        start: jsize,
        buf: &mut [jchar],
    ) -> Result<()> {
        non_null!(array, "get_char_array_region array argument");
        jni_void_call!(
            self.internal,
            GetCharArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_mut_ptr()
        );
        Ok(())
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
    pub fn get_short_array_region(
        &self,
        array: jshortArray,
        start: jsize,
        buf: &mut [jshort],
    ) -> Result<()> {
        non_null!(array, "get_short_array_region array argument");
        jni_void_call!(
            self.internal,
            GetShortArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_mut_ptr()
        );
        Ok(())
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
    pub fn get_int_array_region(
        &self,
        array: jintArray,
        start: jsize,
        buf: &mut [jint],
    ) -> Result<()> {
        non_null!(array, "get_int_array_region array argument");
        jni_void_call!(
            self.internal,
            GetIntArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_mut_ptr()
        );
        Ok(())
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
    pub fn get_long_array_region(
        &self,
        array: jlongArray,
        start: jsize,
        buf: &mut [jlong],
    ) -> Result<()> {
        non_null!(array, "get_long_array_region array argument");
        jni_void_call!(
            self.internal,
            GetLongArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_mut_ptr()
        );
        Ok(())
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
    pub fn get_float_array_region(
        &self,
        array: jfloatArray,
        start: jsize,
        buf: &mut [jfloat],
    ) -> Result<()> {
        non_null!(array, "get_float_array_region array argument");
        jni_void_call!(
            self.internal,
            GetFloatArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_mut_ptr()
        );
        Ok(())
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
    pub fn get_double_array_region(
        &self,
        array: jdoubleArray,
        start: jsize,
        buf: &mut [jdouble],
    ) -> Result<()> {
        non_null!(array, "get_double_array_region array argument");
        jni_void_call!(
            self.internal,
            GetDoubleArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_mut_ptr()
        );
        Ok(())
    }

    /// Copy the contents of the `buf` slice to the java boolean array at the
    /// `start` index.
    pub fn set_boolean_array_region(
        &self,
        array: jbooleanArray,
        start: jsize,
        buf: &[jboolean],
    ) -> Result<()> {
        non_null!(array, "set_boolean_array_region array argument");
        jni_void_call!(
            self.internal,
            SetBooleanArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_ptr()
        );
        Ok(())
    }

    /// Copy the contents of the `buf` slice to the java byte array at the
    /// `start` index.
    pub fn set_byte_array_region(
        &self,
        array: jbyteArray,
        start: jsize,
        buf: &[jbyte],
    ) -> Result<()> {
        non_null!(array, "set_byte_array_region array argument");
        jni_void_call!(
            self.internal,
            SetByteArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_ptr()
        );
        Ok(())
    }

    /// Copy the contents of the `buf` slice to the java char array at the
    /// `start` index.
    pub fn set_char_array_region(
        &self,
        array: jcharArray,
        start: jsize,
        buf: &[jchar],
    ) -> Result<()> {
        non_null!(array, "set_char_array_region array argument");
        jni_void_call!(
            self.internal,
            SetCharArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_ptr()
        );
        Ok(())
    }

    /// Copy the contents of the `buf` slice to the java short array at the
    /// `start` index.
    pub fn set_short_array_region(
        &self,
        array: jshortArray,
        start: jsize,
        buf: &[jshort],
    ) -> Result<()> {
        non_null!(array, "set_short_array_region array argument");
        jni_void_call!(
            self.internal,
            SetShortArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_ptr()
        );
        Ok(())
    }

    /// Copy the contents of the `buf` slice to the java int array at the
    /// `start` index.
    pub fn set_int_array_region(&self, array: jintArray, start: jsize, buf: &[jint]) -> Result<()> {
        non_null!(array, "set_int_array_region array argument");
        jni_void_call!(
            self.internal,
            SetIntArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_ptr()
        );
        Ok(())
    }

    /// Copy the contents of the `buf` slice to the java long array at the
    /// `start` index.
    pub fn set_long_array_region(
        &self,
        array: jlongArray,
        start: jsize,
        buf: &[jlong],
    ) -> Result<()> {
        non_null!(array, "set_long_array_region array argument");
        jni_void_call!(
            self.internal,
            SetLongArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_ptr()
        );
        Ok(())
    }

    /// Copy the contents of the `buf` slice to the java float array at the
    /// `start` index.
    pub fn set_float_array_region(
        &self,
        array: jfloatArray,
        start: jsize,
        buf: &[jfloat],
    ) -> Result<()> {
        non_null!(array, "set_float_array_region array argument");
        jni_void_call!(
            self.internal,
            SetFloatArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_ptr()
        );
        Ok(())
    }

    /// Copy the contents of the `buf` slice to the java double array at the
    /// `start` index.
    pub fn set_double_array_region(
        &self,
        array: jdoubleArray,
        start: jsize,
        buf: &[jdouble],
    ) -> Result<()> {
        non_null!(array, "set_double_array_region array argument");
        jni_void_call!(
            self.internal,
            SetDoubleArrayRegion,
            array,
            start,
            buf.len() as jsize,
            buf.as_ptr()
        );
        Ok(())
    }

    /// Get a field without checking the provided type against the actual field.
    pub fn get_field_unchecked<O, T>(&self, obj: O, field: T, ty: ReturnType) -> Result<JValue<'a>>
    where
        O: Into<JObject<'a>>,
        T: Desc<'a, JFieldID>,
    {
        let obj = obj.into();
        non_null!(obj, "get_field_typed obj argument");

        let field = field.lookup(self)?.into_raw();
        let obj = obj.into_raw();

        // TODO clean this up
        Ok(match ty {
            ReturnType::Object | ReturnType::Array => {
                let obj = jni_non_void_call!(self.internal, GetObjectField, obj, field);
                let obj = unsafe { JObject::from_raw(obj) };
                obj.into()
            }
            ReturnType::Primitive(p) => match p {
                Primitive::Boolean => {
                    jni_unchecked!(self.internal, GetBooleanField, obj, field).into()
                }
                Primitive::Char => jni_unchecked!(self.internal, GetCharField, obj, field).into(),
                Primitive::Short => jni_unchecked!(self.internal, GetShortField, obj, field).into(),
                Primitive::Int => jni_unchecked!(self.internal, GetIntField, obj, field).into(),
                Primitive::Long => jni_unchecked!(self.internal, GetLongField, obj, field).into(),
                Primitive::Float => jni_unchecked!(self.internal, GetFloatField, obj, field).into(),
                Primitive::Double => {
                    jni_unchecked!(self.internal, GetDoubleField, obj, field).into()
                }
                Primitive::Byte => jni_unchecked!(self.internal, GetByteField, obj, field).into(),
                Primitive::Void => {
                    return Err(Error::WrongJValueType("void", "see java field"));
                }
            },
        })
    }

    /// Set a field without any type checking.
    pub fn set_field_unchecked<O, T>(&self, obj: O, field: T, val: JValue) -> Result<()>
    where
        O: Into<JObject<'a>>,
        T: Desc<'a, JFieldID>,
    {
        let obj = obj.into();
        non_null!(obj, "set_field_typed obj argument");

        let field = field.lookup(self)?.into_raw();
        let obj = obj.into_raw();

        // TODO clean this up
        match val {
            JValue::Object(o) => {
                jni_unchecked!(self.internal, SetObjectField, obj, field, o.into_raw());
            }
            // JavaType::Object
            JValue::Bool(b) => {
                jni_unchecked!(self.internal, SetBooleanField, obj, field, b);
            }
            JValue::Char(c) => {
                jni_unchecked!(self.internal, SetCharField, obj, field, c);
            }
            JValue::Short(s) => {
                jni_unchecked!(self.internal, SetShortField, obj, field, s);
            }
            JValue::Int(i) => {
                jni_unchecked!(self.internal, SetIntField, obj, field, i);
            }
            JValue::Long(l) => {
                jni_unchecked!(self.internal, SetLongField, obj, field, l);
            }
            JValue::Float(f) => {
                jni_unchecked!(self.internal, SetFloatField, obj, field, f);
            }
            JValue::Double(d) => {
                jni_unchecked!(self.internal, SetDoubleField, obj, field, d);
            }
            JValue::Byte(b) => {
                jni_unchecked!(self.internal, SetByteField, obj, field, b);
            }
            JValue::Void => {
                return Err(Error::WrongJValueType("void", "see java field"));
            }
        };

        Ok(())
    }

    /// Get a field. Requires an object class lookup and a field id lookup
    /// internally.
    pub fn get_field<O, S, T>(&self, obj: O, name: S, ty: T) -> Result<JValue<'a>>
    where
        O: Into<JObject<'a>>,
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let obj = obj.into();
        let class = self.auto_local(self.get_object_class(obj)?);

        let parsed = ReturnType::from_str(ty.as_ref())?;

        let field_id: JFieldID = (&class, name, ty).lookup(self)?;

        self.get_field_unchecked(obj, field_id, parsed)
    }

    /// Set a field. Does the same lookups as `get_field` and ensures that the
    /// type matches the given value.
    pub fn set_field<O, S, T>(&self, obj: O, name: S, ty: T, val: JValue) -> Result<()>
    where
        O: Into<JObject<'a>>,
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let obj = obj.into();
        let parsed = JavaType::from_str(ty.as_ref())?;
        let in_type = val.primitive_type();

        match parsed {
            JavaType::Object(_) | JavaType::Array(_) => {
                if in_type.is_some() {
                    return Err(Error::WrongJValueType(val.type_name(), "see java field"));
                }
            }
            JavaType::Primitive(p) => {
                if let Some(in_p) = in_type {
                    if in_p == p {
                        // good
                    } else {
                        return Err(Error::WrongJValueType(val.type_name(), "see java field"));
                    }
                } else {
                    return Err(Error::WrongJValueType(val.type_name(), "see java field"));
                }
            }
            JavaType::Method(_) => unimplemented!(),
        }

        let class = self.auto_local(self.get_object_class(obj)?);

        self.set_field_unchecked(obj, (&class, name, ty), val)
    }

    /// Get a static field without checking the provided type against the actual
    /// field.
    pub fn get_static_field_unchecked<'c, T, U>(
        &self,
        class: T,
        field: U,
        ty: JavaType,
    ) -> Result<JValue<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Desc<'a, JStaticFieldID>,
    {
        use JavaType::Primitive as JP;

        let class = class.lookup(self)?.into_raw();
        let field = field.lookup(self)?.into_raw();

        let result = match ty {
            JavaType::Object(_) | JavaType::Array(_) => {
                let obj = jni_non_void_call!(self.internal, GetStaticObjectField, class, field);
                let obj = unsafe { JObject::from_raw(obj) };
                obj.into()
            }
            JavaType::Method(_) => return Err(Error::WrongJValueType("Method", "see java field")),
            JP(Primitive::Boolean) => {
                jni_unchecked!(self.internal, GetStaticBooleanField, class, field).into()
            }
            JP(Primitive::Char) => {
                jni_unchecked!(self.internal, GetStaticCharField, class, field).into()
            }
            JP(Primitive::Short) => {
                jni_unchecked!(self.internal, GetStaticShortField, class, field).into()
            }
            JP(Primitive::Int) => {
                jni_unchecked!(self.internal, GetStaticIntField, class, field).into()
            }
            JP(Primitive::Long) => {
                jni_unchecked!(self.internal, GetStaticLongField, class, field).into()
            }
            JP(Primitive::Float) => {
                jni_unchecked!(self.internal, GetStaticFloatField, class, field).into()
            }
            JP(Primitive::Double) => {
                jni_unchecked!(self.internal, GetStaticDoubleField, class, field).into()
            }
            JP(Primitive::Byte) => {
                jni_unchecked!(self.internal, GetStaticByteField, class, field).into()
            }
            JP(Primitive::Void) => return Err(Error::WrongJValueType("void", "see java field")),
        };
        Ok(result)
    }

    /// Get a static field. Requires a class lookup and a field id lookup
    /// internally.
    pub fn get_static_field<'c, T, U, V>(&self, class: T, field: U, sig: V) -> Result<JValue<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString> + AsRef<str>,
    {
        let ty = JavaType::from_str(sig.as_ref())?;

        // go ahead and look up the class since it's already Copy,
        // and we'll need that for the next call.
        let class = class.lookup(self)?;

        self.get_static_field_unchecked(class, (class, field, sig), ty)
    }

    /// Set a static field. Requires a class lookup and a field id lookup internally.
    pub fn set_static_field<'c, T, U>(&self, class: T, field: U, value: JValue) -> Result<()>
    where
        T: Desc<'a, JClass<'c>>,
        U: Desc<'a, JStaticFieldID>,
    {
        let class = class.lookup(self)?.into_raw();
        let field = field.lookup(self)?.into_raw();

        match value {
            JValue::Object(v) => jni_unchecked!(
                self.internal,
                SetStaticObjectField,
                class,
                field,
                v.into_raw()
            ),
            JValue::Byte(v) => jni_unchecked!(self.internal, SetStaticByteField, class, field, v),
            JValue::Char(v) => jni_unchecked!(self.internal, SetStaticCharField, class, field, v),
            JValue::Short(v) => jni_unchecked!(self.internal, SetStaticShortField, class, field, v),
            JValue::Int(v) => jni_unchecked!(self.internal, SetStaticIntField, class, field, v),
            JValue::Long(v) => jni_unchecked!(self.internal, SetStaticLongField, class, field, v),
            JValue::Bool(v) => {
                jni_unchecked!(self.internal, SetStaticBooleanField, class, field, v)
            }
            JValue::Float(v) => jni_unchecked!(self.internal, SetStaticFloatField, class, field, v),
            JValue::Double(v) => {
                jni_unchecked!(self.internal, SetStaticDoubleField, class, field, v)
            }
            JValue::Void => return Err(Error::WrongJValueType("void", "?")),
        }

        Ok(())
    }

    /// Surrenders ownership of a Rust value to Java.
    ///
    /// This requires an object with a `long` field to store the pointer.
    ///
    /// The Rust value will be implicitly wrapped in a `Box<Mutex<T>>`.
    ///
    /// The Java object will be locked before changing the field value.
    ///
    /// # Safety
    ///
    /// It's important to note that using this API will leak memory if
    /// [`Self::take_rust_field`] is never called so that the Rust type may be
    /// dropped.
    ///
    /// One suggestion that may help ensure that a set Rust field will be
    /// cleaned up later is for the Java object to implement `Closeable` and let
    /// people use a `use` block (Kotlin) or `try-with-resources` (Java).
    ///
    /// **DO NOT** make a copy of the object containing one of these fields
    /// since that will lead to a use-after-free error if the Rust type is
    /// taken and dropped multiple times from Rust.
    #[allow(unused_variables)]
    pub unsafe fn set_rust_field<O, S, T>(&self, obj: O, field: S, rust_object: T) -> Result<()>
    where
        O: Into<JObject<'a>>,
        S: AsRef<str>,
        T: Send + 'static,
    {
        let obj = obj.into();
        let class = self.auto_local(self.get_object_class(obj)?);
        let field_id: JFieldID = (&class, &field, "J").lookup(self)?;

        let guard = self.lock_obj(obj)?;

        // Check to see if we've already set this value. If it's not null, that
        // means that we're going to leak memory if it gets overwritten.
        let field_ptr = self
            .get_field_unchecked(obj, field_id, ReturnType::Primitive(Primitive::Long))?
            .j()? as *mut Mutex<T>;
        if !field_ptr.is_null() {
            return Err(Error::FieldAlreadySet(field.as_ref().to_owned()));
        }

        let mbox = Box::new(::std::sync::Mutex::new(rust_object));
        let ptr: *mut Mutex<T> = Box::into_raw(mbox);

        self.set_field_unchecked(obj, field_id, (ptr as crate::sys::jlong).into())
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
    /// Checks for a null pointer, but assumes that the data it points to is valid for T.
    #[allow(unused_variables)]
    pub unsafe fn get_rust_field<O, S, T>(&self, obj: O, field: S) -> Result<MutexGuard<T>>
    where
        O: Into<JObject<'a>>,
        S: Into<JNIString>,
        T: Send + 'static,
    {
        let obj = obj.into();
        let guard = self.lock_obj(obj)?;

        let ptr = self.get_field(obj, field, "J")?.j()? as *mut Mutex<T>;
        non_null!(ptr, "rust value from Java");
        // dereferencing is safe, because we checked it for null
        Ok((*ptr).lock().unwrap())
    }

    /// Take a Rust field back from Java.
    ///
    /// It sets the field to a null pointer to signal that it's empty.
    ///
    /// The Java object will be locked before taking the field value.
    ///
    /// # Safety
    ///
    /// This will make sure that the pointer is non-null, but still assumes that
    /// the data it points to is valid for T.
    #[allow(unused_variables)]
    pub unsafe fn take_rust_field<O, S, T>(&self, obj: O, field: S) -> Result<T>
    where
        O: Into<JObject<'a>>,
        S: AsRef<str>,
        T: Send + 'static,
    {
        let obj = obj.into();
        let class = self.auto_local(self.get_object_class(obj)?);
        let field_id: JFieldID = (&class, &field, "J").lookup(self)?;

        let mbox = {
            let guard = self.lock_obj(obj)?;

            let ptr = self
                .get_field_unchecked(obj, field_id, ReturnType::Primitive(Primitive::Long))?
                .j()? as *mut Mutex<T>;

            non_null!(ptr, "rust value from Java");

            let mbox = Box::from_raw(ptr);

            // attempt to acquire the lock. This prevents us from consuming the
            // mutex if there's an outstanding lock. No one else will be able to
            // get a new one as long as we're in the guarded scope.
            drop(mbox.try_lock()?);

            self.set_field_unchecked(
                obj,
                field_id,
                (::std::ptr::null_mut::<()>() as sys::jlong).into(),
            )?;

            mbox
        };

        Ok(mbox.into_inner().unwrap())
    }

    /// Lock a Java object. The MonitorGuard that this returns is responsible
    /// for ensuring that it gets unlocked.
    pub fn lock_obj<O>(&self, obj: O) -> Result<MonitorGuard<'a>>
    where
        O: Into<JObject<'a>>,
    {
        let inner = obj.into().into_raw();
        let _ = jni_unchecked!(self.internal, MonitorEnter, inner);

        Ok(MonitorGuard {
            obj: inner,
            env: self.internal,
            life: Default::default(),
        })
    }

    /// Returns underlying `sys::JNIEnv` interface.
    pub fn get_native_interface(&self) -> *mut sys::JNIEnv {
        self.internal
    }

    /// Returns the Java VM interface.
    pub fn get_java_vm(&self) -> Result<JavaVM> {
        let mut raw = ptr::null_mut();
        let res = jni_unchecked!(self.internal, GetJavaVM, &mut raw);
        jni_error_code_to_result(res)?;
        unsafe { JavaVM::from_raw(raw) }
    }

    /// Ensures that at least a given number of local references can be created
    /// in the current thread.
    pub fn ensure_local_capacity(&self, capacity: jint) -> Result<()> {
        jni_void_call!(self.internal, EnsureLocalCapacity, capacity);
        Ok(())
    }

    /// Bind function pointers to native methods of class
    /// according to method name and signature.
    /// For details see [documentation](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#RegisterNatives).
    pub fn register_native_methods<'c, T>(&self, class: T, methods: &[NativeMethod]) -> Result<()>
    where
        T: Desc<'a, JClass<'c>>,
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
        let res = jni_non_void_call!(
            self.internal,
            RegisterNatives,
            class.into_raw(),
            jni_native_methods.as_ptr(),
            jni_native_methods.len() as jint
        );
        jni_error_code_to_result(res)
    }

    /// Unbind all native methods of class.
    pub fn unregister_native_methods<'c, T>(&self, class: T) -> Result<()>
    where
        T: Desc<'a, JClass<'c>>,
    {
        let class = class.lookup(self)?;
        let res = jni_non_void_call!(self.internal, UnregisterNatives, class.into_raw());
        jni_error_code_to_result(res)
    }

    /// Return an AutoArray of the given Java array.
    ///
    /// The result is valid until the AutoArray object goes out of scope, when the
    /// release happens automatically according to the mode parameter.
    ///
    /// Since the returned array may be a copy of the Java array, changes made to the
    /// returned array will not necessarily be reflected in the original array until
    /// the corresponding Release*ArrayElements JNI method is called.
    /// AutoArray has a commit() method, to force a copy of the array if needed (and without
    /// releasing it).

    /// Prefer to use the convenience wrappers:
    /// [`get_int_array_elements`](struct.JNIEnv.html#method.get_int_array_elements)
    /// [`get_long_array_elements`](struct.JNIEnv.html#method.get_long_array_elements)
    /// [`get_byte_array_elements`](struct.JNIEnv.html#method.get_byte_array_elements)
    /// [`get_boolean_array_elements`](struct.JNIEnv.html#method.get_boolean_array_elements)
    /// [`get_char_array_elements`](struct.JNIEnv.html#method.get_char_array_elements)
    /// [`get_short_array_elements`](struct.JNIEnv.html#method.get_short_array_elements)
    /// [`get_float_array_elements`](struct.JNIEnv.html#method.get_float_array_elements)
    /// [`get_double_array_elements`](struct.JNIEnv.html#method.get_double_array_elements)
    /// And the associated [`AutoArray`](struct.objects.AutoArray) struct.
    pub fn get_array_elements<T: TypeArray>(
        &self,
        array: jarray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, T>> {
        non_null!(array, "get_array_elements array argument");
        AutoArray::new(self, unsafe { JObject::from_raw(array) }, mode)
    }

    /// See also [`get_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_int_array_elements(
        &self,
        array: jintArray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, jint>> {
        self.get_array_elements(array, mode)
    }

    /// See also [`get_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_long_array_elements(
        &self,
        array: jlongArray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, jlong>> {
        self.get_array_elements(array, mode)
    }

    /// See also [`get_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_byte_array_elements(
        &self,
        array: jbyteArray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, jbyte>> {
        self.get_array_elements(array, mode)
    }

    /// See also [`get_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_boolean_array_elements(
        &self,
        array: jbooleanArray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, jboolean>> {
        self.get_array_elements(array, mode)
    }

    /// See also [`get_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_char_array_elements(
        &self,
        array: jcharArray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, jchar>> {
        self.get_array_elements(array, mode)
    }

    /// See also [`get_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_short_array_elements(
        &self,
        array: jshortArray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, jshort>> {
        self.get_array_elements(array, mode)
    }

    /// See also [`get_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_float_array_elements(
        &self,
        array: jfloatArray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, jfloat>> {
        self.get_array_elements(array, mode)
    }

    /// See also [`get_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_double_array_elements(
        &self,
        array: jdoubleArray,
        mode: ReleaseMode,
    ) -> Result<AutoArray<'a, jdouble>> {
        self.get_array_elements(array, mode)
    }

    /// Return an AutoPrimitiveArray of the given Java primitive array.
    ///
    /// The result is valid until the corresponding AutoPrimitiveArray object goes out of scope,
    /// when the release happens automatically according to the mode parameter.
    ///
    /// Given that Critical sections must be as short as possible, and that they come with a
    /// number of important restrictions (see GetPrimitiveArrayCritical JNI doc), use this
    /// wrapper wisely, to avoid holding the array longer that strictly necessary.
    /// In any case, you can:
    ///  - Use std::mem::drop explicitly, to force / anticipate resource release.
    ///  - Use a nested scope, to release the array at the nested scope's exit.
    ///
    /// Since the returned array may be a copy of the Java array, changes made to the
    /// returned array will not necessarily be reflected in the original array until
    /// ReleasePrimitiveArrayCritical is called; which happens at AutoPrimitiveArray
    /// destruction.
    ///
    /// If the given array is `null`, an `Error::NullPtr` is returned.
    ///
    /// See also [`get_byte_array_elements`](struct.JNIEnv.html#method.get_array_elements)
    pub fn get_primitive_array_critical(
        &self,
        array: jarray,
        mode: ReleaseMode,
    ) -> Result<AutoPrimitiveArray> {
        non_null!(array, "get_primitive_array_critical array argument");
        let mut is_copy: jboolean = 0xff;
        // Even though this method may throw OoME, use `jni_unchecked`
        // instead of `jni_non_null_call` to remove (a slight) overhead
        // of exception checking. An error will still be detected as a `null`
        // result inside AutoPrimitiveArray ctor; and, as this method is unlikely
        // to create a copy, an OoME is highly unlikely.
        let ptr = jni_unchecked!(
            self.internal,
            GetPrimitiveArrayCritical,
            array,
            &mut is_copy
        );
        AutoPrimitiveArray::new(
            self,
            unsafe { JObject::from_raw(array) },
            ptr,
            mode,
            is_copy == sys::JNI_TRUE,
        )
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
pub struct MonitorGuard<'a> {
    obj: sys::jobject,
    env: *mut sys::JNIEnv,
    life: PhantomData<&'a ()>,
}

impl<'a> Drop for MonitorGuard<'a> {
    fn drop(&mut self) {
        let res: Result<()> = catch!({
            jni_unchecked!(self.env, MonitorExit, self.obj);
            Ok(())
        });

        if let Err(e) = res {
            warn!("error releasing java monitor: {}", e)
        }
    }
}
