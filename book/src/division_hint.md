# Division Hint

One approach relies on these steps:

1. Use `javac` to generate the function name.
2. Decorate the function with `#[no_mangle]` and `pub extern "system"`.
3. Add parameters for dividing. The type `jni::sys::jint` may help.
4. Remember to include the first and second parameters, and to test the solution.
5. If an `UnsatisfiedLinkError` Exception is thrown, ensure that you've
   configured `java.library.path`.
