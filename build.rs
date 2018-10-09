extern crate walkdir;

#[path = "src/wrapper/platform.rs"]
mod platform;

use std::env;
use std::ffi;
use std::fs;
use std::path::{
    Path,
    PathBuf,
};
use std::process::Command;

use platform::{
    LIBJVM_NAME,
    PATHS_SEP,
};

fn main() {
    if cfg!(feature = "invocation") {
        let (java_home, libjvm_path) = env::var("JAVA_HOME").ok()
            .and_then(find_libjvm_in_java_home)
            .or_else(|| {
                ask_java_home()
                    .and_then(find_libjvm_in_java_home)
            })
            .map_or((None, None), |found| (Some(found.0), Some(found.1)));

        let libjvm_path = libjvm_path
            .or_else(find_libjvm_in_library_paths)
            .unwrap_or_else(|| panic!("Failed to find {}. Try setting JAVA_HOME", LIBJVM_NAME));

        println!("cargo:rustc-link-search=native={}", libjvm_path.display());
        println!("cargo:rustc-link-lib=dylib=jvm");
        if cfg!(windows) {
            if let Some(java_home) = java_home {
                let lib_path = java_home.join("lib");
                println!("cargo:rustc-link-search={}", lib_path.display());
            } else if let Some(lib_path) = suggest_lib_from_libjvm(libjvm_path) {
                println!("cargo:rustc-link-search={}", lib_path.display());
            } else {
                println!("Failed to set `jdk/lib` search path. \
                    If linker failed, try setting JAVA_HOME.");
            }
        }
    }
}

fn find_libjvm_in_java_home<S: AsRef<Path>>(path: S) -> Option<(PathBuf, PathBuf)> {
    let walker = walkdir::WalkDir::new(path.as_ref()).follow_links(true);
    let libjvm_name = ffi::OsString::from(LIBJVM_NAME);

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_e) => continue,
        };

        if entry.file_name() == libjvm_name {
            return entry.path().parent()
                .map(|libjvm_path| (path.as_ref().into(), libjvm_path.into()));
        }
    }

    None
}

#[cfg(windows)]
fn suggest_lib_from_libjvm<S: Into<PathBuf>>(path: S) -> Option<PathBuf> {
    let mut path = path.into();
    while let Some(parent) = path.parent().map(ToOwned::to_owned) {
        path = parent.join("lib");
        if path.join("jvm.lib").exists() {
            return Some(path);
        }
    }
    None
}

#[cfg(not(windows))]
fn suggest_lib_from_libjvm<S: Into<PathBuf>>(_path: S) -> Option<PathBuf> {
    unimplemented!("This path is needed only on Windows")
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

fn ask_java_home() -> Option<PathBuf> {
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
