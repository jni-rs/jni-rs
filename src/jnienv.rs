use errors::*;

use macros;

use jni_sys::{self, jclass, jstring, jboolean, jobject, jvalue, jint, jsize};

use std::ffi;
use std::str;

use std::marker::PhantomData;

use jvalue::JValue;
use jclass::JClass;
use jobject::JObject;
use jstring::JString;
use jthrowable::JThrowable;
use jmethodid::JMethodID;

use java_string::JavaStr;

use signature::TypeSignature;
use signature::JavaType;
use signature::Primitive;

use global_ref::GlobalRef;

#[repr(C)]
pub struct JNIEnv<'a> {
    pub internal: *mut jni_sys::JNIEnv,
    lifetime: PhantomData<&'a ()>,
}

impl<'a> From<*mut jni_sys::JNIEnv> for JNIEnv<'a> {
    fn from(other: *mut jni_sys::JNIEnv) -> Self {
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
    ///    SecurityException: if the caller attempts to define a class in the "java" package tree.
    pub fn define_class<S: Into<Vec<u8>>>(&self,
                                          name: S,
                                          loader: JObject,
                                          buf: &[u8])
                                          -> Result<JClass> {
        non_null!(loader, "define_class loader argument");
        let name = ffi::CString::new(name)?;
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
    pub fn find_class<S: Into<Vec<u8>>>(&self,
                                        name: S)
                                        -> Result<JClass> {
        let name = ffi::CString::new(name)?;
        let class = jni_call!(self.internal, FindClass, name.as_ptr());
        Ok(class)
    }

    pub fn get_superclass(&self, class: JClass) -> Result<JClass> {
        non_null!(class, "find_class class argument");
        Ok(jni_call!(self.internal, GetSuperclass, class.into_inner()))
    }

    pub fn is_assignable_from(&self,
                              class1: JClass,
                              class2: JClass)
                              -> Result<bool> {
        non_null!(class1, "is_assignable_from class1 argument");
        non_null!(class2, "is_assignable_from class2 argument");
        Ok(unsafe {
            jni_unchecked!(self.internal,
                           IsAssignableFrom,
                           class1.into_inner(),
                           class2.into_inner())
        } == jni_sys::JNI_TRUE)
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

    pub fn throw_new<S: Into<Vec<u8>>>(&self,
                                       class: JClass,
                                       msg: S)
                                       -> Result<()> {
        non_null!(class, "throw_new class argument");
        let msg = ffi::CString::new(msg)?;
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

    pub fn fatal_error<S: Into<Vec<u8>>>(&self, msg: S) -> Result<()> {
        unsafe {
            jni_unchecked!(self.internal,
                           FatalError,
                           ffi::CString::new(msg)?.as_ptr())
        };
        Ok(())
    }

    pub fn exception_check(&self) -> Result<bool> {
        let check = unsafe { jni_unchecked!(self.internal, ExceptionCheck) } ==
                    jni_sys::JNI_TRUE;
        Ok(check)
    }

    pub fn new_global_ref(&self, obj: JObject) -> Result<GlobalRef> {
        non_null!(obj, "new_global_ref obj argument");
        let new_ref: JObject =
            jni_call!(self.internal, NewGlobalRef, obj.into_inner());
        let global  = unsafe { GlobalRef::new(self.internal, new_ref.into_inner()) };
        Ok(global)
    }

    pub fn new_local_ref(&self, obj: JObject) -> Result<JObject> {
        non_null!(obj, "new_local_ref obj argument");
        Ok(jni_call!(self.internal, NewLocalRef, obj.into_inner()))
    }

    pub fn delete_local_ref(&self, obj: JObject) -> Result<()> {
        non_null!(obj, "delete_local_ref obj argument");
        Ok(unsafe {
            jni_unchecked!(self.internal, DeleteLocalRef, obj.into_inner());
            check_exception!(self.internal);
        })
    }

    pub fn alloc_object(&self, class: JClass) -> Result<JObject> {
        non_null!(class, "alloc_object class argument");
        Ok(jni_call!(self.internal, AllocObject, class.into_inner()))
    }

    fn get_method_id(&self,
                     class: jclass,
                     name: *const i8,
                     sig: *const i8)
                     -> Result<JMethodID> {
        non_null!(class, "get_method_id class argument");
        Ok(jni_call!(self.internal, GetMethodID, class, name, sig))
    }

    pub fn get_object_class(&self, obj: JObject) -> Result<JClass> {
        Ok(jni_call!(self.internal, GetObjectClass, obj.into_inner()))
    }

    pub unsafe fn call_static_method_unsafe(&self,
                                            class: JClass,
                                            method_id: JMethodID,
                                            ret: JavaType,
                                            args: &[JValue]) -> Result<JValue> {
        let jni_args: Vec<jvalue> = args.iter().map(|v| v.to_jni()).collect();

        let method_id = method_id.into_inner();

        let class = class.into_inner();
        let jni_args = jni_args.as_ptr();

        // TODO clean this up
        Ok(match ret {
            JavaType::Object(_) | JavaType::Array(_) => {
                #[allow(unused_unsafe)]
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
                         jni_sys::JNI_TRUE)
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
            }, // JavaType::Primitive
        }) // match parsed.ret
    }

    // calls a method in an unsafe manner. Assumes that the obj and method id
    // arguments line up and that the number of args matches the signature.
    pub unsafe fn call_method_unsafe(&self,
                                     obj: JObject,
                                     method_id: JMethodID,
                                     ret: JavaType,
                                     args: &[JValue]) -> Result<JValue>{
        let jni_args: Vec<jvalue> = args.iter().map(|v| v.to_jni()).collect();

        let method_id = method_id.into_inner();

        let obj = obj.into_inner();
        let jni_args = jni_args.as_ptr();

        // TODO clean this up
        Ok(match ret {
            JavaType::Object(_) | JavaType::Array(_) => {
                #[allow(unused_unsafe)]
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
                         jni_sys::JNI_TRUE)
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
            }, // JavaType::Primitive
        }) // match parsed.ret
    }

    // calls a method safely by looking up the id based on the object's class,
    // method name, and signature and parsing the signature to ensure that the
    // arguments and return type line up.
    pub fn call_method<S, T>(&self,
                             obj: JObject,
                             name: S,
                             sig: T,
                             args: &[JValue])
                             -> Result<JValue>
        where S: Into<String>,
              T: Into<String>
    {
        non_null!(obj, "call_method obj argument");
        let class = self.get_object_class(obj)?;

        // build strings
        let name: String = name.into();
        let sig: String = sig.into();

        // parse the signature
        let parsed = TypeSignature::from_str(&sig)?;
        if parsed.args.len() != args.len() {
            return Err(ErrorKind::InvalidArgList.into());
        }

        // build ffi-compatible strings
        let ffi_name = ffi::CString::new(name.as_str())?;
        let ffi_sig = ffi::CString::new(sig)?;

        // get the actual method
        let method_id = match self.get_method_id(class.into_inner(),
                                                 ffi_name.as_ptr(),
                                                 ffi_sig.as_ptr()) {
            Err(e) => match e.kind() {
                &ErrorKind::NullPtr(_) => return Err(ErrorKind::MethodNotFound(name).into()),
                _ => return Err(e),
            },
            Ok(id) => id,
        };

        unsafe { self.call_method_unsafe(obj, method_id, parsed.ret, args) }
    }

    // calls a static method safely by looking up the id based on the class,
    // method name, and signature and parsing the signature to ensure that the
    // arguments and return type line up.
    pub fn call_static_method<S, T, U>(&self,
                             class: S,
                             name: T,
                             sig: U,
                             args: &[JValue])
                             -> Result<JValue>
        where S: Into<Vec<u8>>,
              T: Into<String>,
              U: Into<String>
    {
        // build strings
        let name: String = name.into();
        let sig: String = sig.into();

        // look up the class
        let class: JClass = self.find_class(class)?;

        // parse the signature
        let parsed = TypeSignature::from_str(&sig)?;
        if parsed.args.len() != args.len() {
            return Err(ErrorKind::InvalidArgList.into());
        }

        // build ffi-compatible strings
        let ffi_name = ffi::CString::new(name.as_str())?;
        let ffi_sig = ffi::CString::new(sig)?;

        // get the actual method
        let method_id = match self.get_method_id(class.into_inner(),
                                                 ffi_name.as_ptr(),
                                                 ffi_sig.as_ptr()) {
            Err(e) => match e.kind() {
                &ErrorKind::NullPtr(_) => return Err(ErrorKind::MethodNotFound(name).into()),
                _ => return Err(e),
            },
            Ok(id) => id,
        };

        unsafe { self.call_static_method_unsafe(class, method_id, parsed.ret, args) }
    }

    pub fn new_object<S, T>(&self,
                            class: S,
                            ctor_sig: T,
                            ctor_args: &[JValue])
                            -> Result<JObject>
        where S: Into<String>,
              T: Into<String>
    {
        let jni_args: Vec<jvalue> =
            ctor_args.iter().map(|v| unsafe { v.to_jni() }).collect();

        let class: String = class.into();

        let class = self.find_class(class)?;

        // build strings
        let name: String = "<init>".into();
        let sig: String = ctor_sig.into();

        // parse the signature
        let parsed = TypeSignature::from_str(&sig)?;
        if parsed.args.len() != jni_args.len() {
            return Err(ErrorKind::InvalidArgList.into());
        }

        if parsed.ret != JavaType::Primitive(Primitive::Void) {
            return Err(ErrorKind::InvalidCtorReturn.into());
        }

        // build ffi-compatible strings
        let ffi_name = ffi::CString::new(name.as_str())?;
        let ffi_sig = ffi::CString::new(sig)?;

        // get the actual method
        let method_id = match self.get_method_id(class.into_inner(),
                                                 ffi_name.as_ptr(),
                                                 ffi_sig.as_ptr()) {
            Err(e) => match e.kind() {
                &ErrorKind::NullPtr(_) => return Err(ErrorKind::MethodNotFound(name).into()),
                _ => return Err(e),
            },
            Ok(id) => id.into_inner(),
        };

        let jni_args = jni_args.as_ptr();

        Ok(jni_call!(self.internal,
                     NewObjectA,
                     class.into_inner(),
                     method_id,
                     jni_args))
    }

    pub fn get_string(&self, obj: JString) -> Result<JavaStr> {
        non_null!(obj, "get_string obj argument");
        JavaStr::from_env(self, obj.into_inner())
    }

    pub fn new_string<S: AsRef<str>>(&self, from: S) -> Result<JString> {
        use cesu8::to_java_cesu8;
        use std::borrow::Borrow;

        let cow = to_java_cesu8(from.as_ref());
        let slice: &[u8] = cow.borrow();
        let with_null = ffi::CString::new(slice)?;

        Ok(jni_call!(self.internal, NewStringUTF, with_null.as_ptr()))
    }

    // pub fn get_string(&self, str_obj: JString) -> Result<String> {
    //     let jni_env = self.internal;
    //     let mut copy = false as jboolean;

    //     Ok(unsafe {
    //         String::from_utf8(ffi::CStr::from_ptr(jni_unchecked!(jni_env,
    //                                                              GetStringUTFChars,
    //                                                              str_obj.into_inner(),
    //                                                              &mut copy))
    //                 .to_bytes()
    //                 .into())
    //             .unwrap()
    //     })
    // }
}
