package com.example;

class CommonBuiltinType {
    public CommonBuiltinType() {
    }
}

class CustomType {
    public CustomType() {
    }
}

public class NativeMethodWrapper {
    public native String processResource(long handle);

    public static native String mixTypes(CommonBuiltinType builtin, CustomType custom);
}
