# Chart Hint
One potential solution does the following:

1. Wrap `render_chart` with a JNI style Rust function.
2. Read Java Strings into Rust Strings.
3. Read primitive arrays into `Vec<f64>`, then combine them into `Vec<(f64, f64)>`.
4. Either pass array length down from Java, or use `get_array_length` to read it
   from Rust.
5. Create a new Java String, from a successful `render_chart`.
6. Handle errors from `render_chart` with `try_java` from [error
   handling](./error_handling.md).

