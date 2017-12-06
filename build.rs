use std::env;

#[cfg(target_os = "linux")]
static JAVA_PATHS: [&'static str; 3] = [
    "/usr/lib/jvm/default",
    "/usr/lib/jvm/default-runtime",
    "/usr/lib/jvm/default-java",
];

#[cfg(target_os = "osx")]
static JAVA_PATHS: [&'static str; 1] = [
    "/Library/Java/JavaVirtualMachines/jdk1.8.0_51.jdk/Contents/Home",
];

#[cfg(target_os = "windows")]
static JAVA_PATHS: [&'static str; 1] = ["C:\\Program Files\\Java\\jre8"];

fn main() {
    if cfg!(feature = "invocation") {
        let noarch_path = "jre/lib/server";
        let arch_path = if cfg!(target_arch = "x86_64") {
            "jre/lib/amd64/server"
        } else if cfg!(target_arch = "x86") {
            "jre/lib/i386/server"
        } else {
            panic!("jni-rs with invocation api is not currently supported on your architecture")
        };

        match env::var("JAVA_HOME").ok() {
            Some(path) => {
                println!("cargo:rustc-link-search=native={}/{}", path, noarch_path);
                println!("cargo:rustc-link-search=native={}/{}", path, arch_path);
            }
            None => for path in JAVA_PATHS.iter() {
                println!("cargo:rustc-link-search=native={}/{}", path, noarch_path);
                println!("cargo:rustc-link-search=native={}/{}", path, arch_path);
            },
        }

        println!("cargo:rustc-link-lib=dylib=jvm");
    }
}
