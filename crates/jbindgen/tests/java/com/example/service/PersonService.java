package com.example.service;

import com.example.data.Person;

/**
 * PersonService class that uses Person from a different package.
 * Demonstrates cross-package type references in bindings.
 */
public class PersonService {
    public PersonService() {
    }

    public static Person createPerson(String name, int age) {
        return new Person(name, age);
    }

    public static String getPersonName(Person person) {
        return person.getName();
    }

    public static int getPersonAge(Person person) {
        return person.getAge();
    }

    public static void updatePerson(Person person, String name, int age) {
        person.setName(name);
        person.setAge(age);
    }
}
