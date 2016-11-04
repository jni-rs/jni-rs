macro_rules! non_null {
    ( $obj:expr, $ctx:expr ) => {
        if $obj.is_null() {
            return Err($crate::errors::ErrorKind::NullPtr($ctx).into());
        } else {
            $obj
        }
    }
}

macro_rules! deref {
    ( $obj:expr, $ctx:expr ) => { *non_null!($obj, $ctx) };
}

macro_rules! jni_method {
    ( $jnienv:expr, $name:tt ) => ({
        trace!("looking up jni method {}", stringify!($name));
        let env = $jnienv;
        match deref!(deref!(env, "JNIEnv"), "*JNIEnv").$name {
            Some(meth) => {
                trace!("found jni method");
                meth
            },
            None => {
                trace!("jnienv method not defined, returning error");
                return Err($crate::errors::Error::from(
                    $crate::errors::ErrorKind::JNIEnvMethodNotFound(
                        stringify!($name))).into())},
        }
    })
}

macro_rules! check_exception {
    ( $jnienv:expr ) => {
        trace!("checking for exception");
        let env: $crate::jnienv::JNIEnv = $jnienv.into();
        if try!(env.exception_check()) {
            trace!("exception found, returning error");
            return Err($crate::errors::Error::from(
                $crate::errors::ErrorKind::JavaException).into());
        }
        trace!("no exception found");
    }
}

macro_rules! jni_unchecked {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling unchecked jni method: {}", stringify!($name));
        let res = jni_method!($jnienv, $name)($jnienv, $($args),*);
        res
    })
}

macro_rules! jni_call {
    ( $jnienv:expr, $name:tt $(, $args:expr )* ) => ({
        trace!("calling checked jni method: {}", stringify!($name));
        unsafe {
            trace!("entering unsafe");
            let res = jni_method!($jnienv, $name)($jnienv, $($args),*);
            check_exception!($jnienv);
            trace!("exiting unsafe");
            non_null!(res, concat!(stringify!($name), " result")).into()
        }
    })
}
