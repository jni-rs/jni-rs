package com.example;

/**
 * Test class with various field types for testing bind_java_type field
 * bindings.
 */
public class TestFields {
    // Static primitive fields
    public static int staticIntField = 42;
    public static long staticLongField = 9876543210L;
    public static boolean staticBooleanField = true;
    public static byte staticByteField = 127;
    public static short staticShortField = 32000;
    public static float staticFloatField = 3.14f;
    public static double staticDoubleField = 2.71828;
    public static char staticCharField = 'X';

    // Static String field
    public static String staticStringField = "static string value";

    // Instance primitive fields
    public int intField;
    public long longField;
    public boolean booleanField;
    public byte byteField;
    public short shortField;
    public float floatField;
    public double doubleField;
    public char charField;

    // Instance String field
    public String stringField;

    // Constructor
    public TestFields() {
        this.intField = 10;
        this.longField = 100L;
        this.booleanField = false;
        this.byteField = 1;
        this.shortField = 200;
        this.floatField = 1.5f;
        this.doubleField = 2.5;
        this.charField = 'A';
        this.stringField = "instance string";
    }

    public TestFields(int intValue, String stringValue) {
        this.intField = intValue;
        this.longField = intValue * 10L;
        this.booleanField = intValue > 0;
        this.byteField = (byte) (intValue % 128);
        this.shortField = (short) (intValue * 2);
        this.floatField = intValue * 1.5f;
        this.doubleField = intValue * 2.5;
        this.charField = 'B';
        this.stringField = stringValue;
    }

    // Helper methods to verify field access
    public int getIntField() {
        return intField;
    }

    public String getStringField() {
        return stringField;
    }

    public static int getStaticIntField() {
        return staticIntField;
    }

    public static String getStaticStringField() {
        return staticStringField;
    }
}
