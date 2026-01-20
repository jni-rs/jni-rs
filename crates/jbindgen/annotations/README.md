# JBindgen Annotations

Annotation library for customizing Rust bindings generation with [jbindgen](https://github.com/jni-rs/jni-rs/tree/master/crates/jbindgen).

## Overview

This library provides annotations that can be used in Java source code to control how jbindgen generates Rust bindings. The annotations are processed at binding generation time and do not introduce any runtime dependencies.

## Installation

You have two options for using the jbindgen annotations:

### Option 1: Maven Central (Recommended)

Add the dependency to your project:

#### Maven

```xml
<dependency>
    <groupId>io.github.jni-rs</groupId>
    <artifactId>jbindgen-annotations</artifactId>
    <version>0.1.0</version>
    <scope>provided</scope>
</dependency>
```

#### Gradle

```gradle
compileOnly 'io.github.jni-rs:jbindgen-annotations:0.1.0'
```

**Note:** Use `compileOnly` or `provided` scope since these annotations are only needed at binding generation time, not at runtime.

### Option 2: Vendor the Annotation Files

If you prefer not to add a Maven dependency, you can vendor the annotation source files directly into your project using the jbindgen CLI:

```bash
# Generate annotation files in your project's source directory
jbindgen annotations --output src/main/java

# Or in the current directory
jbindgen annotations
```

This will create the following structure:
```
io/github/jni_rs/jbindgen/
├── RustName.java
└── package-info.java
```

You can then commit these files to your repository and use them without any external dependencies.

## Available Annotations

### `@RustName`

Override the generated Rust name for classes, methods, fields, and constructors.

```java
import io.github.jni_rs.jbindgen.RustName;

// Override class name
@RustName("CustomClassName")
public class MyClass {

    // Override field name
    @RustName("custom_field")
    public String someField;

    // Override constructor name
    @RustName("new_with_config")
    public MyClass(Config config) { }

    // Override method name
    @RustName("custom_method")
    public void someMethod() { }

    // Override native method name
    @RustName("native_callback")
    public native void onCallback();
}
```

**Naming Conventions:**
- Types: Use `PascalCase` (e.g., `"MyType"`)
- Methods and fields: Use `snake_case` (e.g., `"my_method"`)
- Constructors: Typically `"new"` or `"new_*"` (e.g., `"new_with_options"`)

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](../../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](../../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

See the main [jni-rs contributing guide](../../../CONTRIBUTING.md).
