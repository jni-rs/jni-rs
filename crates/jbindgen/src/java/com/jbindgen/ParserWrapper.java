package com.jbindgen;

/**
 * Wrapper around Parser with convenient static methods for JNI invocation.
 */
public class ParserWrapper {

    /**
     * Parse Java source files and return array of ClassDescription objects.
     *
     * @param sourcePaths      Array of source file or directory paths
     * @param classPathEntries Array of classpath entries (JARs or directories)
     * @param classPattern     Pattern to match classes (e.g., "android.app.*")
     * @return Array of ClassDescription objects
     */
    public static Parser.ClassDescription[] parse(String[] sourcePaths, String[] classPathEntries,
            String classPattern) {
        try {
            Parser parser = new Parser();
            return parser.parse(sourcePaths, classPathEntries, classPattern);
        } catch (Exception e) {
            throw new RuntimeException("Failed to parse sources: " + e.getMessage(), e);
        }
    }
}
