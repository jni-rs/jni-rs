# Counter Exercise

An ordinary Java Counter might be implemented like this:

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

Implement a similar Counter in Rust thats owned by Java. Applying JNI in this
manner is guaranteed to be slower and more complicated than a pure Java
solution. These are the problems that you should think about:

1. allocation: How do you allocate the counter, and transfer ownership to Java?
2. closing: How do you deallocate the counter? Does it protect from
   use-after-free, or double free? What happens if the caller forgets to call
   `close()`?
3. thread-safety: Can the counter safely be used from multiple threads?
