use std::str;

use std::marker::PhantomData;

use std::iter::IntoIterator;

use std::slice;

use std::sync::Mutex;
use std::sync::MutexGuard;

use errors::*;

use std::os::raw::{
    c_char,
    c_void,
};
use std::ptr;
use sys::{
    self,
    jarray,
    jboolean,
    jbyte,
    jchar,
    jdouble,
    jfloat,
    jint,
    jlong,
    jshort,
    jsize,
    jvalue,
    jbooleanArray,
    jbyteArray,
    jcharArray,
    jdoubleArray,
    jfloatArray,
    jintArray,
    jlongArray,
    jobjectArray,
    jshortArray,
};

use strings::JNIString;
use strings::JavaStr;

use objects::AutoLocal;
use objects::GlobalRef;
use objects::JByteBuffer;
use objects::JClass;
use objects::JFieldID;
use objects::JList;
use objects::JMap;
use objects::JMethodID;
use objects::JObject;
use objects::JStaticFieldID;
use objects::JStaticMethodID;
use objects::JString;
use objects::JThrowable;
use objects::JValue;

use descriptors::Desc;

use signature::JavaType;
use signature::Primitive;
use signature::TypeSignature;

use JNIVersion;
use JavaVM;

/// FFI-compatible JNIEnv struct. You can safely use this as the JNIEnv argument
/// to exported methods that will be called by java. This is where most of the
/// magic happens. All methods on this object are wrappers around JNI functions,
/// so the documentation on their behavior is still pretty applicable.
///
/// Since we're calling into the JVM with this, many methods also have the
/// potential to cause an exception to get thrown. If this is the case, an `Err`
/// result will be returned with the error kind `JavaException`. Note that this
/// will _not_ clear the exception - it's up to the caller to decide whether to
/// do so or to let it continue being thrown.
///
/// Because null pointers are a thing in Java, this also converts them to an
/// `Err` result with the kind `NullPtr`. This may occur when either a null
/// argument is passed to a method or when a null would be returned. Where
/// applicable, the null error is changed to a more applicable error type, such
/// as `MethodNotFound`.
#[repr(C)]
pub struct JNIEnv<'a> {
    internal: *mut sys::JNIEnv,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> JNIEnv<'a> {
    /// Create a JNIEnv from a raw pointer.
    ///
    /// Only does a null check - otherwise assumes that the pointer is valid.
    pub unsafe fn from_raw(ptr: *mut sys::JNIEnv) -> Result<Self> {
        non_null!(ptr, "from_raw ptr argument");
        Ok(JNIEnv {
            internal: ptr,
            lifetime: PhantomData,
        })
    }

    /// Get the java version that we're being executed from.
    pub fn get_version(&self) -> Result<JNIVersion> {
        Ok(unsafe { jni_unchecked!(self.internal, GetVersion) }.into())
    }

    /// Define a new java class. See the JNI docs for more details - I've never
    /// had occasion to use this and haven't researched it fully.
    pub fn define_class<S>(&self, name: S, loader: JObject, buf: &[u8]) -> Result<JClass>
    where
        S: Into<JNIString>,
    {
        non_null!(loader, "define_class loader argument");
        let name = name.into();
        let class = jni_non_null_call!(
            self.internal,
            DefineClass,
            name.as_ptr(),
            loader.into_inner(),
            buf.as_ptr() as *const jbyte,
            buf.len() as jsize
        );
        Ok(class)
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
        Ok(class)
    }

    /// Get the superclass for a particular class. As with `find_class`, takes
    /// a descriptor.
    pub fn get_superclass<T>(&self, class: T) -> Result<JClass>
    where
        T: Desc<'a, JClass<'a>>,
    {
        let class = class.lookup(self)?;
        Ok(jni_non_null_call!(self.internal, GetSuperclass, class.into_inner()))
    }

    /// Tests whether class1 is assignable from class2.
    pub fn is_assignable_from<T, U>(&self, class1: T, class2: U) -> Result<bool>
    where
        T: Desc<'a, JClass<'a>>,
        U: Desc<'a, JClass<'a>>,
    {
        let class1 = class1.lookup(self)?;
        let class2 = class2.lookup(self)?;
        Ok(unsafe {
            jni_unchecked!(
                self.internal,
                IsAssignableFrom,
                class1.into_inner(),
                class2.into_inner()
            )
        } == sys::JNI_TRUE)
    }

    /// Returns true if the object reference can be cast to the given type.
    ///
    /// _NB: Unlike the operator `instanceof`, function `IsInstanceOf` *returns `true`*
    /// for all classes *if `object` is `null`.*_
    ///
    /// See [JNI documentation](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#IsInstanceOf)
    /// for details.
    pub fn is_instance_of<T>(&self, object: JObject<'a>, class: T) -> Result<bool>
    where
        T: Desc<'a, JClass<'a>>,
    {
        let class = class.lookup(self)?;
        Ok(
            unsafe {
                jni_unchecked!(
                    self.internal,
                    IsInstanceOf,
                    object.into_inner(),
                    class.into_inner()
                )
            } == sys::JNI_TRUE,
        )
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
    pub fn throw<E>(&self, obj: E) -> Result<()>
    where
        E: Desc<'a, JThrowable<'a>>,
    {
        let throwable = obj.lookup(self)?;
        let res: i32 = unsafe { jni_unchecked!(self.internal, Throw, throwable.into_inner()) };
        if res < 0 {
            Err(format!("throw failed with code {}", res).into())
        } else {
            Ok(())
        }
    }

    /// Create and throw a new exception from a class descriptor and an error
    /// message.
    ///
    /// # Example
    /// ```rust,ignore
    /// let _ = env.throw_new("java/lang/Exception", "something bad happened");
    /// ```
    pub fn throw_new<S, T>(&self, class: T, msg: S) -> Result<()>
    where
        S: Into<JNIString>,
        T: Desc<'a, JClass<'a>>,
    {
        self.throw((class, msg))
    }

    /// Check whether or not an exception is currently in the process of being
    /// thrown. An exception is in this state from the time it gets thrown and
    /// not caught in a java function until `exception_clear` is called.
    pub fn exception_occurred(&self) -> Result<JThrowable> {
        let throwable = unsafe { jni_unchecked!(self.internal, ExceptionOccurred) };
        Ok(JThrowable::from(throwable))
    }

    /// Print exception information to the console.
    pub fn exception_describe(&self) -> Result<()> {
        unsafe { jni_unchecked!(self.internal, ExceptionDescribe) };
        Ok(())
    }

    /// Clear an exception in the process of being thrown. If this is never
    /// called, the exception will continue being thrown when control is
    /// returned to java.
    pub fn exception_clear(&self) -> Result<()> {
        unsafe { jni_unchecked!(self.internal, ExceptionClear) };
        Ok(())
    }

    /// Abort the JVM with an error message.
    #[allow(unused_variables, unreachable_code)]
    pub fn fatal_error<S: Into<JNIString>>(&self, msg: S) -> ! {
        let msg = msg.into();
        let res: Result<()> = catch!({
            unsafe { jni_unchecked!(self.internal, FatalError, msg.as_ptr()) }
            unreachable!()
        });

        panic!(res.unwrap_err());
    }

    /// Check to see if an exception is being thrown. This only differs from
    /// `exception_occurred` in that it doesn't return the actual thrown
    /// exception.
    pub fn exception_check(&self) -> Result<bool> {
        let check = unsafe { jni_unchecked!(self.internal, ExceptionCheck) } == sys::JNI_TRUE;
        Ok(check)
    }

    /// Create a new instance of a direct java.nio.ByteBuffer.
    pub fn new_direct_byte_buffer(&self, data: &mut [u8]) -> Result<JByteBuffer> {
        let obj = unsafe {
            jni_unchecked!(
                self.internal,
                NewDirectByteBuffer,
                data.as_mut_ptr() as *mut c_void,
                data.len() as jlong
            )
        };
        Ok(JByteBuffer::from(obj))
    }

    /// Returns the starting address of the memory of the direct
    /// java.nio.ByteBuffer.
    pub fn get_direct_buffer_address(&self, buf: JByteBuffer) -> Result<&mut [u8]> {
        let ptr: *mut c_void =
            unsafe { jni_unchecked!(self.internal, GetDirectBufferAddress, buf.into_inner()) };
        let capacity = self.get_direct_buffer_capacity(buf)?;
        unsafe { Ok(slice::from_raw_parts_mut(ptr as *mut u8, capacity as usize)) }
    }

    /// Returns the capacity of the direct java.nio.ByteBuffer.
    pub fn get_direct_buffer_capacity(&self, buf: JByteBuffer) -> Result<jlong> {
        let capacity =
            unsafe { jni_unchecked!(self.internal, GetDirectBufferCapacity, buf.into_inner()) };
        Ok(capacity)
    }

    /// Turns an object into a global ref. This has the benefit of removing the
    /// lifetime bounds since it's guaranteed to not get GC'd by java. It
    /// releases the GC pin upon being dropped.
    pub fn new_global_ref(&self, obj: JObject) -> Result<GlobalRef> {
        non_null!(obj, "new_global_ref obj argument");
        let new_ref: JObject = jni_non_null_call!(self.internal, NewGlobalRef, obj.into_inner());
        let global = unsafe { GlobalRef::from_raw(self.get_java_vm()?, new_ref.into_inner()) };
        Ok(global)
    }

    /// Create a new local ref to an object.
    ///
    /// Note that the object passed to this is *already* a local ref. This
    /// creates yet another reference to it, which is most likely not what you
    /// want.
    pub fn new_local_ref<T>(&self, obj: JObject) -> Result<JObject> {
        non_null!(obj, "new_local_ref obj argument");
        let local: JObject = jni_non_null_call!(self.internal, NewLocalRef, obj.into_inner());
        Ok(local)
    }

    /// Creates a new auto-deleted local reference.
    ///
    /// See also `with_local_frame` method that can be more convenient
    /// when you create a bounded number of local references in a method but
    /// can't rely on automatic de-allocation (e.g., in case of recursion
    /// or just deep call stacks).
    pub fn auto_local(&'a self, obj: JObject<'a>) -> AutoLocal<'a> {
        AutoLocal::new(self, obj)
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
        non_null!(obj, "delete_local_ref obj argument");
        Ok(unsafe {
            jni_unchecked!(self.internal, DeleteLocalRef, obj.into_inner());
        })
    }

    /// Creates a new local reference frame, in which at least a given number
    /// of local
    /// references can be created.
    ///
    /// Returns `Err` on failure, with a pending `OutOfMemoryError`.
    ///
    /// Prefer to use `with_local_frame` instead of direct `push_local_frame`/
    /// `pop_local_frame` calls.
    ///
    /// See also `auto_local` method and `AutoLocal` type - that approach can
    /// be more
    /// convenient in loops.
    pub fn push_local_frame(&self, capacity: i32) -> Result<()> {
        // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
        let res = unsafe {
            jni_unchecked!(self.internal, PushLocalFrame, capacity)
        };
        jni_error_code_to_result(res)
    }

    /// Pops off the current local reference frame, frees all the local
    /// references allocated
    /// on the current stack frame.
    ///
    /// Note that resulting `JObject` can be `NULL` if `result` is `NULL`.
    pub fn pop_local_frame(&self, result: JObject) -> Result<JObject> {
        // This method is safe to call in case of pending exceptions (see chapter 2 of the spec)
        Ok(unsafe { jni_unchecked!(self.internal, PopLocalFrame, result.into_inner()).into() })
    }

    /// Provides a convenient way to use `push_local_frame` by automatically
    /// calling
    /// `pop_local_frame` function.
    pub fn with_local_frame<F>(&self, capacity: i32, f: F) -> Result<JObject>
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
    pub fn alloc_object<T>(&self, class: T) -> Result<JObject>
    where
        T: Desc<'a, JClass<'a>>,
    {
        let class = class.lookup(self)?;
        Ok(jni_non_null_call!(self.internal, AllocObject, class.into_inner()))
    }

    /// Common functionality for finding methods.
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
            Err(e) => match e.kind() {
                &ErrorKind::NullPtr(_) => {
                    let name: String = ffi_name.into();
                    let sig: String = sig.into();
                    return Err(ErrorKind::MethodNotFound(name, sig).into());
                }
                _ => return Err(e),
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
    pub fn get_method_id<'c, T, U, V>(&self, class: T, name: U, sig: V) -> Result<JMethodID<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        self.get_method_id_base(class, name, sig, |class, name, sig| {
            Ok(jni_non_null_call!(
                self.internal,
                GetMethodID,
                class.into_inner(),
                name.as_ptr(),
                sig.as_ptr()
            ))
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
    ) -> Result<JStaticMethodID<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        self.get_method_id_base(class, name, sig, |class, name, sig| {
            Ok(jni_non_null_call!(
                self.internal,
                GetStaticMethodID,
                class.into_inner(),
                name.as_ptr(),
                sig.as_ptr()
            ))
        })
    }

    /// Look up the field ID for a class/name/type combination.
    ///
    /// # Example
    /// ```rust,ignore
    /// let field_id = env.get_field_id("com/my/Class", "intField", "I");
    /// ```
    pub fn get_field_id<'c, T, U, V>(&self, class: T, name: U, sig: V) -> Result<JFieldID<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.into();
        let ffi_sig = sig.into();

        let res: Result<JFieldID> = catch!({
            Ok(jni_non_null_call!(
                self.internal,
                GetFieldID,
                class.into_inner(),
                ffi_name.as_ptr(),
                ffi_sig.as_ptr()
            ))
        });

        match res {
            Ok(m) => Ok(m),
            Err(e) => match e.kind() {
                &ErrorKind::NullPtr(_) => {
                    let name: String = ffi_name.into();
                    let sig: String = ffi_sig.into();
                    return Err(ErrorKind::FieldNotFound(name, sig).into());
                }
                _ => return Err(e),
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
    ) -> Result<JStaticFieldID<'a>>
    where
        T: Desc<'a, JClass<'c>>,
        U: Into<JNIString>,
        V: Into<JNIString>,
    {
        let class = class.lookup(self)?;
        let ffi_name = name.into();
        let ffi_sig = sig.into();

        let res: Result<JStaticFieldID> = catch!({
            Ok(jni_non_null_call!(
                self.internal,
                GetStaticFieldID,
                class.into_inner(),
                ffi_name.as_ptr(),
                ffi_sig.as_ptr()
            ))
        });

        match res {
            Ok(m) => Ok(m),
            Err(e) => match e.kind() {
                &ErrorKind::NullPtr(_) => {
                    let name: String = ffi_name.into();
                    let sig: String = ffi_sig.into();
                    return Err(ErrorKind::FieldNotFound(name, sig).into());
                }
                _ => return Err(e),
            },
        }
    }

    /// Get the class for an object.
    pub fn get_object_class(&self, obj: JObject) -> Result<JClass<'a>> {
        Ok(jni_non_null_call!(self.internal, GetObjectClass, obj.into_inner()))
    }

    /// Call a static method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the class, whether the return
    /// type is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `CallStatic<Type>MethodA` method
    /// with the provided arguments.
    #[allow(unused_unsafe)]
    pub unsafe fn call_static_method_unsafe<T, U>(
        &self,
        class: T,
        method_id: U,
        ret: JavaType,
        args: &[JValue],
    ) -> Result<JValue>
    where
        T: Desc<'a, JClass<'a>>,
        U: Desc<'a, JStaticMethodID<'a>>,
    {
        let class = class.lookup(self)?;

        let method_id = method_id.lookup(self)?.into_inner();

        let class = class.into_inner();
        let args: Vec<jvalue> = args.into_iter().map(|v| v.to_jni()).collect();
        let jni_args = args.as_ptr();

        // TODO clean this up
        Ok(match ret {
            JavaType::Object(_) | JavaType::Array(_) => {
                let obj: JObject = jni_non_null_call!(
                    self.internal,
                    CallStaticObjectMethodA,
                    class,
                    method_id,
                    jni_args
                );
                obj.into()
            }
            // JavaType::Object
            JavaType::Method(_) => unimplemented!(),
            JavaType::Primitive(p) => {
                let v: JValue = match p {
                    Primitive::Boolean => (jni_unchecked!(
                        self.internal,
                        CallStaticBooleanMethodA,
                        class,
                        method_id,
                        jni_args
                    ) == sys::JNI_TRUE)
                        .into(),
                    Primitive::Char => jni_unchecked!(
                        self.internal,
                        CallStaticCharMethodA,
                        class,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Short => jni_unchecked!(
                        self.internal,
                        CallStaticShortMethodA,
                        class,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Int => jni_unchecked!(
                        self.internal,
                        CallStaticIntMethodA,
                        class,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Long => jni_unchecked!(
                        self.internal,
                        CallStaticLongMethodA,
                        class,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Float => jni_unchecked!(
                        self.internal,
                        CallStaticFloatMethodA,
                        class,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Double => jni_unchecked!(
                        self.internal,
                        CallStaticDoubleMethodA,
                        class,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Byte => jni_unchecked!(
                        self.internal,
                        CallStaticByteMethodA,
                        class,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Void => jni_unchecked!(
                        self.internal,
                        CallStaticVoidMethodA,
                        class,
                        method_id,
                        jni_args
                    ).into(),
                };
                v.into()
            } // JavaType::Primitive
        }) // match parsed.ret
    }

    /// Call an object method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the object, whether the return
    /// type is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `Call<Type>MethodA` method with
    /// the provided arguments.
    #[allow(unused_unsafe)]
    pub unsafe fn call_method_unsafe<T>(
        &self,
        obj: JObject,
        method_id: T,
        ret: JavaType,
        args: &[JValue],
    ) -> Result<JValue>
    where
        T: Desc<'a, JMethodID<'a>>,
    {
        let method_id = method_id.lookup(self)?.into_inner();

        let obj = obj.into_inner();

        let args: Vec<jvalue> = args.into_iter().map(|v| v.to_jni()).collect();
        let jni_args = args.as_ptr();

        // TODO clean this up
        Ok(match ret {
            JavaType::Object(_) | JavaType::Array(_) => {
                let obj: JObject =
                    jni_non_void_call!(self.internal, CallObjectMethodA, obj, method_id, jni_args)
                        .into();
                obj.into()
            }
            // JavaType::Object
            JavaType::Method(_) => unimplemented!(),
            JavaType::Primitive(p) => {
                let v: JValue = match p {
                    Primitive::Boolean => (jni_non_void_call!(
                        self.internal,
                        CallBooleanMethodA,
                        obj,
                        method_id,
                        jni_args
                    ) == sys::JNI_TRUE)
                        .into(),
                    Primitive::Char => {
                        jni_non_void_call!(self.internal, CallCharMethodA, obj, method_id, jni_args)
                            .into()
                    }
                    Primitive::Short => jni_non_void_call!(
                        self.internal,
                        CallShortMethodA,
                        obj,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Int => {
                        jni_non_void_call!(self.internal, CallIntMethodA, obj, method_id, jni_args)
                            .into()
                    }
                    Primitive::Long => {
                        jni_non_void_call!(self.internal, CallLongMethodA, obj, method_id, jni_args)
                            .into()
                    }
                    Primitive::Float => jni_non_void_call!(
                        self.internal,
                        CallFloatMethodA,
                        obj,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Double => jni_non_void_call!(
                        self.internal,
                        CallDoubleMethodA,
                        obj,
                        method_id,
                        jni_args
                    ).into(),
                    Primitive::Byte => {
                        jni_non_void_call!(self.internal, CallByteMethodA, obj, method_id, jni_args)
                            .into()
                    }
                    Primitive::Void => {
                        jni_void_call!(self.internal, CallVoidMethodA, obj, method_id, jni_args);
                        return Ok(JValue::Void);
                    }
                };
                v.into()
            } // JavaType::Primitive
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
    /// * Calls `call_method_unsafe` with the verified safe arguments.
    ///
    /// Note: this may cause a java exception if the arguments are the wrong
    /// type, in addition to if the method itself throws.
    pub fn call_method<S, T>(
        &'a self,
        obj: JObject,
        name: S,
        sig: T,
        args: &[JValue],
    ) -> Result<JValue>
    where
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        non_null!(obj, "call_method obj argument");

        // parse the signature
        let parsed = TypeSignature::from_str(sig.as_ref())?;
        if parsed.args.len() != args.len() {
            return Err(ErrorKind::InvalidArgList.into());
        }

        let class = self.auto_local(self.get_object_class(obj)?.into());

        let res = unsafe { self.call_method_unsafe(obj, (&class, name, sig), parsed.ret, args) };

        res
    }

    /// Calls a static method safely. This comes with a number of
    /// lookups/checks. It
    ///
    /// * Parses the type signature to find the number of arguments and return
    ///   type
    /// * Looks up the JMethodID for the class/name/signature combination
    /// * Ensures that the number of args matches the signature
    /// * Calls `call_method_unsafe` with the verified safe arguments.
    ///
    /// Note: this may cause a java exception if the arguments are the wrong
    /// type, in addition to if the method itself throws.
    pub fn call_static_method<T, U, V>(
        &self,
        class: T,
        name: U,
        sig: V,
        args: &[JValue],
    ) -> Result<JValue>
    where
        T: Desc<'a, JClass<'a>>,
        U: Into<JNIString>,
        V: Into<JNIString> + AsRef<str>,
    {
        let parsed = TypeSignature::from_str(&sig)?;
        if parsed.args.len() != args.len() {
            return Err(ErrorKind::InvalidArgList.into());
        }

        // go ahead and look up the class since it's already Copy,
        // and we'll need that for the next call.
        let class = class.lookup(self)?;

        unsafe { self.call_static_method_unsafe(class, (class, name, sig), parsed.ret, args) }
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
            return Err(ErrorKind::InvalidArgList.into());
        }

        if parsed.ret != JavaType::Primitive(Primitive::Void) {
            return Err(ErrorKind::InvalidCtorReturn.into());
        }

        // build strings
        let class = class.lookup(self)?;

        let method_id: JMethodID = (class, ctor_sig).lookup(self)?;

        self.new_object_by_id(class, method_id, ctor_args)
    }

    /// Create a new object using a constructor. Arguments aren't checked
    /// because
    /// of the `JMethodID` usage.
    pub fn new_object_by_id<'c, T>(
        &self,
        class: T,
        ctor_id: JMethodID,
        ctor_args: &[JValue],
    ) -> Result<JObject<'a>>
    where
        T: Desc<'a, JClass<'c>>,
    {
        let class = class.lookup(self)?;

        let jni_args: Vec<jvalue> = ctor_args.into_iter().map(|v| v.to_jni()).collect();
        let jni_args = jni_args.as_ptr();

        Ok(jni_non_null_call!(
            self.internal,
            NewObjectA,
            class.into_inner(),
            ctor_id.into_inner(),
            jni_args
        ))
    }

    /// Cast a JObject to a JString. This won't throw exceptions or return errors
    /// in the event that the object isn't actually a list, but the methods on
    /// the resulting map object will.
    pub fn get_list(&self, obj: JObject<'a>) -> Result<JList> {
        non_null!(obj, "get_list obj argument");
        JList::from_env(self, obj)
    }

    /// Cast a JObject to a JMap. This won't throw exceptions or return errors
    /// in the event that the object isn't actually a map, but the methods on
    /// the resulting map object will.
    pub fn get_map(&self, obj: JObject<'a>) -> Result<JMap> {
        non_null!(obj, "get_map obj argument");
        JMap::from_env(self, obj)
    }

    /// Get a JavaStr from a JString. This allows conversions from java string
    /// objects to rust strings.
    ///
    /// This entails a call to `GetStringUTFChars` and only decodes java's
    /// modified UTF-8 format on conversion to a rust-compatible string.
    pub fn get_string(&self, obj: JString<'a>) -> Result<JavaStr> {
        non_null!(obj, "get_string obj argument");
        JavaStr::from_env(self, obj)
    }

    /// Get a pointer to the character array beneath a JString. This is in
    /// Java's modified UTF-8 and will leak memory if `release_string_utf_chars`
    /// is never called.
    #[allow(unused_unsafe)]
    pub unsafe fn get_string_utf_chars(&self, obj: JString) -> Result<*const c_char> {
        non_null!(obj, "get_string_utf_chars obj argument");
        let ptr: *const c_char = jni_non_null_call!(
            self.internal,
            GetStringUTFChars,
            obj.into_inner(),
            ::std::ptr::null::<jboolean>() as *mut jboolean
        );
        Ok(ptr)
    }

    /// Unpin the array returned by `get_string_utf_chars`.
    #[allow(unused_unsafe)]
    pub unsafe fn release_string_utf_chars(&self, obj: JString, arr: *const c_char) -> Result<()> {
        non_null!(obj, "release_string_utf_chars obj argument");
        // This method is safe to call in case of pending exceptions (see the chapter 2 of the spec)
        jni_unchecked!(self.internal, ReleaseStringUTFChars, obj.into_inner(), arr);
        Ok(())
    }

    /// Create a new java string object from a rust string. This requires a
    /// re-encoding of rusts *real* UTF-8 strings to java's modified UTF-8
    /// format.
    pub fn new_string<S: Into<JNIString>>(&self, from: S) -> Result<JString<'a>> {
        let ffi_str = from.into();
        Ok(jni_non_null_call!(self.internal, NewStringUTF, ffi_str.as_ptr()))
    }

    /// Get the length of a java array
    pub fn get_array_length(&self, array: jarray) -> Result<jsize> {
        non_null!(array, "get_array_length array argument");
        let len: jsize = unsafe { jni_unchecked!(self.internal, GetArrayLength, array) };
        Ok(len)
    }

    /// Construct a new array holding objects in class `element_class`.
    /// All elements are initially set to `initial_element`.
    ///
    /// This function returns a local reference, that must not be allocated
    /// excessively.
    /// See [Java documentation][1] for details.
    /// [1]: https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/design.html#global_and_local_references
    pub fn new_object_array<T>(
        &self,
        length: jsize,
        element_class: T,
        initial_element: JObject,
    ) -> Result<jobjectArray>
    where
        T: Desc<'a, JClass<'a>>,
    {
        let class = element_class.lookup(self)?;
        Ok(jni_non_null_call!(
            self.internal,
            NewObjectArray,
            length,
            class.into_inner(),
            initial_element.into_inner()
        ))
    }

    /// Returns an element of the `jobjectArray` array.
    pub fn get_object_array_element(&self, array: jobjectArray, index: jsize) -> Result<JObject> {
        non_null!(array, "get_object_array_element array argument");
        Ok(jni_non_null_call!(
            self.internal,
            GetObjectArrayElement,
            array,
            index
        ))
    }

    /// Sets an element of the `jobjectArray` array.
    pub fn set_object_array_element(
        &self,
        array: jobjectArray,
        index: jsize,
        value: JObject,
    ) -> Result<()> {
        non_null!(array, "set_object_array_element array argument");
        Ok(jni_void_call!(
            self.internal,
            SetObjectArrayElement,
            array,
            index,
            value.into_inner()
        ))
    }

    /// Create a new java byte array from a rust byte slice.
    pub fn byte_array_from_slice(&self, buf: &[u8]) -> Result<jbyteArray> {
        let length = buf.len() as i32;
        let bytes: jbyteArray = self.new_byte_array(length)?;
        unsafe {
            jni_unchecked!(
                self.internal,
                SetByteArrayRegion,
                bytes,
                0,
                length,
                buf.as_ptr() as *const i8
            );
        }
        Ok(bytes)
    }

    /// Converts a java byte array to a rust vector of bytes.
    pub fn convert_byte_array(&self, array: jbyteArray) -> Result<Vec<u8>> {
        non_null!(array, "convert_byte_array array argument");
        let length = jni_non_void_call!(self.internal, GetArrayLength, array);
        let mut vec = vec![0u8; length as usize];
        unsafe {
            jni_unchecked!(
                self.internal,
                GetByteArrayRegion,
                array,
                0,
                length,
                vec.as_mut_ptr() as *mut i8
            );
        }
        check_exception!(self.internal);
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
    /// `buf` slice.
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
    /// slice.
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
    /// `buf` slice.
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
    /// `buf` slice.
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
    /// `buf` slice.
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
    /// `buf` slice.
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
    /// `buf` slice.
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
    /// `buf` slice.
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
    #[allow(unused_unsafe)]
    pub unsafe fn get_field_unsafe<T>(&self, obj: JObject, field: T, ty: JavaType) -> Result<JValue>
    where
        T: Desc<'a, JFieldID<'a>>,
    {
        non_null!(obj, "get_field_typed obj argument");

        let field = field.lookup(self)?.into_inner();
        let obj = obj.into_inner();

        // TODO clean this up
        Ok(match ty {
            JavaType::Object(_) | JavaType::Array(_) => {
                let obj: JObject = jni_non_null_call!(self.internal, GetObjectField, obj, field);
                obj.into()
            }
            // JavaType::Object
            JavaType::Method(_) => unimplemented!(),
            JavaType::Primitive(p) => {
                let v: JValue = match p {
                    Primitive::Boolean => {
                        (jni_unchecked!(self.internal, GetBooleanField, obj, field)
                            == sys::JNI_TRUE)
                            .into()
                    }
                    Primitive::Char => {
                        jni_unchecked!(self.internal, GetCharField, obj, field).into()
                    }
                    Primitive::Short => {
                        jni_unchecked!(self.internal, GetShortField, obj, field).into()
                    }
                    Primitive::Int => jni_unchecked!(self.internal, GetIntField, obj, field).into(),
                    Primitive::Long => {
                        jni_unchecked!(self.internal, GetLongField, obj, field).into()
                    }
                    Primitive::Float => {
                        jni_unchecked!(self.internal, GetFloatField, obj, field).into()
                    }
                    Primitive::Double => {
                        jni_unchecked!(self.internal, GetDoubleField, obj, field).into()
                    }
                    Primitive::Byte => {
                        jni_unchecked!(self.internal, GetByteField, obj, field).into()
                    }
                    Primitive::Void => {
                        return Err(ErrorKind::WrongJValueType("void", "see java field").into());
                    }
                };
                v.into()
            } // JavaType::Primitive
        }) // match parsed.ret
    }

    /// Set a field without any type checking.
    pub unsafe fn set_field_unsafe<T>(&self, obj: JObject, field: T, val: JValue) -> Result<()>
    where
        T: Desc<'a, JFieldID<'a>>,
    {
        non_null!(obj, "set_field_typed obj argument");

        let field = field.lookup(self)?.into_inner();
        let obj = obj.into_inner();

        // TODO clean this up
        match val {
            JValue::Object(o) => {
                jni_unchecked!(self.internal, SetObjectField, obj, field, o.into_inner());
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
                return Err(ErrorKind::WrongJValueType("void", "see java field").into());
            }
        };

        Ok(())
    }

    /// Get a field. Requires an object class lookup and a field id lookup
    /// internally.
    pub fn get_field<S, T>(&self, obj: JObject, name: S, ty: T) -> Result<JValue>
    where
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let class = self.auto_local(self.get_object_class(obj)?.into());

        let parsed = JavaType::from_str(ty.as_ref())?;

        let field_id: JFieldID = (&class, name, ty).lookup(self)?;

        unsafe { self.get_field_unsafe(obj, field_id, parsed) }
    }

    /// Set a field. Does the same lookups as `get_field` and ensures that the
    /// type matches the given value.
    pub fn set_field<S, T>(&self, obj: JObject, name: S, ty: T, val: JValue) -> Result<()>
    where
        S: Into<JNIString>,
        T: Into<JNIString> + AsRef<str>,
    {
        let parsed = JavaType::from_str(ty.as_ref())?;
        let in_type = val.primitive_type();

        match parsed {
            JavaType::Object(_) | JavaType::Array(_) => {
                if let None = in_type {
                    // we're good here
                } else {
                    return Err(
                        ErrorKind::WrongJValueType(val.type_name(), "see java field").into(),
                    );
                }
            }
            JavaType::Primitive(p) => {
                if let Some(in_p) = in_type {
                    if in_p == p {
                        // good
                    } else {
                        return Err(
                            ErrorKind::WrongJValueType(val.type_name(), "see java field").into(),
                        );
                    }
                } else {
                    return Err(
                        ErrorKind::WrongJValueType(val.type_name(), "see java field").into(),
                    );
                }
            }
            JavaType::Method(_) => unimplemented!(),
        }

        let class = self.auto_local(self.get_object_class(obj)?.into());

        let res = unsafe { self.set_field_unsafe(obj, (&class, name, ty), val) };

        res
    }

    /// Get a static field without checking the provided type against the actual
    /// field.
    #[allow(unused_unsafe)]
    pub unsafe fn get_static_field_unsafe<T, U>(
        &self,
        class: T,
        field: U,
        ty: JavaType,
    ) -> Result<JValue>
    where
        T: Desc<'a, JClass<'a>>,
        U: Desc<'a, JStaticFieldID<'a>>,
    {
        let class = class.lookup(self)?.into_inner();

        let field_id = field.lookup(self)?.into_inner();

        // TODO clean this up
        Ok(match ty {
            JavaType::Object(_) | JavaType::Array(_) => {
                let obj: JObject = jni_non_null_call!(self.internal, GetStaticObjectField, class, field_id);
                obj.into()
            }
            // JavaType::Object
            JavaType::Method(_) => {
                return Err(ErrorKind::WrongJValueType("Method", "see java field").into())
            }
            JavaType::Primitive(p) => {
                let v: JValue = match p {
                    Primitive::Boolean => {
                        (jni_unchecked!(self.internal, GetStaticBooleanField, class, field_id)
                            == sys::JNI_TRUE)
                            .into()
                    }
                    Primitive::Char => {
                        jni_unchecked!(self.internal, GetStaticCharField, class, field_id).into()
                    }
                    Primitive::Short => {
                        jni_unchecked!(self.internal, GetStaticShortField, class, field_id).into()
                    }
                    Primitive::Int => {
                        jni_unchecked!(self.internal, GetStaticIntField, class, field_id).into()
                    }
                    Primitive::Long => {
                        jni_unchecked!(self.internal, GetStaticLongField, class, field_id).into()
                    }
                    Primitive::Float => {
                        jni_unchecked!(self.internal, GetStaticFloatField, class, field_id).into()
                    }
                    Primitive::Double => {
                        jni_unchecked!(self.internal, GetStaticDoubleField, class, field_id).into()
                    }
                    Primitive::Byte => {
                        jni_unchecked!(self.internal, GetStaticByteField, class, field_id).into()
                    }
                    Primitive::Void => {
                        return Err(ErrorKind::WrongJValueType("void", "see java field").into());
                    }
                };
                v.into()
            } // JavaType::Primitive
        }) // match ty
    }

    /// Get a static field. Requires a class lookup and a field id lookup
    /// internally.
    pub fn get_static_field<T, U, V>(&self, class: T, field: U, sig: V) -> Result<JValue>
    where
        T: Desc<'a, JClass<'a>>,
        U: Into<JNIString>,
        V: Into<JNIString> + AsRef<str>,
    {
        let ty = JavaType::from_str(sig.as_ref())?;

        // go ahead and look up the class since it's already Copy,
        // and we'll need that for the next call.
        let class = class.lookup(self)?;

        unsafe { self.get_static_field_unsafe(class, (class, field, sig), ty) }
    }

    /// Surrenders ownership of a rust object to Java. Requires an object with a
    /// `long` field to store the pointer. The Rust value will be wrapped in a
    /// Mutex since Java will be controlling where it'll be used thread-wise.
    /// Unsafe because it leaks memory if `take_rust_field` is never called (so
    /// be sure to make a finalizer).
    ///
    /// **DO NOT** make a copy of the object containing one of these fields. If
    /// you've set up a finalizer to pass it back to Rust upon being GC'd, it
    /// will point to invalid memory and will likely attempt to be deallocated
    /// again.
    #[allow(unused_variables)]
    pub unsafe fn set_rust_field<S, T>(&self, obj: JObject, field: S, rust_object: T) -> Result<()>
    where
        S: AsRef<str>,
        T: Send + 'static,
    {
        let class = self.auto_local(self.get_object_class(obj)?.into());
        let field_id: JFieldID = (&class, &field, "J").lookup(self)?;

        let guard = self.lock_obj(obj)?;

        // Check to see if we've already set this value. If it's not null, that
        // means that we're going to leak memory if it gets overwritten.
        let field_ptr = self.get_field_unsafe(obj, field_id, JavaType::Primitive(Primitive::Long))?
            .j()? as *mut Mutex<T>;
        if !field_ptr.is_null() {
            return Err(format!("field already set: {}", field.as_ref()).into());
        }

        let mbox = Box::new(::std::sync::Mutex::new(rust_object));
        let ptr: *mut Mutex<T> = Box::into_raw(mbox);

        self.set_field_unsafe(obj, field_id, (ptr as ::sys::jlong).into())
    }

    /// Gets a lock on a Rust value that's been given to a Java object. Java
    /// still retains ownership and `take_rust_field` will still need to be
    /// called at some point. Checks for a null pointer, but assumes that the
    /// data it ponts to is valid for T.
    #[allow(unused_variables)]
    pub unsafe fn get_rust_field<S, T>(&self, obj: JObject, field: S) -> Result<MutexGuard<T>>
    where
        S: Into<JNIString>,
        T: Send + 'static,
    {
        let guard = self.lock_obj(obj)?;

        let ptr = self.get_field(obj, field, "J")?.j()? as *mut Mutex<T>;
        non_null!(ptr, "rust value from Java");
        Ok((*ptr).lock().unwrap())
    }

    /// Take a Rust field back from Java. Makes sure that the pointer is
    /// non-null, but still assumes that the data it points to is valid for T.
    /// Sets the field to a null pointer to signal that it's empty.
    ///
    /// This will return an error in the event that there's an outstanding lock
    /// on the object.
    #[allow(unused_variables)]
    pub unsafe fn take_rust_field<S, T>(&self, obj: JObject, field: S) -> Result<T>
    where
        S: AsRef<str>,
        T: Send + 'static,
    {
        let class = self.auto_local(self.get_object_class(obj)?.into());
        let field_id: JFieldID = (&class, &field, "J").lookup(self)?;

        let mbox = {
            let guard = self.lock_obj(obj)?;

            let ptr = self.get_field_unsafe(obj, field_id, JavaType::Primitive(Primitive::Long))?
                .j()? as *mut Mutex<T>;

            non_null!(ptr, "rust value from Java");

            let mbox = Box::from_raw(ptr);

            // attempt to acquire the lock. This prevents us from consuming the
            // mutex if there's an outstanding lock. No one else will be able to
            // get a new one as long as we're in the guarded scope.
            let _ = mbox.try_lock()?;

            self.set_field_unsafe(
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
    pub fn lock_obj(&self, obj: JObject) -> Result<MonitorGuard<'a>> {
        unsafe {
            let _ = jni_unchecked!(self.internal, MonitorEnter, obj.into_inner());
        }

        Ok(MonitorGuard {
            obj: obj.into_inner(),
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
        unsafe {
            let res = jni_unchecked!(self.internal, GetJavaVM, &mut raw);
            jni_error_code_to_result(res)?;
            JavaVM::from_raw(raw)
        }
    }

    /// Ensures that at least a given number of local references can be created
    /// in the current thread.
    pub fn ensure_local_capacity(&self, capacity: jint) -> Result<()> {
        Ok(jni_void_call!(self.internal, EnsureLocalCapacity, capacity))
    }
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
            unsafe { jni_unchecked!(self.env, MonitorExit, self.obj) };
            Ok(())
        });

        match res {
            Err(e) => warn!("error releasing java monitor: {}", e),
            _ => {}
        }
    }
}
