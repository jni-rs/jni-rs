package io.github.jni_rs.jbindgen.test;

import io.github.jni_rs.jbindgen.RustSkip;

/**
 * Test class to verify @RustSkip annotation behavior
 */
public class RustSkipTest {

    // This field should be included
    public int includedField;

    // This field should be skipped
    @RustSkip
    public int skippedField;

    // This constructor should be included
    public RustSkipTest() {
    }

    // This constructor should be skipped
    @RustSkip
    public RustSkipTest(int value) {
    }

    // This method should be included
    public void includedMethod() {
    }

    // This method should be skipped
    @RustSkip
    public void skippedMethod() {
    }

    // This method should be included
    public int getIncludedField() {
        return includedField;
    }

    // This method should be skipped
    @RustSkip
    public int getSkippedField() {
        return skippedField;
    }
}

/**
 * This entire class should be skipped
 */
@RustSkip
class SkippedClass {
    public int field;

    public void method() {
    }
}

/**
 * This class should be included but has some skipped members
 */
class MixedClass {
    public int normalField;

    @RustSkip
    public int internalField;

    public void publicMethod() {
    }

    @RustSkip
    public void internalMethod() {
    }
}
