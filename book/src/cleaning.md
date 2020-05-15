# TODO Cleaning up Resource Leaks

Since Java doesn’t automatically free the memory on Rust’s heap when the
`Counter` garbage collected, there could be a memory leak. Many native resources
need similar treatment, and so we are going to automate it using the [Cleaner
API](https://docs.oracle.com/javase/9/docs/api/java/lang/ref/Cleaner.html),
which requires a design change. All the native state will be stored within an
object inside of Counter, which will be registered to the Cleaner, together with
a cleaning action.

TODO: At minimum, we need to discuss setup for the cleaner, and best practices
around cleaner initialization. We also need to describe finalizers and why they
usually are inappropriate, or provide a reference for the argument.

```java
import java.lang.ref.Cleaner;

class Counter implements AutoCloseable {
    private static final Cleaner CLEANER = NativeAPI.getCleaner();
    private final State state;
    private final Cleaner.Cleanable cleanable;

    static class State implements Runnable {
        private long ptr;

        public State() {
            this.ptr = NativeAPI.counter_new();
        }

        public void run() {
            NativeAPI.counter_destroy(ptr);
        }

        public int get() {
            return NativeAPI.counter_get(ptr);
        }

        public int increment() {
            return NativeAPI.counter_increment(ptr);
        }
    }

    public Counter() {
        this.state = new State();
        this.cleanable = CLEANER.register(this, state);
    }

    public int get() {
        return this.state.get();
    }

    public int increment() {
        return this.state.increment();
    }

    public void close() {
        cleanable.clean();
    }
}
```


