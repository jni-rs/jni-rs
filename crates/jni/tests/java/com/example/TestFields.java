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
    public static byte[] staticByteArray = new byte[0];

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
    public byte[] byteArrayField;

    // Instance String field
    public String stringField;

    // Fields for testing non_null validation
    public String nullableStringField; // Can be null
    public String requiredStringField; // Should be validated with non_null
    public String validatedStringField; // Should be validated with non_null (block syntax)

    // Fields guarded by _cfg_test feature (never enabled in tests)
    public static int staticCfgTestField = 99;
    public int instanceCfgTestField;

    // Fields guarded by invocation feature (always available in tests)
    public static int staticInvocationField = 55;
    public int instanceInvocationField;

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
        this.byteArrayField = new byte[0];
        this.stringField = "instance string";
        this.nullableStringField = null; // Initialize to null for testing
        this.requiredStringField = null; // Initialize to null to test validation
        this.validatedStringField = null; // Initialize to null to test validation
        this.instanceCfgTestField = 77;
        this.instanceInvocationField = 88;
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
}
