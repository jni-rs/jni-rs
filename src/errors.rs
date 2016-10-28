error_chain!{
    foreign_links {
        ::std::ffi::NulError, NulError;
    }

    errors {
        InvalidCtorReturn {
            description("Invalid contructor return type (must be void)")
            display("Invalid contructor return type (must be void)")
        }
        InvalidArgList {
            description("Invalid number of arguments passed to java method")
            display("Invalid number of arguments passed to java method")
        }
        MethodNotFound(name: String) {
            description("Method not found")
            display("Method not found: {}", name)
        }
        JavaException {
            description("Java exception was thrown")
            display("Java exception was thrown")
        }
        JNIEnvMethodNotFound(name: &'static str) {
            description("Method pointer null in JNIEnv")
            display("JNIEnv null method pointer for {}", name)
        }
        NullPtr(context: &'static str) {
            description("null pointer")
            display("null pointer in {}", context)
        }
    }
}
