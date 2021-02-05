# Chart Exercise
Next, we'll begin manipulating strings and primitive arrays. You'll write JNI
code to wrap a Chart Plotting library - which will give you an opportunity to
work with more Java types from Rust.

## Goal

Implement a class called `ChartRenderer`. It should delegate to a native method
for rendering the chart.

```Java
package jni_rs_book;

public class ChartRenderer {

    private ChartRenderer() {}

    public static String renderChart(/* Various parameters for the drawing the chart */) {
    
    }
}
```

The Rust function that you should figure out how to wrap is given to you below,
and is also present in the starter code:

```rust,noplaypen
{{#include ../projects/starter/jnibookrs/src/charts.rs}}
```

To make progress, first try working with the suggested docs. If that fails, then
refer to [the hints](./chart_hint.md), or [a solution](./chart_solution.md).

## Docs to get you started
These are some selections from the `jni-rs` docs that may help you.

### Reading Java Array Lengths
[`JNIEnv::get_array_length(&self, array: jarray) ->
Result<jsize>`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.get_array_length)
can be used to read the length of Java Arrays.

```rust,noplaypen
    let size = env.get_array_length(my_array)? as usize;
```

### Reading Java Primitive Arrays
[`JNIEnv::get_double_array_region(&self, array: jdoubleArray, start: jsize, buf:
&mut [jdouble]) ->
Result<()>`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.get_double_array_region)
can be used to copy doubles out of a Java array, and into a mutable slice. Note
that there are similar APIs for other primitive types (e.g., `int`, `long`,
etc.).

### Reading Java Strings
[`JNIEnv::get_string(&self, obj: JString<'a>) -> Result<JavaStr<'a,
'_>>`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.get_string) can
be used to get a Rust String. [As described in the
docs](https://docs.rs/jni/0.18.0/jni/strings/struct.JavaStr.html), it
automatically releases the String on `drop`.

```rust,noplaypen
let my_string: String = env.get_string(java_string)?.into();
```

### Creating Java Strings
[`JNIEnv::new_string<S: Into<JNIString>>(&self, from: S) ->
Result<JString<'a>>`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.new_string)
can be used to create a Java String from a Rust `&str`.

### JString and jstring

See [the JString
docs](https://docs.rs/jni/0.18.0/jni/objects/struct.JString.html) to understand
the difference between the two.
