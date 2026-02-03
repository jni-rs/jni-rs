package io.github.jni_rs.jbindgen;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Annotation to skip generation of bindings for a Java class, constructor,
 * method, or field.
 *
 * <p>
 * This annotation tells jbindgen to completely skip the annotated element
 * during binding
 * generation. This is useful for:
 * <ul>
 * <li>Excluding deprecated or internal APIs</li>
 * <li>Skipping methods that are not needed in Rust</li>
 * <li>Avoiding problematic APIs that cannot be easily bound</li>
 * <li>Reducing the size of generated bindings</li>
 * </ul>
 *
 * <p>
 * <b>Usage examples:</b>
 *
 * <pre>
 * // Skip an entire class
 * {@literal @}RustSkip
 * public class InternalClass { }
 *
 * // Skip a specific constructor
 * public class MyClass {
 *     {@literal @}RustSkip
 *     public MyClass(InternalConfig config) { }
 *
 *     public MyClass() { } // This constructor will be included
 * }
 *
 * // Skip a specific method
 * public class MyClass {
 *     {@literal @}RustSkip
 *     public void internalMethod() { }
 *
 *     public void publicMethod() { } // This method will be included
 * }
 *
 * // Skip a field
 * public class MyClass {
 *     {@literal @}RustSkip
 *     public String internalField;
 *
 *     public String publicField; // This field will be included
 * }
 * </pre>
 *
 * <p>
 * <b>Note:</b> When applied to a class, the entire class and all its members
 * will be skipped.
 * When applied to individual members, only those members are skipped.
 *
 * @see io.github.jni_rs.jbindgen.RustName
 * @since 1.0
 */
@Retention(RetentionPolicy.RUNTIME)
@Target({ ElementType.TYPE, ElementType.CONSTRUCTOR, ElementType.METHOD, ElementType.FIELD })
public @interface RustSkip {
}
