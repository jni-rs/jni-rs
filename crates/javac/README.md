# javac

A library for [Cargo build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
to compile Java source files into `.class` files. This crate provides a simple
builder API similar to the `cc` crate, making it easy to compile Java code as
part of your Rust build process.

This crate does not compile code itself; it calls out to the `javac` compiler
on your system (located via `JAVA_HOME` or `PATH`). It will automatically
handle cross-platform differences and properly encode file paths.

## Usage

First, add `javac` as a build dependency in your `Cargo.toml`:

```toml
[build-dependencies]
javac = "0.1"
```

Then, in your `build.rs`:

```rust
fn main() {
    javac::Build::new()
        .file("java/com/example/HelloWorld.java")
        .compile();
}
```

For more complex scenarios:

```rust
fn main() {
    javac::Build::new()
        .files(&["java/Foo.java", "java/Bar.java"])
        .source_dir("java")  // Recursively compile all .java files
        .classpath("lib/dependency.jar")
        .release("11")  // Java 11 compatibility
        .encoding("UTF-8")
        .debug(true)
        .compile();
}
```

The compiled `.class` files will be placed in `$OUT_DIR/javac-build/classes/`
by default, or you can specify a custom output directory with `.output_dir()`.

## Requirements

- A Java Development Kit (JDK) with `javac` must be installed
- The `javac -verbose` output must be compatible with the OpenJDK compiler which includes
  `[wrote /path/to/Name.class]` lines, in order to track what `.class` files are written by
  the compiler.
- Either `JAVA_HOME` environment variable must be set, or `javac` must be in `PATH`

## Documentation

Refer to the [documentation](https://docs.rs/javac) for detailed API documentation.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
