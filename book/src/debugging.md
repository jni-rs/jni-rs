# Debugging
This section introduces debugging Java processes that use Rust shared libraries.
If you don't have a thing to debug, then start with the [division
exercise](./division_exercise.md).

## Java Instructions
Set a breakpoint here:

```java
class NativeAPI {
●   static native int divide(int a, int b);
}
```

Then debug your test, and leave it paused at the breakpoint. You'll need the PID
of your test. If you're using Java 9, then you can fetch it using the
`ProcessHandle` API, as shown below. Otherwise, you'll need to find the PID
through other means, such as through the `jps` command (provided with your Java
installation) or `ps -x` on Unix-like systems.

```bash
# Example of finding the pid using jps
$ jps
32273 Jps
9316
26997 Main
27308 Main
```

```java
@Test
public void testDivide() {
    // Java 9+ only
    long pid = ProcessHandle.current().pid();
    System.out.println("PID: " + pid);
    // ... the rest of the test
}
```

Leave the test paused at the Java breakpoint, record the pid for the process,
and continue to Native Debugging.

## Native Debugging
There are several tools that you can use debug the native side, such as `gdb` or
`lldb`. When you attach or launch a program using a native debugger, you must it
to ignore SIGSEGV to prevent it from [randomly
closing](https://neugens.wordpress.com/2015/02/26/debugging-the-jdk-with-gdb/),
due to [Java implementation
details](https://medium.com/@pirogov.alexey/gdb-debug-native-part-of-java-application-c-c-libraries-and-jdk-6593af3b4f3f).
For gdb, this is forced using `(gdb) handle SIGSEGV nostop noprint pass`.

If you'd like to debug both Java and the native side during an execution, then
do the following:

1. Run the Java program with a Java debugger, with a breakpoint set on or before
   the native function.
2. Attach your favorite Rust debugger to the Java process. Remember to ignore
   SIGSEGV.
3. Set a breakpoint in the Rust `divide` code.
4. Resume the Java debugger, so that the process advances to the native stack.
5. From here, you can continue to work with both debuggers as necessary.

# Learn More
Aside from the debugger, it's also sometimes necessary to [troubleshoot memory
leaks](https://docs.oracle.com/en/java/javase/11/troubleshoot/troubleshoot-memory-leaks.html#GUID-79F26B47-9240-4F32-A817-1DD77A361F31).
[-Xcheck:jni](https://docs.oracle.com/javase/8/docs/technotes/guides/troubleshoot/clopts002.html)
can also be useful for validating usage of certain JNI calls.
