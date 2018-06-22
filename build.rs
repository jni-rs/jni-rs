extern crate walkdir;

use std::env;
use std::ffi;
use std::fs;
use std::path::{
    Path,
    PathBuf,
};

#[cfg(windows)]
const LIBJVM_NAME: &str = "jvm.dll";
#[cfg(all(unix, any(target_os = "macos", target_os = "ios")))]
const LIBJVM_NAME: &str = "libjvm.dylib";
#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
const LIBJVM_NAME: &str = "libjvm.so";

#[cfg(windows)]
const JAVA_EXE_NAME: &str = "java.exe";
#[cfg(not(windows))]
const JAVA_EXE_NAME: &str = "java";

#[cfg(windows)]
const PATHS_SEP: &str = ";";
#[cfg(not(windows))]
const PATHS_SEP: &str = ":";

fn main() {
    if cfg!(feature = "invocation") {
        let libjvm_path = env::var("JAVA_HOME").ok()
            .and_then(find_libjvm_in_java_home)
            .or_else(find_java_home)
            .and_then(find_libjvm_in_java_home)
            .or_else(find_libjvm_in_library_paths)
            .unwrap_or_else(|| panic!("Failed to find {}. Try setting JAVA_HOME", LIBJVM_NAME));

        println!("cargo:rustc-link-search=native={}", libjvm_path.display());
        println!("cargo:rustc-link-lib=dylib=jvm");
    }
}

fn find_libjvm_in_java_home<S: AsRef<Path>>(path: S) -> Option<PathBuf> {
    let walker = walkdir::WalkDir::new(path).follow_links(true);
    let libjvm_name = ffi::OsString::from(LIBJVM_NAME);

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_e) => continue,
        };

        if entry.file_name() == libjvm_name {
            return entry.path().parent().map(Into::into);
        }
    }

    None
}

#[cfg(windows)]
fn find_libjvm_in_library_paths() -> Option<PathBuf> {
    env::var("PATH").ok()
        .and_then(check_lib_paths)
}

#[cfg(all(unix, any(target_os = "macos", target_os = "ios")))]
fn find_libjvm_in_library_paths() -> Option<PathBuf> {
    env::var("LD_LIBRARY_PATH").ok()
        .and_then(check_lib_paths)
        .or_else(|| {
            env::var("DYLD_LIBRARY_PATH").ok()
                .and_then(check_lib_paths)
        })
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
fn find_libjvm_in_library_paths() -> Option<PathBuf> {
    env::var("LD_LIBRARY_PATH").ok()
        .and_then(check_lib_paths)
}

fn check_lib_paths<P: AsRef<str>>(paths: P) -> Option<PathBuf> {
    paths.as_ref().split(PATHS_SEP)
        .filter(|p| is_jvm_lib_path(p))
        .flat_map(follow_symlinks)
        .next()
}

fn is_jvm_lib_path<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().join(LIBJVM_NAME).is_file()
}

fn find_java_home() -> Option<PathBuf> {
    env::var("PATH").ok()
        .and_then(|path_var| {
            path_var.split(PATHS_SEP)
                .filter_map(java_home_if_exe_path)
                .next()
        })
}

fn java_home_if_exe_path<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    let path = path.as_ref();
    if !path.join(JAVA_EXE_NAME).is_file() {
        return None;
    }
    follow_symlinks(path)
        .and_then(|path| {
            path.parent()
                .and_then(|p| p.parent())
                .map(Into::into)
        })
}

fn follow_symlinks<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    let path = path.as_ref();
    fs::symlink_metadata(path)
        .and_then(|mut meta| {
            let mut path = PathBuf::from(path);
            while meta.file_type().is_symlink() {
                meta = fs::symlink_metadata(&path)?;
                path = fs::read_link(&path)?;
            };
            Ok(path)
        })
        .ok()
}
