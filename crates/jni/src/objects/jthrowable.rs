crate::bind_java_type! {
    pub JThrowable => "java.lang.Throwable",
    __jni_core = true,
    __sys_type = jthrowable,
    methods {
        /// Get the message of the throwable by calling the `getMessage` method.
        fn get_message() -> JString,

        /// Get the cause of the throwable by calling the `getCause` method.
        fn get_cause() -> JThrowable,

        /// Gets the stack trace of the throwable by calling the `getStackTrace` method.
        fn get_stack_trace() -> JStackTraceElement[]
    }
}
