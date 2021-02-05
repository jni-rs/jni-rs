# Chart Solution
We'll define the Java native method as follows:

```Java
class NativeAPI {
{{#include ../projects/completed/jnibookgradle/src/main/java/jni_rs_book/NativeAPI.java:chart}}
}
```

Then we implement the `ChartRenderer` class:

```java
{{#include ../projects/completed/jnibookgradle/src/main/java/jni_rs_book/ChartRenderer.java:complete}}
```

Since we're already working in Java, we'll write a sanity test for it as
follows:

```java
public class ChartRendererTest {

    @Test
    public void testRenderChart() {
        String s = ChartRenderer.renderChart(80, 50, "Spaceship Number",
                0, 5,
                new double[]{1.0, 2.0, 3.0, 4.0, 5.0},
                "Launch Time in Hours",
                0, 5,
                new double[]{3.0, 2.0, 5.0, 4.0, 1.0});
        System.out.println(s);
    }
}
```

Next, we implement the native side. The wrapper copies the points out of the
Java arrays and into the types expected by `render_chart`.

```rust,noplaypen
{{#include ../projects/completed/jnibookrs/src/charts.rs:complete}}
```

When check our test output, we see these results:

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

In this exercise, we rendered a chart using Rust code and data from Java arrays
and Strings. Along the way, used the `try_java` function from [Error
handling](./error_handling.md) to handle possible errors, and created a new
String from native code.

An alternative implementation could instead leverage [JNI critical
APIs](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/functions.html#GetPrimitiveArrayCritical_ReleasePrimitiveArrayCritical),
which can sometimes avoid array copies. (jni-rs also has
[`AutoPrimitiveArray`](https://docs.rs/jni/0.18.0/jni/struct.JNIEnv.html#method.get_auto_primitive_array_critical),
which ensures that resources are released on `drop`.)

