use std::str;

use std::marker::PhantomData;

use sys::{self, jvalue, jint};

use errors::*;

use ffi_str::JNIString;
use java_str::JavaStr;

use jmap::JMap;
use jvalue::JValue;
use jclass::JClass;
use jobject::JObject;
use jstring::JString;
use jthrowable::JThrowable;
use jmethodid::JMethodID;

use desc::Desc;
use jclass::ClassDesc;
use jclass::IntoClassDesc;
use jmethodid::MethodDesc;
use jmethodid::IntoMethodDesc;

use signature::TypeSignature;
use signature::JavaType;
use signature::Primitive;

use global_ref::GlobalRef;

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
    pub fn get_version(&self) -> Result<jint> {
        Ok(unsafe { jni_unchecked!(self.internal, GetVersion) })
    }

    /// THROWS:
    ///    ClassFormatError: if the class data does not specify a valid class.
    ///    ClassCircularityError: if a class or interface would be its own
    ///        superclass or superinterface.
    ///    OutOfMemoryError: if the system runs out of memory.
    ///    SecurityException: if the caller attempts to define a class in the
    ///        "java" package tree.
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
                              buf.as_ptr() as *const i8,
                              buf.len() as i32);
        Ok(class)
    }

    /// THROWS:
    ///    ClassFormatError: if the class data does not specify a valid class.
    ///    ClassCircularityError: if a class or interface would be its own
    ///        superclass or superinterface.
    ///    NoClassDefFoundError: if no definition for a requested class or
    ///        interface can be found.
    ///    OutOfMemoryError: if the system runs out of memory.
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

    pub fn get_superclass<S, T>(&self, class: T) -> Result<JClass<'a>>
        where S: Into<JNIString>,
              T: IntoClassDesc<'a, S>
    {
        let class = try!(self.find_class(class));
        Ok(jni_call!(self.internal, GetSuperclass, class.into_inner()))
    }

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

    pub fn exception_occurred(&self) -> Result<JThrowable> {
        let throwable = jni_call!(self.internal, ExceptionOccurred);
        Ok(throwable)
    }

    pub fn exception_describe(&self) -> Result<()> {
        unsafe { jni_unchecked!(self.internal, ExceptionDescribe) };
        Ok(())
    }

    pub fn exception_clear(&self) -> Result<()> {
        unsafe { jni_unchecked!(self.internal, ExceptionClear) };
        Ok(())
    }

    pub fn fatal_error<S: Into<JNIString>>(&self, msg: S) -> Result<()> {
        let msg = msg.into();
        unsafe { jni_unchecked!(self.internal, FatalError, msg.as_ptr()) };
        Ok(())
    }

    pub fn exception_check(&self) -> Result<bool> {
        let check = unsafe { jni_unchecked!(self.internal, ExceptionCheck) } ==
                    sys::JNI_TRUE;
        Ok(check)
    }

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

    pub fn alloc_object<S, T>(&self, class: T) -> Result<JObject<'a>>
        where S: Into<JNIString>,
              T: IntoClassDesc<'a, S>
    {
        let class = try!(self.find_class(class));
        Ok(jni_call!(self.internal, AllocObject, class.into_inner()))
    }

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

    pub fn get_object_class(&self, obj: JObject) -> Result<JClass> {
        Ok(jni_call!(self.internal, GetObjectClass, obj.into_inner()))
    }

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

        let jni_args: Vec<jvalue> = args.iter().map(|v| v.to_jni()).collect();

        let class = class.into_inner();
        let jni_args = jni_args.as_ptr();

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

    // calls a method in an unsafe manner. Assumes that the obj and method id
    // arguments line up and that the number of args matches the signature.
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
        let jni_args: Vec<jvalue> = args.iter().map(|v| v.to_jni()).collect();

        let method_id = try!(self.get_method_id(method_id)).into_inner();

        let obj = obj.into_inner();
        let jni_args = jni_args.as_ptr();

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

    // calls a method safely by looking up the id based on the object's class,
    // method name, and signature and parsing the signature to ensure that the
    // arguments and return type line up.
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

    // calls a static method safely by looking up the id based on the class,
    // method name, and signature and parsing the signature to ensure that the
    // arguments and return type line up.
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
            ctor_args.iter().map(|v| unsafe { v.to_jni() }).collect();

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

    pub fn get_map(&'a self, obj: JObject<'a>) -> Result<JMap<'a>> {
        non_null!(obj, "get_map obj argument");
        JMap::from_env(self, obj)
    }

    pub fn get_string(&self, obj: JString) -> Result<JavaStr> {
        non_null!(obj, "get_string obj argument");
        JavaStr::from_env(self, obj.into_inner())
    }

    pub fn new_string<S: Into<JNIString>>(&self, from: S) -> Result<JString> {
        let ffi_str = from.into();
        Ok(jni_call!(self.internal, NewStringUTF, ffi_str.as_ptr()))
    }
}
