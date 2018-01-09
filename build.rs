extern crate walkdir;

use std::env;
use std::fs::symlink_metadata;
use std::path::{
    Path,
    PathBuf,
};

fn main() {
    if cfg!(feature = "invocation") {
        let libjvm_path = env::var("JAVA_HOME")
            .ok()
            .and_then(|p| find_libjvm(p))
            .or_else(|| find_java_home().and_then(|p| find_libjvm(p)));

        match libjvm_path {
            Some(path) => println!("cargo:rustc-link-search=native={}", path.display()),
            None => panic!("Failed to find libjvm.so. Try setting JAVA_HOME"),
        }

        println!("cargo:rustc-link-lib=dylib=jvm");
    }
}

fn find_libjvm<S: AsRef<Path>>(path: S) -> Option<PathBuf> {
    let walker = walkdir::WalkDir::new(path).follow_links(true);

    let expected_name = if cfg!(target_os = "windows") {
        "jvm.dll"
    } else if cfg!(target_os = "linux") {
        "libjvm.so"
    } else {
        "libjvm.dylib"
    };

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_e) => continue,
        };

        let file_name = entry.file_name().to_str().unwrap_or("");

        if file_name == expected_name {
            return entry.path().parent().map(Into::into);
        }
    }

    None
}

fn find_java_home() -> Option<PathBuf> {
    let path = env::var("PATH").ok()?;

    let path_sep = if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    };

    let paths = path.split(path_sep);
    let (mut exe_path, mut exe_meta): (PathBuf, _) = paths
        .filter_map(|p| symlink_metadata(p).map(|m| (p.into(), m)).ok())
        .nth(0)?;

    while exe_meta.file_type().is_symlink() {
        exe_path = ::std::fs::read_link(&exe_path).ok()?;
        exe_meta = symlink_metadata(&exe_path).ok()?;
    }

    exe_path.parent().and_then(|p| p.parent()).map(Into::into)
}
