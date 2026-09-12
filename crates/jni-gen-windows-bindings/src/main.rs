use windows_bindgen::Bindgen;

fn main() {
    Bindgen::new()
        .output("../jni/src/windows_sys.rs")
        .flat()
        .sys()
        .filter("WC_COMPOSITECHECK")
        .filter("WC_NO_BEST_FIT_CHARS")
        .filter("CP_UTF7")
        .filter("CP_UTF8")
        .filter("WideCharToMultiByte")
        .filter("MultiByteToWideChar")
        .filter("GetACP")
        .filter("FlsAlloc")
        .filter("FlsSetValue")
        .filter("FlsGetValue")
        .write();
}
