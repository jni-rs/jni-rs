windows_link::link!("kernel32.dll" "system" fn FlsAlloc(lpcallback : PFLS_CALLBACK_FUNCTION) -> u32);
windows_link::link!("kernel32.dll" "system" fn FlsGetValue(dwflsindex : u32) -> *mut core::ffi::c_void);
windows_link::link!("kernel32.dll" "system" fn FlsSetValue(dwflsindex : u32, lpflsdata : *const core::ffi::c_void) -> BOOL);
windows_link::link!("kernel32.dll" "system" fn GetACP() -> u32);
windows_link::link!("kernel32.dll" "system" fn MultiByteToWideChar(codepage : u32, dwflags : u32, lpmultibytestr : *const i8, cbmultibyte : i32, lpwidecharstr : PWSTR, cchwidechar : i32) -> i32);
windows_link::link!("kernel32.dll" "system" fn WideCharToMultiByte(codepage : u32, dwflags : u32, lpwidecharstr : *const u16, cchwidechar : i32, lpmultibytestr : PSTR, cbmultibyte : i32, lpdefaultchar : *const i8, lpuseddefaultchar : *mut BOOL) -> i32);
pub type BOOL = i32;
pub const CP_UTF7: i32 = 65000;
pub const CP_UTF8: i32 = 65001;
pub type PFLS_CALLBACK_FUNCTION =
    Option<unsafe extern "system" fn(lpflsdata: *const core::ffi::c_void)>;
pub type PSTR = *mut u8;
pub type PWSTR = *mut u16;
pub const WC_COMPOSITECHECK: i32 = 512;
pub const WC_NO_BEST_FIT_CHARS: i32 = 1024;
