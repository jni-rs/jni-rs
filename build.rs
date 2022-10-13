//! This build script is used to link with `jvm` dynamic library when
//! `invocation` feature is enabled.
//!
//! To do so, we look for `JAVA_HOME` environment variable.
//! * If it exists, we recursively search for `jvm` library file inside `JAVA_HOME` directory.
//! * If it is not set, we use the following commmand to find actual JVM home directory:
//!   ```bash
//!   java -XshowSettings:properties -version | grep 'java.home'
//!   ```
//!   Then, we search for `jvm` as we have `JAVA_HOME`.
//!
//! On Windows, we also need to find `jvm.lib` file which is used while linking
//! at build time. This file is typically placed in `$JAVA_HOME/lib` directory.

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(target_os = "windows")]
const EXPECTED_JVM_FILENAME: &str = "jvm.dll";
#[cfg(any(
    target_os = "android",
    target_os = "freebsd",
    target_os = "linux",
    target_os = "netbsd",
    target_os = "openbsd"
))]
const EXPECTED_JVM_FILENAME: &str = "libjvm.so";
#[cfg(target_os = "macos")]
const EXPECTED_JVM_FILENAME: &str = "libjli.dylib";

fn main() {
    if cfg!(feature = "invocation") {
        let java_home = match env::var("JAVA_HOME") {
            Ok(java_home) => PathBuf::from(java_home),
            Err(_) => find_java_home().expect(
                "Failed to find Java home directory. \
                 Try setting JAVA_HOME",
            ),
        };

        let libjvm_path =
            find_libjvm(&java_home).expect("Failed to find libjvm.so. Check JAVA_HOME");

        println!("cargo:rustc-link-search=native={}", libjvm_path.display());

        // On Windows, we need additional file called `jvm.lib`
        // and placed inside `JAVA_HOME\lib` directory.
        if cfg!(windows) {
            let lib_path = java_home.join("lib");
            println!("cargo:rustc-link-search={}", lib_path.display());
        }

        println!("cargo:rerun-if-env-changed=JAVA_HOME");

        // On MacOS, we need to link to libjli instead of libjvm as a workaround
        // to a Java8 bug. See here for more information:
        // https://bugs.openjdk.java.net/browse/JDK-7131356
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-lib=dylib=jli");
        } else {
            println!("cargo:rustc-link-lib=dylib=jvm");
        }
    }
}

/// To find Java home directory, we call
/// `java -XshowSettings:properties -version` command and parse its output to
/// find the line `java.home=<some path>`.
fn find_java_home() -> Option<PathBuf> {
    Command::new("java")
        .arg("-XshowSettings:properties")
        .arg("-version")
        .output()
        .ok()
        .and_then(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stdout.lines().chain(stderr.lines()) {
                if line.contains("java.home") {
                    let pos = line.find('=').unwrap() + 1;
                    let path = line[pos..].trim();
                    return Some(PathBuf::from(path));
                }
            }
            None
        })
}

fn find_libjvm<S: AsRef<Path>>(path: S) -> Option<PathBuf> {
    let walker = walkdir::WalkDir::new(path).follow_links(true);

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_e) => continue,
        };

        let file_name = entry.file_name().to_str().unwrap_or("");

        if file_name == EXPECTED_JVM_FILENAME {
            return entry.path().parent().map(Into::into);
        }
    }

    None
}
