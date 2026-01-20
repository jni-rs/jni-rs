package io.github.jni_rs.jbindgen;

import java.lang.annotation.ElementType;
import java.lang.annotation.Retention;
import java.lang.annotation.RetentionPolicy;
import java.lang.annotation.Target;

/**
 * Annotation to map a Java primitive type parameter to a custom Rust type.
 *
 * <p>
 * This annotation is used for parameters in native methods and regular methods
 * where a Java primitive (typically {@code long}) is used to represent a Rust
 * type that transparently wraps the primitive (such as a handle or pointer
 * type).
 *
 * <p>
 * This requires a corresponding "unsafe RustType => javaPrimitive" type mapping
 * to be provided via --type-map or in a type mapping file.
 *
 * <p>
 * <b>Usage example:</b>
 *
 * <pre>
 * public class MyClass {
 *     // Native method accepting a handle wrapped as a long
 *     public native String processHandle({@literal @}RustPrimitive("ThingHandle") long thing);
 *
 *     // Regular method accepting a handle wrapped as a long
 *     public String doSomething({@literal @}RustPrimitive("ThingHandle") long handle) {
 *         return processHandle(handle);
 *     }
 *
 *     // Static method with custom type
 *     public static int getRawFd({@literal @}RustPrimitive("RawFd") int fd) {
 *         return fd;
 *     }
 * }
 * </pre>
 *
 * <p>
 * <b>Type mapping requirement:</b>
 * 
 * <pre>
 * unsafe ThingHandle => long
 * unsafe RawFd => int
 * </pre>
 *
 * <p>
 * The generated Rust binding will use the custom type instead of the primitive:
 * 
 * <pre>
 * fn process_handle(&amp;self, env: &amp;mut Env, thing: ThingHandle) -> Result&lt;JString&gt;
 * </pre>
 *
 * @since 1.0
 */
@Retention(RetentionPolicy.RUNTIME)
@Target(ElementType.PARAMETER)
public @interface RustPrimitive {
    /**
     * The Rust type name that this primitive maps to.
     *
     * @return the Rust type name (e.g., "ThingHandle", "RawFd")
     */
    String value();
}
