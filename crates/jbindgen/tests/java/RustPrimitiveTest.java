package com.example.test;

import io.github.jni_rs.jbindgen.RustPrimitive;

/**
 * Test class for @RustPrimitive annotation support.
 *
 * This tests the ability to use custom Rust types that map to Java primitives,
 * such as handle types that are passed as long values.
 */
public class RustPrimitiveTest {

    /**
     * Process a handle passed from Rust as a long.
     * The @RustPrimitive annotation tells the generator to use ThingHandle
     * instead of jlong in the generated Rust bindings.
     */
    public native String processHandle(@RustPrimitive("ThingHandle") long handle);

    /**
     * Create a new handle and return it as a long.
     * Native implementation would box a Rust object and return the pointer.
     */
    public native long createHandle(@RustPrimitive("OtherHandle") long existingHandle);

    /**
     * Test with multiple primitive parameters, some annotated and some not.
     */
    public native int processMultiple(
            @RustPrimitive("ThingHandle") long handle1,
            int normalInt,
            @RustPrimitive("ThingHandle") long handle2,
            long normalLong);

    /**
     * Static native method with RustPrimitive annotation.
     */
    public static native void staticWithHandle(@RustPrimitive("ThingHandle") long handle);

    /**
     * Test array parameters - arrays cannot use RustPrimitive.
     */
    public native void processArray(long[] handles);

    /**
     * Regular method (non-native) with RustPrimitive - should be ignored/error.
     */
    public void regularMethod(@RustPrimitive("ThingHandle") long handle) {
        // This should either be ignored or cause an error since it's not native
    }
}
