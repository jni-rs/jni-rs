crate::bind_java_type! {
    pub JStackTraceElement => "java.lang.StackTraceElement",
    methods {
        /// Get the class name of the stack trace element.
        fn get_class_name() -> JString,
        /// Get the file name of the stack trace element, if available.
        fn get_file_name() -> JString,
        /// Get the line number of the stack trace element.
        fn get_line_number() -> jint,
        /// Get the method name of the stack trace element.
        fn get_method_name() -> JString,
        /// Check if the stack trace element corresponds with a native method.
        fn is_native() -> bool,
        /// Returns a string representation of this stack trace element.
        fn try_to_string {
            name = "toString",
            sig = () -> JString,
        }
    }
}
