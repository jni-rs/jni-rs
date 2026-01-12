package com.example;

/**
 * Test class for verifying overload naming conventions.
 * Tests various overload scenarios including different arities,
 * type variations, and edge cases.
 */
public class Overloads {

    // Constructors with various arities
    public Overloads() {
    }

    public Overloads(int value) {
    }

    public Overloads(String name) {
    }

    public Overloads(int value, String name) {
    }

    public Overloads(String name, int value) {
    }

    public Overloads(String first, String second) {
    }

    // Arity 3: all same type - should get just arity suffix "new3"
    public Overloads(int a, int b, int c) {
    }

    // Arity 4: partial variation (position 0 always int, positions 1-3 vary)
    // Should get arity prefix "4" plus varying type suffixes
    public Overloads(int a, int b, int c, int d) {
    }

    public Overloads(int a, String b, int c, int d) {
    }

    public Overloads(int a, String b, String c, int d) {
    }

    public Overloads(int a, String b, String c, String d) {
    }

    // Methods with arity 0 and 1 - tests that arity 1 gets type suffix
    public void process() {
    }

    public void process(int value) {
    }

    public void process(String text) {
    }

    // Methods with same arity but different types at different positions
    public int calculate(int a, int b) {
        return a + b;
    }

    public int calculate(int a, String b) {
        return a;
    }

    public int calculate(String a, int b) {
        return b;
    }

    public int calculate(String a, String b) {
        return 0;
    }

    // Methods with arity 3 where only some positions vary
    public void transform(int x, String y, boolean z) {
    }

    public void transform(int x, String y, int z) {
    }

    public void transform(int x, int y, boolean z) {
    }

    // Methods with arrays
    public void update(int[] values) {
    }

    public void update(String[] values) {
    }

    public void update(int[][] matrix) {
    }

    // Methods where all arguments are the same type (no varying positions)
    public void set(int value) {
    }

    public void set(int a, int b) {
    }

    public void set(int a, int b, int c) {
    }

    // Static methods with overloads
    public static String format(String text) {
        return text;
    }

    public static String format(String format, Object arg) {
        return format;
    }

    public static String format(String format, Object arg1, Object arg2) {
        return format;
    }

    // Methods testing edge case: base name ends with a number
    public void method1() {
    }

    public void method1(int value) {
    }

    public void method1(int a, int b) {
    }

    // Methods with mixed primitive and object types
    public void combine(int primitive, String object) {
    }

    public void combine(Integer boxed, String object) {
    }

    public void combine(int primitive, int primitive2) {
    }
}
