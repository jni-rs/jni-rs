# jbindgen

A library and CLI tool for generating Rust JNI bindings by parsing Java APIs.

`jbindgen` parses Java source code or bytecode and generates safe Rust JNI bindings compatible with
the `jni` crate.

## Features

- Parse `.java` source files
  - Uses an embedded Java compiler, based on `javax.tools.JavaCompiler` API (not shelling out to
    `javac`)
  - Supports parsing source code within a .jar file (such as Android SDK source stubs)
  - Supports `@Deprecated` annotations
  - Supports `@RustName` annotation for overriding generated Rust names from Java source code
  - Supports extracting Javadoc comments for documentation
- Parse `.class` files and `.jar` files containing compiled bytecode
  - Uses the `cafebabe` crate for parsing Java class files
- Android SDK bindings support
  - Parses both `android.jar` (bytecode) and `android-stubs-src.jar` (source stubs) to get complete
    public API surface
  - Supports filtering using `hiddenapi-flags.csv` to exclude hidden/non-public APIs
- Generates `jni::bind_java_type!` based JNI bindings
  - Supports constructors, methods, native methods and fields
  - Supports calling and/or implementing native methods (generates safe trait for implementation of
    native methods)
  - Supports casting to superclasses and interfaces
  - Supports static and instance methods/fields
  - Fields that are `final` in Java are generated as read-only in Rust
  - Automatically gives overloaded methods unique Rust names by appending arity and parameter-type
    suffixes
  - Warns about non-overloaded methods that clash (e.g. `toUri` vs `toURI` both want to map to
    `to_uri`)
  - Explicitly skip specific methods/fields
  - Override the default method / field names in Rust
- Supports linking with existing bindings by providing your own list of Rust -> Java type mappings
- Generates `jni_init` functions for each module to pre-load and cache class references and
  method/field IDs and register native method implementations
  - Lets you explicitly control when bindings are initialized at runtime (alternative to lazy
    initialization)
  - Lets you specify a custom `ClassLoader` for loading classes
  - Enables runtime testing of binding correctness (since it will error if any class/method/field is
    missing or has an incorrect signature)
- API for integration into build scripts or other tools
  - `Builder` API for configuring binding generation options programmatically
  - `Bindings` output that gives control over how to write the generated bindings
    - Generate a single file with all bindings
    - Generate a module tree (based on the Java package hierarchy) with one file per class
- `jbindgen` CLI tool enables you to generate bindings ahead of time, instead of at build time

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
jbindgen = "0.1.0"
```

Or install the CLI tool:

```bash
cargo install jbindgen
```

## Library Usage

```rust
//! An example build script that generates JNI bindings using jbindgen.
use jbindgen::Builder;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR")?;
    let bindings_path = PathBuf::from(out_dir).join("bindings.rs");

    let bindings = Builder::new()
        .input_class("src/java/com/example/MyClass.class")
        .generate()?;

    bindings.write_to_file(&bindings_path)?;

    // In your lib.rs or main.rs, include the generated bindings:
    // include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    Ok(())
}
```

## CLI Usage

Basic usage:

```bash
# Generate bindings to stdout (assuming bindings will be imported under `crate` root in lib.rs or main.rs)
jbindgen classfile MyClass.class

# Write bindings to a file
jbindgen classfile -o bindings.rs MyClass.class

# Specify a custom root module path
jbindgen classfile --root crate::bindings MyClass.class

# Use a custom Rust type name
jbindgen classfile --rust-name MyCustomType MyClass.class

# Generate a private type
jbindgen classfile --private MyClass.class
```

### JAR Files

Generate bindings for all classes in a JAR:

```bash
# Output to stdout (all classes concatenated)
jbindgen classfile mylib.jar

# Filter by package prefix
jbindgen classfile --pattern com.example mylib.jar

# Write each class to a separate file in a directory
jbindgen classfile --output-dir bindings/ mylib.jar
```

### Java Source Files

Generate bindings from Java source files:

```bash
# Generate bindings for a specific Java source file (assuming bindings will be imported under `crate` root in lib.rs or main.rs)
jbindgen java MyClass.java

# Specify a custom root module path
jbindgen java --root crate::bindings::jni src/MyClass.java
```

### Android SDK Bindings

Generate bindings from the Android SDK:

```bash
# Generate bindings for a specific Android class (assuming bindings will be imported under `crate` root in lib.rs)
jbindgen android --api-level 35 --pattern android.app.Activity

# Generate bindings for multiple classes using wildcards
jbindgen android --api-level 35 --pattern 'android.app.*'

# Save to a directory (one file per class)
jbindgen android --api-level 35 --output-dir bindings/ --pattern 'android.os.*'

# Save to a single file
jbindgen android --api-level 35 --output-file activity.rs --pattern android.app.Activity

# Filter out hidden/non-public APIs using hiddenapi-flags.csv
jbindgen android --api-level 35 --pattern 'android.os.*' \
    --hiddenapi-flags /path/to/hiddenapi-flags.csv \
    --output-dir bindings/
```

**Requirements for Android SDK support:**
- Set `ANDROID_HOME` or `ANDROID_SDK_ROOT` environment variable
- Install the Android SDK for the target API level
- Download Android SDK Sources for the API level (required for parsing source stubs)

**How it works:**

Android bindings are generated through a multi-stage filtering process:

1. Parse `android.jar` (compiled bytecode) for the complete implementation
2. Parse `android-stubs-src.jar` (source stubs) for the public API surface
3. Intersect the two, keeping only APIs present in both
4. Optionally filter using `hiddenapi-flags.csv` to exclude hidden/non-public APIs

This ensures bindings only include APIs that are both implemented and officially public.

**Example:**
```bash
export ANDROID_HOME=/path/to/android-sdk
jbindgen android --api-level 35 --output-dir android-bindings/ --pattern android.os.Build
```

This will generate Rust bindings for all classes in the `android.app` package.

## Example

Given a Java class:

```java
package com.example;

public class Calculator {
    public Calculator() {}

    public static int add(int a, int b) {
        return a + b;
    }

    public int square(int x) {
        return x * x;
    }
}
```

Generate bindings:

```bash
jbindgen java com/example/Calculator.java
```

Output:

```rust
// This file was generated by jbindgen. Do not edit manually.

mod com {
    mod example {
        use jni::bind_java_type;

        bind_java_type! {
            /// Test class with various method signatures.
            pub Calculator => "com.example.Calculator",
            type_map = {
                Calculator => "com.example.Calculator",
            },
            constructors {
                fn new(),
            },
            methods {
                static fn add(a: jint, b: jint) -> jint,
                fn square(x: jint) -> jint,
            },
        }

        /// Initialize all Java bindings in this module.
        ///
        /// This loads and caches JClass references and method/field IDs for all
        /// bindings in this module and its child modules.
        ///
        /// # Arguments
        ///
        /// * `env` - The JNI environment
        /// * `loader` - The LoaderContext to use for loading classes
        pub fn jni_init(env: &jni::Env, loader: &jni::LoaderContext) -> jni::errors::Result<()> {
            let _ = CalculatorAPI::get(env, loader)?;
            Ok(())
        }
    }

    /// Initialize all Java bindings in this module.
    ///
    /// This loads and caches JClass references and method/field IDs for all
    /// bindings in this module and its child modules.
    ///
    /// # Arguments
    ///
    /// * `env` - The JNI environment
    /// * `loader` - The LoaderContext to use for loading classes
    pub fn jni_init(env: &jni::Env, loader: &jni::LoaderContext) -> jni::errors::Result<()> {
        example::jni_init(env, loader)?;
        Ok(())
    }
}
```

### Using the Generated Bindings

```rust
use jni::JavaVM;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the JVM
    let jvm = JavaVM::new(jni::InitArgsBuilder::new().build()?)?;
    let mut env = jvm.attach_current_thread()?;

    // Option 1: Use bindings directly (lazy initialization)
    let calc = com::example::Calculator::new(&mut env)?;
    let result = calc.square(&mut env, 5)?;
    println!("5 squared = {}", result);

    // Option 2: Pre-initialize all bindings for better performance
    let loader = jni::LoaderContext::system();
    com::jni_init(&env, &loader)?;

    // Now all classes/methods are cached for faster access
    let sum = com::example::Calculator::add(&mut env, 3, 4)?;
    println!("3 + 4 = {}", sum);

    Ok(())
}
```

The `jni_init` functions allow you to:
- Explicitly control when to load and cache classes + method/field IDs
- Use a specific `ClassLoader` that your bindings require
- Bulk initialize bindings for runtime testing of binding correctness
- Improve performance by pre-caching all JNI metadata at startup
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
