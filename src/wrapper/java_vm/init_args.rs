use std::os::raw::c_void;

use std::ffi::CString;

use sys::{JavaVMInitArgs, JavaVMOption};

use JNIVersion;

error_chain! {
    errors {
        /// Opt string had internal null
        NullOptString(opt: String) {
            display("internal null in option: {}", opt)
            description("internal null in option string")
        }
    }
}

/// Builder for JavaVM InitArgs
pub struct InitArgsBuilder {
    opts: Vec<String>,
    ignore_unrecognized: bool,
    version: i32,
}

impl Default for InitArgsBuilder {
    fn default() -> Self {
        InitArgsBuilder {
            opts: vec![],
            ignore_unrecognized: false,
            version: JNIVersion::V1.into(),
        }
    }
}

impl InitArgsBuilder {
    /// Create a new default InitArgsBuilder
    pub fn new() -> Self {
        Default::default()
    }

    /// Add an option to the init args
    ///
    /// The `vfprintf`, `abort`, and `exit` options are unsupported at this time.
    pub fn option(self, opt_string: &str) -> Self {
        let mut s = self;

        match opt_string {
            "vfprintf" | "abort" | "exit" => return s,
            _ => {}
        }

        s.opts.push(opt_string.into());

        s
    }

    /// Set JNI version for the init args
    ///
    /// Default: V1
    pub fn version(self, version: JNIVersion) -> Self {
        let mut s = self;
        s.version = version.into();
        s
    }

    /// Set the `ignoreUnrecognized` init arg flag
    ///
    /// If ignoreUnrecognized is true, JavaVM::new ignores all unrecognized option strings that
    /// begin with "-X" or "_". If ignoreUnrecognized is false, JavaVM::new returns Err as soon as
    /// it encounters any unrecognized option strings.
    ///
    /// Default: `false`
    pub fn ignore_unrecognized(self, ignore: bool) -> Self {
        let mut s = self;
        s.ignore_unrecognized = ignore;
        s
    }

    /// Build the `InitArgs`
    ///
    /// This will check for internal nulls in the option strings and will return
    /// an error if one is found.
    pub fn build(self) -> Result<InitArgs> {
        let mut opts = Vec::with_capacity(self.opts.len());
        for opt in self.opts {
            let option_string =
                CString::new(opt.as_str()).map_err(|_| ErrorKind::NullOptString(opt))?;
            let jvm_opt = JavaVMOption {
                optionString: option_string.into_raw(),
                extraInfo: ::std::ptr::null_mut(),
            };
            opts.push(jvm_opt);
        }

        Ok(InitArgs {
            inner: JavaVMInitArgs {
                version: self.version,
                ignoreUnrecognized: self.ignore_unrecognized as _,
                options: opts.as_ptr() as _,
                nOptions: opts.len() as _,
            },
            opts,
        })
    }
}

/// JavaVM InitArgs
pub struct InitArgs {
    inner: JavaVMInitArgs,
    opts: Vec<JavaVMOption>,
}

impl InitArgs {
    pub(crate) fn inner_ptr(&self) -> *mut c_void {
        &self.inner as *const _ as _
    }
}

impl Drop for InitArgs {
    fn drop(&mut self) {
        for opt in self.opts.iter() {
            unsafe { CString::from_raw(opt.optionString) };
        }
    }
}
