extern crate walkdir;

use std::env;
use std::path::{
    Path,
    PathBuf,
};
use std::process::Command;

#[cfg(target_os = "windows")]
const EXPECTED_JVM_FILENAME: &str = "jvm.dll";
#[cfg(target_os = "linux")]
const EXPECTED_JVM_FILENAME: &str = "libjvm.so";
#[cfg(target_os = "macos")]
const EXPECTED_JVM_FILENAME: &str = "libjvm.dylib";

fn main() {
    if cfg!(feature = "invocation") {
        let java_home = match env::var("JAVA_HOME") {
            Ok(java_home) => PathBuf::from(java_home),
            Err(_) => find_java_home().expect("Failed to find Java home directory. Try setting JAVA_HOME")
        };

        let libjvm_path = find_libjvm(&java_home).expect("Failed to find libjvm.so. Check JAVA_HOME");

        println!("cargo:rustc-link-search=native={}", libjvm_path.display());

        if cfg!(windows) {
            let lib_path = java_home.join("lib");
            println!("cargo:rustc-link-search={}", lib_path.display());
        }

        println!("cargo:rustc-link-lib=dylib=jvm");
    }
}

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
