use std::os::raw::c_void;

use std::ffi::CString;

use sys::{jint, JNI_VERSION_1_1, JavaVMInitArgs, JavaVMOption};

error_chain! {
    errors {
        NullOptString(opt: String) {
            display("internal null in option: {}", opt)
            description("internal null in option string")
        }
    }
}

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
            version: JNI_VERSION_1_1,
        }
    }
}

impl InitArgsBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn option(self, opt_string: &str) -> Self {
        let mut s = self;

        match opt_string {
            "vfprintf" | "abort" | "exit" => return s,
            _ => {}
        }

        s.opts.push(opt_string.into());

        s
    }

    pub fn version(self, version: jint) -> Self {
        let mut s = self;
        s.version = version;
        s
    }

    pub fn ignore_unrecognized(self, ignore: bool) -> Self {
        let mut s = self;
        s.ignore_unrecognized = ignore;
        s
    }

    pub fn build(self) -> Result<InitArgs> {
        let mut opts = Vec::with_capacity(self.opts.len());
        for opt in self.opts {
            let option_string =
                CString::new(opt.as_str()).map_err(|e| ErrorKind::NullOptString(opt))?;
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
