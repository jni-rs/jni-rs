package com.example;

/**
 * Test class to verify name collision detection and resolution.
 * Contains fields and methods that map to the same Rust snake_case names.
 */
public class NameCollisionTest {
    // These two fields will collide: both map to "my_value"
    public String myValue;
    public String myVALUE;

    // Another field collision: getData vs getDATA
    public int getData;
    public int getDATA;

    public NameCollisionTest() {
        this.myValue = "value";
        this.myVALUE = "VALUE";
        this.getData = 1;
        this.getDATA = 2;
    }

    // These two methods will collide: both map to "to_uri"
    public String toURI() {
        return "URI";
    }

    public String toUri() {
        return "uri";
    }

    // Another collision example: getURL vs getUrl
    public String getURL() {
        return "URL";
    }

    public String getUrl() {
        return "url";
    }

    // Static method collision
    public static void setID(int id) {
        // do nothing
    }

    public static void setId(int id) {
        // do nothing
    }
}
