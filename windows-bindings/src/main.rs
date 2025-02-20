// XXX: Once the `windows-link` releases it would be good to use.
// This slightly diverges from the upstream because of a current `jni` MSRV of 1.77 and because
// the name `link` is too ambiguous for Rust to allow re-exporting.
const LINKAGE: &[u8] = br#"
// jni-rs specific additions vendored from https://github.com/microsoft/windows-rs/blob/master/crates/libs/link/src/lib.rs
mod windows_targets {
    #[cfg(all(windows, target_arch = "x86"))]
    macro_rules! win_link {
        ($library:literal $abi:literal $($link_name:literal)? fn $($function:tt)*) => (
            #[link(name = $library, kind = "raw-dylib", modifiers = "+verbatim", import_name_type = "undecorated")]
            extern $abi {
                $(#[link_name=$link_name])?
                pub fn $($function)*;
            }
        )
    }
    #[cfg(all(windows, not(target_arch = "x86")))]
    macro_rules! win_link {
        ($library:literal $abi:literal $($link_name:literal)? fn $($function:tt)*) => (
            #[link(name = $library, kind = "raw-dylib", modifiers = "+verbatim")]
            extern $abi {
                $(#[link_name=$link_name])?
                pub fn $($function)*;
            }
        )
    }
    pub(super) use win_link as link;
}
"#;

use std::io::Write;
fn main() {
    let out_file = "../src/wrapper/windows_sys.rs";
    windows_bindgen::bindgen(&["--out", out_file, "--etc", "./bindings.config"]);
    let mut bindings = std::fs::File::options()
        .append(true)
        .open(out_file)
        .unwrap();
    bindings.write_all(LINKAGE).unwrap();
}
