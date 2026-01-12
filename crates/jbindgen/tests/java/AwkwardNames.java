package com.example;

/**
 * Test class with awkward method names that don't have reversible
 * transformations.
 */
public class AwkwardNames {
    /**
     * Method with consecutive uppercase letters.
     * updateUI -> update_ui, but update_ui -> updateUi (not reversible)
     */
    public void updateUI() {
    }

    /**
     * Another awkward name.
     * getHTTPResponse -> get_httpresponse, but get_httpresponse -> getHttpresponse
     * (not reversible)
     */
    public String getHTTPResponse() {
        return "OK";
    }

    /**
     * Normal reversible name for comparison.
     * getValue -> get_value, and get_value -> getValue (reversible)
     */
    public int getValue() {
        return 42;
    }
}
