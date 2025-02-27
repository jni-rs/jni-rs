// Bindings generated by `windows-bindgen` 0.59.0

#![allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    dead_code,
    clippy::all
)]

windows_targets::link!("kernel32.dll" "system" fn GetACP() -> u32);
windows_targets::link!("kernel32.dll" "system" fn MultiByteToWideChar(codepage : u32, dwflags : MULTI_BYTE_TO_WIDE_CHAR_FLAGS, lpmultibytestr : PCSTR, cbmultibyte : i32, lpwidecharstr : PWSTR, cchwidechar : i32) -> i32);
windows_targets::link!("kernel32.dll" "system" fn WideCharToMultiByte(codepage : u32, dwflags : u32, lpwidecharstr : PCWSTR, cchwidechar : i32, lpmultibytestr : PSTR, cbmultibyte : i32, lpdefaultchar : PCSTR, lpuseddefaultchar : *mut BOOL) -> i32);
pub type BOOL = i32;
pub const CP_UTF7: u32 = 65000u32;
pub const CP_UTF8: u32 = 65001u32;
pub type MULTI_BYTE_TO_WIDE_CHAR_FLAGS = u32;
pub type PCSTR = *const u8;
pub type PCWSTR = *const u16;
pub type PSTR = *mut u8;
pub type PWSTR = *mut u16;
pub const WC_COMPOSITECHECK: u32 = 512u32;
pub const WC_NO_BEST_FIT_CHARS: u32 = 1024u32;

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
