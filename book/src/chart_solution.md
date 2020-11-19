# Chart Solution
We'll define the Java native method as follows:

```Java
class NativeAPI {

    static native String draw_points_plaintext(long chartWidth, long chartHeight, 
        int arrayLength,
        String xTitle, double xStart, double xEnd, double[] xs,
        String yTitle, double yStart, double yEnd, double[] ys);
}
```

Note that this API expects array lengths as part of the native call, so that we
don't need to use `get_array_length`. Then, we implement the `ChartRenderer`
class:

```java
public class ChartRenderer {

    public String renderChart(long chartWidth, long chartHeight,
                       String xTitle, double xStart, double xEnd, double[] xs,
                       String yTitle, double yStart, double yEnd, double[] ys) {
        if (xs.length != ys.length) {
            throw new IllegalArgumentException("xs and ys array lengths must match");
        }
        return NativeAPI.draw_points_plaintext(chartWidth, chartHeight, xs.length,
                xTitle, xStart, xEnd, xs,
                yTitle, yStart, yEnd, ys);
    }
}
```

Next, we implement the native side. The wrapper copies the points out of the
Java arrays, and into the format expected by `render_chart`.

```rust,noplaypen
#[no_mangle]
pub extern "system" fn Java_jni_1rs_1book_NativeAPI_draw_1points_1plaintext(
    env: JNIEnv,
    _class: JClass,
    chart_width: jint,
    chart_height: jint,
    arr_length: jint,
    java_x_title: JString,
    x_start: jdouble,
    x_end: jdouble,
    java_xs: jdoubleArray,
    java_y_title: JString,
    y_start: jdouble,
    y_end: jdouble,
    java_ys: jdoubleArray,
) -> jstring {
    try_java(env, std::ptr::null_mut(), || {
        let mut xs = vec![0.0; arr_length];
        // Copy the xs
        env.get_double_array_region(java_xs, 0, &mut xs)?;

        let mut ys = vec![0.0; arr_length];
        // Copy the ys
        env.get_double_array_region(java_ys, 0, &mut ys)?;

        // Get the x and y titles...
        let x_title: String = env.get_string(java_x_title)?.into();
        let y_title: String = env.get_string(java_y_title)?.into();

        // Copy the doubles into the format that the API we're wrapping expects
        let points: Vec<(f64, f64)> = xs.into_iter().zip(ys.into_iter()).collect();
        let output: String = render_chart(
            width as u32,
            height as u32,
            &x_title,
            &y_title,
            x_start,
            x_end,
            y_start,
            y_end,
            points,
        )?;
        // Finally, create and return the Java String - 
        // using the non-lifetimed representation 
        Ok(env.new_string(&output)?.into_inner())
    })
}
```

An alternative implementation could instead leverage [JNI critical
APIs](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#GetPrimitiveArrayCritical_ReleasePrimitiveArrayCritical),
which can sometimes avoid array copies. (jni-rs has
[`AutoPrimitiveArray`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.get_auto_primitive_array_critical),
which ensures that resources are released on `drop`.)

Finally, we pass test data from Java, to check that the render succeeds.

```java
public class ChartRendererTest {

    @Test
    public void testRenderChart() {
        ChartRenderer chartRenderer = new ChartRenderer();
        String s = chartRenderer.renderChart(80, 50, "Spaceship Number",
                0, 5,
                new double[]{1.0, 2.0, 3.0, 4.0, 5.0},
                "Launch Time in Hours",
                0, 5,
                new double[]{3.0, 2.0, 5.0, 4.0, 1.0});
        System.out.println(s);
    }
}
```

Then, we get these results:

```
   5-|                                              ●                                
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
   4-|                                                              ●                
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
 L   |                                                                               
 a   |                                                                               
 u   |                                                                               
 n   |                                                                               
 c   |                                                                               
 h 3-|              ●                                                                
     |                                                                               
 T   |                                                                               
 i   |                                                                               
 m   |                                                                               
 e   |                                                                               
     |                                                                               
 i   |                                                                               
 n   |                                                                               
     |                                                                               
 H 2-|                              ●                                                
 o   |                                                                               
 u   |                                                                               
 r   |                                                                               
 s   |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
   1-|                                                                              ●
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
     |                                                                               
   0+-------------------------------------------------------------------------------- 
    |               |               |               |               |               | 
    0               1               2               3               4               5 
                                    Spaceship Number                                  
```

# Summary

In this exercise, we passed Java arrays and Strings down to Rust for chart
rendering. Along the way, we copied data out of them and used the `try_java`
function from [Error handling](./error_handling.md) to handle possible errors, and returned a new String.
