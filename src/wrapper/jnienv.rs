use std::str;

use std::marker::PhantomData;

use std::iter::IntoIterator;

use errors::*;

use sys::{self, jvalue, jint, jsize, jbyte};

use strings::JNIString;
use strings::JavaStr;

use objects::JMap;
use objects::JValue;
use objects::JClass;
use objects::JObject;
use objects::JString;
use objects::JThrowable;
use objects::JMethodID;
use objects::GlobalRef;

use descriptors::Desc;
use descriptors::ClassDesc;
use descriptors::IntoClassDesc;
use descriptors::MethodDesc;
use descriptors::IntoMethodDesc;

use signature::TypeSignature;
use signature::JavaType;
use signature::Primitive;

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
    pub internal: *mut sys::JNIEnv,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<*mut sys::JNIEnv> for JNIEnv<'a> {
    fn from(other: *mut sys::JNIEnv) -> Self {
        JNIEnv {
            internal: other,
            lifetime: PhantomData,
        }
    }
}

impl<'a> JNIEnv<'a> {
    /// Get the java version that we're being executed from. This is encoded and
    /// will need to be checked against constants from the sys module.
    ///
    /// TODO: convert this to something more usable.
    pub fn get_version(&self) -> Result<jint> {
        Ok(unsafe { jni_unchecked!(self.internal, GetVersion) })
    }

    /// Define a new java class. See the JNI docs for more details - I've never
    /// had occasion to use this and haven't researched it fully.
    pub fn define_class<S>(&self,
                           name: S,
                           loader: JObject,
                           buf: &[u8])
                           -> Result<JClass>
        where S: Into<JNIString>
    {
        non_null!(loader, "define_class loader argument");
        let name = name.into();
        let class = jni_call!(self.internal,
                              DefineClass,
                              name.as_ptr(),
                              loader.into_inner(),
                              buf.as_ptr() as *const jbyte,
                              buf.len() as jsize);
        Ok(class)
    }

    /// Look up a class by name. The argument to this will be something like
    /// `java/lang/String`. This can also take a concrete JClass, in which case
    /// it simply returns it after doing a null check. This is so that it can be
    /// generic over both concrete classes and class descriptor strings. Methods
    /// with class arguments should therefore take `IntoClassDesc` instead, and
    /// use ths when an actual class is needed. That way, optimizations such as
    /// reusing a class object to look up multiple methods can be done.
    ///
    /// # Example
    /// ```rust,ignore
    /// let class: JClass<'a> = env.find_class("java/lang/String");
    /// ```
    pub fn find_class<S, T>(&self, name: T) -> Result<JClass<'a>>
        where S: Into<JNIString>,
              T: IntoClassDesc<'a, S>
    {
        let ClassDesc(name) = name.into_desc();
        match name {
            Desc::Descriptor(name) => {
                let name = name.into();
                let class = jni_call!(self.internal, FindClass, name.as_ptr());
                Ok(class)
            }
            Desc::Value(class) => {
                non_null!(class, "find_class value");
                Ok(class)
            }
        }
    }

    /// Get the superclass for a particular class. As with `find_class`, takes
    /// a descriptor.
    pub fn get_superclass<S, T>(&self, class: T) -> Result<JClass<'a>>
        where S: Into<JNIString>,
              T: IntoClassDesc<'a, S>
    {
        let class = try!(self.find_class(class));
        Ok(jni_call!(self.internal, GetSuperclass, class.into_inner()))
    }

    /// Tests whether class1 is assignable from class2.
    pub fn is_assignable_from<S, T, U, V>(&self,
                                          class1: U,
                                          class2: V)
                                          -> Result<bool>
        where S: Into<JNIString>,
              T: Into<JNIString>,
              U: IntoClassDesc<'a, S>,
              V: IntoClassDesc<'a, T>
    {
        let class1 = try!(self.find_class(class1));
        let class2 = try!(self.find_class(class2));
        Ok(unsafe {
            jni_unchecked!(self.internal,
                           IsAssignableFrom,
                           class1.into_inner(),
                           class2.into_inner())
        } == sys::JNI_TRUE)
    }

    /// Raise an exception from an existing object. This will continue being
    /// thrown in java unless `exception_clear` is called.
    pub fn throw(&self, obj: JThrowable) -> Result<()> {
        let obj = non_null!(obj, "throw obj argument");
        let res: i32 =
            unsafe { jni_unchecked!(self.internal, Throw, obj.into_inner()) };
        if res < 0 {
            Err(format!("throw failed with code {}", res).into())
        } else {
            Ok(())
        }
    }

    /// Create and throw a new exception from a class descriptor and an error
    /// message.
    pub fn throw_new<S, T, U>(&self, class: U, msg: T) -> Result<()>
        where S: Into<JNIString>,
              T: Into<JNIString>,
              U: IntoClassDesc<'a, S>
    {
        let class = try!(self.find_class(class));
        let msg = msg.into();
        let res: i32 = unsafe {
            jni_unchecked!(self.internal,
                           ThrowNew,
                           class.into_inner(),
                           msg.as_ptr())
        };
        if res < 0 {
            Err(format!("throw failed with code {}", res).into())
        } else {
            Ok(())
        }
    }

    /// Check whether or not an exception is currently in the process of being
    /// thrown. An exception is in this state from the time it gets thrown and
    /// not caught in a java function until `exception_clear` is called.
    pub fn exception_occurred(&self) -> Result<JThrowable> {
        let throwable = jni_call!(self.internal, ExceptionOccurred);
        Ok(throwable)
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
    pub fn fatal_error<S: Into<JNIString>>(&self, msg: S) -> Result<()> {
        let msg = msg.into();
        unsafe { jni_unchecked!(self.internal, FatalError, msg.as_ptr()) };
        Ok(())
    }

    /// Check to see if an exception is being thrown. This only differs from
    /// `exception_occurred` in that it doesn't return the actual thrown
    /// exception.
    pub fn exception_check(&self) -> Result<bool> {
        let check = unsafe { jni_unchecked!(self.internal, ExceptionCheck) } ==
                    sys::JNI_TRUE;
        Ok(check)
    }

    /// Turns an object into a global ref. This has the benefit of removing the
    /// lifetime bounds since it's guaranteed to not get GC'd by java. It
    /// releases the GC pin upon being dropped.
    pub fn new_global_ref(&self, obj: JObject) -> Result<GlobalRef> {
        non_null!(obj, "new_global_ref obj argument");
        let new_ref: JObject =
            jni_call!(self.internal, NewGlobalRef, obj.into_inner());
        let global =
            unsafe { GlobalRef::new(self.internal, new_ref.into_inner()) };
        Ok(global)
    }

    // Not public yet - not sure what the GC behavior is. Needs more research
    #[allow(dead_code)]
    fn new_local_ref(&self, obj: JObject) -> Result<JObject> {
        non_null!(obj, "new_local_ref obj argument");
        Ok(jni_call!(self.internal, NewLocalRef, obj.into_inner()))
    }

    #[allow(dead_code)]
    fn delete_local_ref(&self, obj: JObject) -> Result<()> {
        non_null!(obj, "delete_local_ref obj argument");
        Ok(unsafe {
            jni_unchecked!(self.internal, DeleteLocalRef, obj.into_inner());
            check_exception!(self.internal);
        })
    }

    /// Allocates a new object from a class descriptor without running a
    /// constructor.
    pub fn alloc_object<S, T>(&self, class: T) -> Result<JObject<'a>>
        where S: Into<JNIString>,
              T: IntoClassDesc<'a, S>
    {
        let class = try!(self.find_class(class));
        Ok(jni_call!(self.internal, AllocObject, class.into_inner()))
    }

    /// Look up a method by class descriptor (or concrete class), name, and
    /// signature. Like `find_class`, this is generic over descriptors and
    /// concrete JMethodID objects and can take both. If given a concrete
    /// object, it simply returns it.
    ///
    /// # Example
    /// ```rust,ignore
    /// let method_id: JMethodID = env.get_method_id(
    ///     ("java/lang/String", "getString", "()Ljava/lang/String;"),
    /// );
    /// ```
    pub fn get_method_id<S, T, U, V, W>(&self, desc: W) -> Result<JMethodID<'a>>
        where S: Into<JNIString>,
              T: IntoClassDesc<'a, S>,
              U: Into<JNIString>,
              V: Into<JNIString>,
              W: IntoMethodDesc<'a, S, T, U, V>
    {
        let MethodDesc(desc, _) = desc.into_desc();
        match desc {
            Desc::Descriptor((class, name, sig)) => {
                // TODO this block is ugly and does an extra copy on errors.
                // Fix?
                let class = try!(self.find_class(class));
                let ffi_name = name.into();
                let sig = sig.into();

                let res = (|| -> Result<JMethodID> {
                    Ok(jni_call!(self.internal,
                                 GetMethodID,
                                 class.into_inner(),
                                 ffi_name.as_ptr(),
                                 sig.as_ptr()))
                })();

                match res {
                    Ok(m) => Ok(m),
                    Err(e) => {
                        match e.kind() {
                            &ErrorKind::NullPtr(_) => {
                                let name: String = ffi_name.into();
                                return Err(ErrorKind::MethodNotFound(name)
                                    .into());
                            }
                            _ => return Err(e),
                        }
                    }
                }
            }
            Desc::Value(id) => Ok(id),
        }
    }

    /// Get the class for an object.
    pub fn get_object_class(&self, obj: JObject) -> Result<JClass> {
        Ok(jni_call!(self.internal, GetObjectClass, obj.into_inner()))
    }

    /// Call a static method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the class, whether the return type
    /// is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `CallStatic<Type>MethodA` method
    /// with the provided arguments.
    #[allow(unused_unsafe)]
    pub unsafe fn call_static_method_unsafe<S, T, U, V, W, X, Y>
        (&self,
         class: Y,
         method_id: W,
         ret: JavaType,
         args: &[JValue<'a>])
         -> Result<JValue<'a>>
        where S: Into<JNIString>,
              T: IntoClassDesc<'a, S>,
              U: Into<JNIString>,
              V: Into<JNIString>,
              W: IntoMethodDesc<'a, S, T, U, V>,
              X: Into<JNIString>,
              Y: IntoClassDesc<'a, X>
    {
        let class = try!(self.find_class(class));

        let MethodDesc(method_desc, _) = method_id.into_desc();
        let method_desc = match method_desc {
            Desc::Descriptor((_, name, sig)) => {
                Desc::Descriptor((class, name, sig))
            }
            Desc::Value(v) => Desc::Value(v),
        };

        let method_id = try!(self.get_method_id(method_desc)).into_inner();

        let class = class.into_inner();
        let args: Vec<jvalue> = args.into_iter().map(|v| v.to_jni()).collect();
        let jni_args = args.as_ptr();

        // TODO clean this up
        Ok(match ret {
            JavaType::Object(_) |
            JavaType::Array(_) => {
                let obj: JObject = jni_call!(self.internal,
                                             CallStaticObjectMethodA,
                                             class,
                                             method_id,
                                             jni_args);
                obj.into()
            } // JavaType::Object
            JavaType::Method(_) => unimplemented!(),
            JavaType::Primitive(p) => {
                let v: JValue = match p {
                    Primitive::Boolean => {
                        (jni_unchecked!(self.internal,
                                        CallStaticBooleanMethodA,
                                        class,
                                        method_id,
                                        jni_args) ==
                         sys::JNI_TRUE)
                            .into()
                    }
                    Primitive::Char => {
                        jni_unchecked!(self.internal,
                                       CallStaticCharMethodA,
                                       class,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Short => {
                        jni_unchecked!(self.internal,
                                       CallStaticShortMethodA,
                                       class,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Int => {
                        jni_unchecked!(self.internal,
                                       CallStaticIntMethodA,
                                       class,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Long => {
                        jni_unchecked!(self.internal,
                                       CallStaticLongMethodA,
                                       class,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Float => {
                        jni_unchecked!(self.internal,
                                       CallStaticFloatMethodA,
                                       class,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Double => {
                        jni_unchecked!(self.internal,
                                       CallStaticDoubleMethodA,
                                       class,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Byte => {
                        jni_unchecked!(self.internal,
                                       CallStaticByteMethodA,
                                       class,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Void => {
                        jni_unchecked!(self.internal,
                                       CallStaticVoidMethodA,
                                       class,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                };
                v.into()
            } // JavaType::Primitive
        }) // match parsed.ret
    }

    /// Call an object method in an unsafe manner. This does nothing to check
    /// whether the method is valid to call on the object, whether the return type
    /// is correct, or whether the number of args is valid for the method.
    ///
    /// Under the hood, this simply calls the `Call<Type>MethodA` method
    /// with the provided arguments.
    #[allow(unused_unsafe)]
    pub unsafe fn call_method_unsafe<S, T, U, V, W>(&self,
                                                    obj: JObject,
                                                    method_id: W,
                                                    ret: JavaType,
                                                    args: &[JValue<'a>])
                                                    -> Result<JValue<'a>>
        where S: Into<JNIString>,
              T: IntoClassDesc<'a, S>,
              U: Into<JNIString>,
              V: Into<JNIString>,
              W: IntoMethodDesc<'a, S, T, U, V>
    {
        let method_id = try!(self.get_method_id(method_id)).into_inner();

        let obj = obj.into_inner();

        let args: Vec<jvalue> = args.into_iter().map(|v| v.to_jni()).collect();
        let jni_args = args.as_ptr();

        // TODO clean this up
        Ok(match ret {
            JavaType::Object(_) |
            JavaType::Array(_) => {
                let obj: JObject = jni_call!(self.internal,
                                             CallObjectMethodA,
                                             obj,
                                             method_id,
                                             jni_args);
                obj.into()
            } // JavaType::Object
            JavaType::Method(_) => unimplemented!(),
            JavaType::Primitive(p) => {
                let v: JValue = match p {
                    Primitive::Boolean => {
                        (jni_unchecked!(self.internal,
                                        CallBooleanMethodA,
                                        obj,
                                        method_id,
                                        jni_args) ==
                         sys::JNI_TRUE)
                            .into()
                    }
                    Primitive::Char => {
                        jni_unchecked!(self.internal,
                                       CallCharMethodA,
                                       obj,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Short => {
                        jni_unchecked!(self.internal,
                                       CallShortMethodA,
                                       obj,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Int => {
                        jni_unchecked!(self.internal,
                                       CallIntMethodA,
                                       obj,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Long => {
                        jni_unchecked!(self.internal,
                                       CallLongMethodA,
                                       obj,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Float => {
                        jni_unchecked!(self.internal,
                                       CallFloatMethodA,
                                       obj,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Double => {
                        jni_unchecked!(self.internal,
                                       CallDoubleMethodA,
                                       obj,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Byte => {
                        jni_unchecked!(self.internal,
                                       CallByteMethodA,
                                       obj,
                                       method_id,
                                       jni_args)
                            .into()
                    }
                    Primitive::Void => {
                        jni_unchecked!(self.internal,
                                       CallVoidMethodA,
                                       obj,
                                       method_id,
                                       jni_args);
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
    pub fn call_method<S, T>(&'a self,
                             obj: JObject<'a>,
                             name: S,
                             sig: T,
                             args: &[JValue<'a>])
                             -> Result<JValue<'a>>
        where S: Into<JNIString>,
              T: Into<JNIString> + AsRef<str>
    {
        non_null!(obj, "call_method obj argument");

        // parse the signature
        let parsed = try!(TypeSignature::from_str(sig.as_ref()));
        if parsed.args.len() != args.len() {
            return Err(ErrorKind::InvalidArgList.into());
        }

        let class = try!(self.get_object_class(obj));

        unsafe {
            self.call_method_unsafe(obj, (class, name, sig), parsed.ret, args)
        }
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
    pub fn call_static_method<S, T, U, V>(&self,
                                          class: V,
                                          name: T,
                                          sig: U,
                                          args: &[JValue<'a>])
                                          -> Result<JValue<'a>>
        where S: Into<JNIString>,
              T: Into<JNIString>,
              U: Into<JNIString> + AsRef<str>,
              V: IntoClassDesc<'a, S>
    {
        let parsed = try!(TypeSignature::from_str(&sig));
        if parsed.args.len() != args.len() {
            return Err(ErrorKind::InvalidArgList.into());
        }

        // go ahead and look up the class since it's already Copy,
        // and we'll need that for the next call.
        let class = try!(self.find_class(class));

        unsafe {
            self.call_static_method_unsafe(class,
                                           (class, name, sig),
                                           parsed.ret,
                                           args)
        }
    }

    /// Create a new object using a constructor. This is done safely using
    /// checks similar to those in `call_static_method`.
    pub fn new_object<S, T, U>(&self,
                               class: U,
                               ctor_sig: T,
                               ctor_args: &[JValue<'a>])
                               -> Result<JObject<'a>>
        where S: Into<JNIString>,
              T: Into<JNIString> + AsRef<str>,
              U: IntoClassDesc<'a, S>
    {
        // parse the signature
        let parsed = try!(TypeSignature::from_str(&ctor_sig));

        if parsed.args.len() != ctor_args.len() {
            return Err(ErrorKind::InvalidArgList.into());
        }

        if parsed.ret != JavaType::Primitive(Primitive::Void) {
            return Err(ErrorKind::InvalidCtorReturn.into());
        }

        let jni_args: Vec<jvalue> =
            ctor_args.into_iter().map(|v| v.to_jni()).collect();

        // build strings
        let name = "<init>";

        let class = try!(self.find_class(class));

        let method_id = try!(self.get_method_id((class, name, ctor_sig)));

        let jni_args = jni_args.as_ptr();

        Ok(jni_call!(self.internal,
                     NewObjectA,
                     class.into_inner(),
                     method_id.into_inner(),
                     jni_args))
    }

    /// Cast a JObject to a JMap. This won't throw exceptions or return errors
    /// in the event that the object isn't actually a map, but the methods on
    /// the resulting map object will.
    pub fn get_map(&'a self, obj: JObject<'a>) -> Result<JMap<'a>> {
        non_null!(obj, "get_map obj argument");
        JMap::from_env(self, obj)
    }

    /// Get a JavaStr from a JString. This allows conversions from java string
    /// objects to rust strings.
    ///
    /// This entails a call to `GetStringUTFChars` and only decodes java's
    /// modified UTF-8 format on conversion to a rust-compatible string.
    pub fn get_string(&self, obj: JString) -> Result<JavaStr> {
        non_null!(obj, "get_string obj argument");
        JavaStr::from_env(self, obj.into_inner())
    }

    /// Create a new java string object from a rust string. This requires a
    /// re-encoding of rusts *real* UTF-8 strings to java's modified UTF-8
    /// format.
    pub fn new_string<S: Into<JNIString>>(&self, from: S) -> Result<JString> {
        let ffi_str = from.into();
        Ok(jni_call!(self.internal, NewStringUTF, ffi_str.as_ptr()))
    }
}
