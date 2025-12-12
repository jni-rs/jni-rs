fn main() {
    windows_bindgen::bindgen(&[
        "--out",
        "../jni/src/windows_sys.rs",
        "--etc",
        "./bindings.config",
    ])
    .unwrap();
}
