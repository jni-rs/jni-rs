package com.example.test;

import io.github.jni_rs.jbindgen.RustName;

/**
 * Test class for @RustName annotation support.
 */
@RustName("CustomAnnotatedClass")
public class AnnotationTest {

    @RustName("custom_static_field")
    public static String STATIC_FIELD = "static";

    @RustName("custom_instance_field")
    public String instanceField = "instance";

    public String normalField = "normal";

    @RustName("new_default")
    public AnnotationTest() {
    }

    @RustName("new_with_value")
    public AnnotationTest(String value) {
    }

    // Constructor without annotation
    public AnnotationTest(int number) {
    }

    @RustName("custom_static_method")
    public static String staticMethod() {
        return "static";
    }

    @RustName("custom_instance_method")
    public String instanceMethod() {
        return "instance";
    }

    // Method without annotation
    public String normalMethod() {
        return "normal";
    }

    @RustName("custom_native_method")
    public native void nativeMethod();

    // Native method without annotation
    public native void normalNativeMethod();
}
