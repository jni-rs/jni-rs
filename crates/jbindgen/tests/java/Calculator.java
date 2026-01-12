package com.example;

/**
 * Test class with various method signatures.
 */
public class Calculator {
    public Calculator() {
    }

    public static int multiply(int a, int b) {
        return a * b;
    }

    public static long multiplyLong(long a, long b) {
        return a * b;
    }

    public static double divide(double a, double b) {
        return a / b;
    }

    public int square(int x) {
        return x * x;
    }

    public double power(double base, double exponent) {
        return Math.pow(base, exponent);
    }
}
