package io.github.jni_rs.jbindgen;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Annotation to override the generated Rust name for Java classes, methods,
 * fields, and constructors.
 *
 * <p>
 * This annotation allows you to customize the Rust identifier names generated
 * by jbindgen,
 * which is useful when:
 * <ul>
 * <li>The automatic naming conversion doesn't produce the desired result</li>
 * <li>You want to match existing Rust naming conventions in your codebase</li>
 * <li>You need to avoid naming conflicts</li>
 * <li>Java method overloads need more readable Rust names</li>
 * </ul>
 *
 * <p>
 * <b>Usage examples:</b>
 * 
 * <pre>
 * // Override class name
 * {@literal @}RustName("CustomClassName")
 * public class MyClass { }
 *
 * // Override method name (use snake_case for Rust methods)
 * {@literal @}RustName("custom_method")
 * public void someMethod() { }
 *
 * // Override field name (use snake_case for Rust fields)
 * {@literal @}RustName("custom_field")
 * public String someField;
 *
 * // Override constructor - useful for overloaded constructors
 * {@literal @}RustName("new_with_config")
 * public MyClass(Config config) { }
 *
 * // Override native method name
 * {@literal @}RustName("native_callback")
 * public native void onCallback();
 * </pre>
 *
 * <p>
 * <b>Naming conventions:</b>
 * <ul>
 * <li>For types: Use PascalCase (e.g., "MyType")</li>
 * <li>For methods and fields: Use snake_case (e.g., "my_method")</li>
 * <li>For constructors: Typically "new" or "new_*" (e.g.,
 * "new_with_options")</li>
 * </ul>
 *
 * <p>
 * The annotation value must be a valid Rust identifier. The jbindgen tool will
 * use this
 * exact name in the generated bindings without any transformation.
 *
 * @since 0.1.0
 */
@Retention(RetentionPolicy.RUNTIME)
@Target({ ElementType.TYPE, ElementType.METHOD, ElementType.FIELD, ElementType.CONSTRUCTOR })
public @interface RustName {
    /**
     * The Rust name to use in generated bindings.
     *
     * <p>
     * This should be a valid Rust identifier following Rust naming conventions:
     * <ul>
     * <li>Types: PascalCase</li>
     * <li>Methods/fields: snake_case</li>
     * <li>Constructors: typically "new" or "new_*"</li>
     * </ul>
     *
     * @return the Rust identifier name
     */
    String value();
}
