# Counter Exercise

This chapter introduces the techniques for exposing a Rust-backed Counter
through JNI. Applying JNI in this manner is guaranteed to be slower and more
complicated than a pure Java solution, but it can be a helpful learning
exercise. An ordinary Java Counter might be implemented like this:

```java
class Counter {
    int count;

    public Counter() {}
    
    // Get the current count
    public int get() {
        return count;
    }

    // Increment and return the new count
    public int increment() {
        return ++count;
    }
}
```

Implement a Counter in Rust, and wrap it using Java. The implementation doesn't
need to be safe to use across different threads, or safe from double frees in
`close()`, or use-after-free, since the purpose is to get started. We will correct these problems in future exercises.
