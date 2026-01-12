// Support class for examples/bind_java_type_instance_of.rs
package com.example;

import java.util.ArrayList;
import java.util.List;

/**
 * A simple class that extends ArrayList to demonstrate is_instance_of
 * functionality for the bind_java_type macro.
 * Since it extends ArrayList, it can be cast to java.util.List.
 */
public class InstanceOf extends ArrayList<String> {

    public InstanceOf() {
        super();
    }

    public boolean addUpper(String val) {
        return super.add(val.toUpperCase());
    }
}
