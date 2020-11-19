# Division Exercise

Now that you've seen an example applied via `verify_link`, it's time to get
familiar with writing your own JNI Rust code. Add a function to `NativeAPI` with
this signature:

```java 
static native int divide(int a, int b);
```

Your goal is to implement this method in Rust. For hints, check the next page.
Don't worry about division by 0, since we'll discuss a few ways to address that
in [Error Handling](./error_handling.md).
