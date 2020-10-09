# Debugging

Division gives us a perfect opportunity to exercise debugging skills. Write a
test in `jnibookjava` that divides by zero, then run it. It's likely that the
JVM will crash and you'll see a message similar to this:

```
``thread '<unnamed>' panicked at 'attempt to divide by zero', src/division.rs:14:5
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
fatal runtime error: failed to initiate panic, error 5
```

Dividing by zero triggers a panic. All panics across the FFI boundary cause
undefined behavior. In this chapter, we'll learn about how to debug native 
libraries.

## Putting a Debugger in a Debugger

### Java Instructions

Run a test with debug and add a breakpoint here:

```
●   static native int divide(int a, int b);
```

You'll need to find the PID of your test. If you're using Java 11, then you can
fetch it using the `ProcessHandle` API, as shown below. Otherwise, you'll need
to find the PID through other means, such as through `ps -x` on Unix-like
systems.

```java
@Test
public void testDivideByZero() {
    long pid = ProcessHandle.current().pid();
    System.out.println("PID: " + pid);
    // ... the rest of the test
}
```

Debug your test that divides by zero, and leave the test paused so that Java is
still running and the native code has not yet been entered. It's necessary to
leave the test in this state, so that you can next attach a debugger for the
native code (if there were debuggers that supported both Java and native code,
this wouldn't be necessary). Record the pid for the process, and continue to
Native Debugging.

### Native Debugging
There are several different tools that you can use to debug the native side. A
few popular choices are `gdb`, `lldb`, or IDE's like `CLIon` (which provide
frontends to `gdb` and `lldb`). Pick your favorite debugger that supports Rust,
then attach it to the Java process and set a breakpoint in the native code. If
you need a refresher or want to try out some suggested tools, refer to the
Appendix on Debugging.

If you only need to trace the native code without Java's involvement at all, it
is simpler to attach only the native debugger. Similarly, one can also debug
only Java code.

## Exercises

1. Instead of using two debuggers, try replacing the Java debugger with a print
   message and a `Thread.sleep()`. See which you prefer.
