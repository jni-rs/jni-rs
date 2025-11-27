#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
    t.compile_fail("tests/ui/bind_java_type/fail/*.rs");
    t.pass("tests/ui/bind_java_type/pass/*.rs");
    t.compile_fail("tests/ui/native_method/fail/*.rs");
}
