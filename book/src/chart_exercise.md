# Chart Exercise

Next, we'll begin manipulating strings and primitive arrays. You'll write JNI
code to wrap a Chart Plotting library - which, along the way, copies primitive
arrays and Strings, and return a new Java String.

## Goal

The target Java usage appears as:

```Java
package jni_rs_book;

public class ChartRenderer {

    public String renderChart(/* Various parameters for the drawing the chart */) {
    
    }
}
```

And the Rust function that you should figure out how to wrap is:

```rust,noplaypen
/// Given various data, render a Chart as a String. 
/// Upon failure, return Error.
fn render_chart(
    width: u32,
    height: u32,
    x_label: &str,
    y_label: &str,
    x_start: f64,
    x_end: f64,
    y_start: f64,
    y_end: f64,
    data: Vec<(f64, f64)>,
) -> Result<String, anyhow::Error> {
    use plotlib::page::Page;
    use plotlib::repr::Plot;
    use plotlib::style::{PointMarker, PointStyle};
    use plotlib::view::ContinuousView;

    let plot = Plot::new(data).point_style(PointStyle::new().marker(PointMarker::Circle));

    let v = ContinuousView::new()
        .add(plot)
        .x_range(x_start, x_end)
        .y_range(y_start, y_end)
        .x_label(x_label)
        .y_label(y_label);

    Ok(Page::single(&v)
        .dimensions(width, height)
        .to_text()
        .map_err(|err| err.compat())?)
}
```

If you get stuck, try working with the docs, or the next page for hints, or the
next-next page for a solution.

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
be used to get a Rust String. [As described in the docs](https://docs.rs/jni/0.18.0/jni/strings/struct.JavaStr.html), it automatically releases the String on `drop`.

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
