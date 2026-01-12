package com.example;

class CommonBuiltinType {
    public CommonBuiltinType() {
    }
}

class CustomType {
    public CustomType() {
    }
}

public class BindJavaTypeWrapper {
    public BindJavaTypeWrapper() {
    }

    public CommonBuiltinType builtinField;
    public CustomType customField;

    public String mixTypes(CommonBuiltinType builtin, CustomType custom) {
        return "Handled mixed types";
    }
}
