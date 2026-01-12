package com.jbindgen;

import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.lang.model.element.*;
import javax.lang.model.type.*;
import javax.lang.model.util.*;
import javax.tools.*;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.jar.*;

/**
 * Java source parser that uses JavacTask to analyze Java source files
 * and extract API information including fully-qualified types and parameter
 * names.
 *
 * Supports reading source files from:
 * - Regular filesystem .java files
 * - Source files within JAR archives
 */
public class Parser {
    private final JavaCompiler compiler;
    private final StandardJavaFileManager fileManager;

    public Parser() {
        this.compiler = ToolProvider.getSystemJavaCompiler();
        if (this.compiler == null) {
            throw new RuntimeException("No Java compiler available. Please ensure you're running with a JDK.");
        }
        this.fileManager = compiler.getStandardFileManager(null, null, null);
    }

    /**
     * Custom JavaFileObject for reading source files from JAR archives.
     */
    private static class JarSourceFileObject extends SimpleJavaFileObject {
        private final String content;
        private final String className;

        public JarSourceFileObject(String name, String content) {
            super(URI.create("jar:///" + name.replace('\\', '/')), Kind.SOURCE);
            this.content = content;
            this.className = name;
        }

        @Override
        public CharSequence getCharContent(boolean ignoreEncodingErrors) {
            return content;
        }

        @Override
        public String getName() {
            return className;
        }
    }

    /**
     * Parse Java source files and return array of ClassDescription objects.
     *
     * @param sourcePaths      Array of source file, directory paths, or JAR file
     *                         paths
     * @param classPathEntries Array of classpath entries (JARs or directories)
     * @param classPattern     Pattern to match classes (e.g., "android.app.*")
     * @return Array of ClassDescription objects
     */
    public ClassDescription[] parse(String[] sourcePaths, String[] classPathEntries, String classPattern)
            throws IOException {
        // Collect all source files from various sources
        List<JavaFileObject> compilationUnits = new ArrayList<>();

        for (String sourcePath : sourcePaths) {
            File sourceFile = new File(sourcePath);

            if (sourceFile.isDirectory()) {
                // Handle directory of .java files
                List<File> javaFiles = new ArrayList<>();
                findJavaFiles(sourceFile, javaFiles);
                Iterable<? extends JavaFileObject> fileObjects = fileManager.getJavaFileObjectsFromFiles(javaFiles);
                for (JavaFileObject fo : fileObjects) {
                    compilationUnits.add(fo);
                }
            } else if (sourcePath.endsWith(".jar")) {
                // Handle JAR file containing .java sources
                List<JavaFileObject> jarSources = extractSourcesFromJar(sourcePath);
                compilationUnits.addAll(jarSources);
            } else if (sourcePath.endsWith(".java")) {
                // Handle individual .java file
                JavaFileObject fileObject = fileManager.getJavaFileObjects(sourceFile).iterator().next();
                compilationUnits.add(fileObject);
            }
        }

        if (compilationUnits.isEmpty()) {
            return new ClassDescription[0];
        }

        // Set up compiler options
        List<String> options = new ArrayList<>();
        if (classPathEntries.length > 0) {
            options.add("-classpath");
            String joinedClasspath = String.join(File.pathSeparator, classPathEntries);
            options.add(joinedClasspath);
        }
        options.add("-proc:none"); // Disable annotation processing
        options.add("-nowarn"); // Suppress warnings

        // Create and configure the compilation task
        JavaCompiler.CompilationTask task = compiler.getTask(
                null, // use default writer
                fileManager,
                null, // use default diagnostic listener
                options,
                null, // no classes for annotation processing
                compilationUnits);

        JavacTask javacTask = (JavacTask) task;

        // Parse and analyze
        try {
            Iterable<? extends CompilationUnitTree> trees = javacTask.parse();
            javacTask.analyze();

            Elements elements = javacTask.getElements();
            Types types = javacTask.getTypes();
            Trees treeUtils = Trees.instance(javacTask);

            List<ClassDescription> classes = new ArrayList<>();

            for (CompilationUnitTree tree : trees) {
                new TreePathScanner<Void, Void>() {
                    @Override
                    public Void visitClass(ClassTree node, Void p) {
                        Element element = treeUtils.getElement(getCurrentPath());
                        if (element instanceof TypeElement) {
                            TypeElement typeElement = (TypeElement) element;

                            // Get the binary name (using $ for inner classes)
                            String binaryName = getBinaryName(typeElement);
                            String simpleName = typeElement.getSimpleName().toString();

                            // Skip anonymous inner classes (e.g., ClassName$1, ClassName$1$1)
                            // These are synthetic classes generated by the compiler for anonymous classes
                            // Also skip classes with empty simple names (synthetic/generated classes)
                            if (isAnonymousClass(binaryName) || simpleName.isEmpty()) {
                                return super.visitClass(node, p);
                            }

                            // Check if this class matches the pattern
                            if (classPattern == null || matchesPattern(binaryName, classPattern)) {
                                ClassDescription classDesc = analyzeClass(typeElement, elements, types);
                                classes.add(classDesc);
                            }
                        }
                        return super.visitClass(node, p);
                    }
                }.scan(tree, null);
            }

            return classes.toArray(new ClassDescription[0]);

        } catch (IOException e) {
            throw new IOException("Failed to parse source files", e);
        }
    }

    /**
     * Extract Java source files from a JAR archive.
     *
     * Extracts ALL .java files from the JAR.
     * This is necessary because the Java compiler may need access to dependency
     * classes and related types in order to properly parse and analyze the classes
     * that match the pattern. The pattern filtering happens later during class
     * analysis.
     *
     * @param jarPath Path to the JAR file
     * @return List of JavaFileObject instances for all sources in the JAR
     */
    private List<JavaFileObject> extractSourcesFromJar(String jarPath) throws IOException {
        List<JavaFileObject> sources = new ArrayList<>();

        try (JarFile jarFile = new JarFile(jarPath)) {
            Enumeration<JarEntry> entries = jarFile.entries();

            while (entries.hasMoreElements()) {
                JarEntry entry = entries.nextElement();
                String name = entry.getName();

                // Only process .java files - no pattern filtering here
                if (!name.endsWith(".java")) {
                    continue;
                }

                // Read the source file content
                try (InputStream is = jarFile.getInputStream(entry);
                        InputStreamReader reader = new InputStreamReader(is, StandardCharsets.UTF_8)) {
                    StringBuilder content = new StringBuilder();
                    char[] buffer = new char[8192];
                    int read;
                    while ((read = reader.read(buffer)) != -1) {
                        content.append(buffer, 0, read);
                    }

                    sources.add(new JarSourceFileObject(name, content.toString()));
                }
            }
        }

        return sources;
    }

    /**
     * Check if a class name represents an anonymous inner class.
     * Anonymous classes have names like: OuterClass$1, OuterClass$1$1, etc.
     * These are synthetic classes created by the compiler.
     */
    private boolean isAnonymousClass(String binaryName) {
        // Split by $ to get the parts
        String[] parts = binaryName.split("\\$");
        if (parts.length <= 1) {
            return false; // No $ means not an inner class
        }

        // Check if any part after $ is a pure digit (indicates anonymous class)
        for (int i = 1; i < parts.length; i++) {
            if (parts[i].matches("\\d+")) {
                return true;
            }
        }

        return false;
    }

    private boolean matchesPattern(String jvmClassName, String patterns) {
        if (patterns.equals("*")) {
            return true;
        }

        // Support comma-separated patterns
        String[] patternList = patterns.split(",");
        for (String pattern : patternList) {
            pattern = pattern.trim();
            if (matchesSinglePattern(jvmClassName, pattern)) {
                return true;
            }
        }
        return false;
    }

    private boolean matchesSinglePattern(String jvmClassName, String pattern) {
        if (pattern.equals("*")) {
            return true;
        }

        // Pattern is given in Java format (with dots), convert to JVM format (with $)
        String jvmPattern = convertToJvmClassName(pattern);

        // Convert glob-style pattern to regex
        // e.g., "android.app.*" -> "^android\.app\..*$"
        String regex;
        if (pattern.contains("*")) {
            // Wildcard pattern - just convert to regex
            regex = "^" + jvmPattern
                    .replace(".", "\\.")
                    .replace("$", "\\$")
                    .replace("*", ".*")
                    + "$";
        } else if (pattern.contains("$")) {
            // Specific inner class pattern - match exactly
            regex = "^" + jvmPattern
                    .replace(".", "\\.")
                    .replace("$", "\\$")
                    + "$";
        } else {
            // Outer class pattern - also match all inner classes
            regex = "^" + jvmPattern
                    .replace(".", "\\.")
                    .replace("$", "\\$")
                    + "(\\$.*)?$"; // Match the class itself or any inner class
        }

        return jvmClassName.matches(regex);
    }

    private ClassDescription analyzeClass(TypeElement element, Elements elements, Types types) {
        ClassDescription desc = new ClassDescription();

        // Build the binary name (with $ for inner classes) by walking the enclosing
        // elements
        String binaryName = getBinaryName(element);

        desc.className = binaryName.replace('.', '/');
        desc.simpleName = element.getSimpleName().toString();

        // Find the package by checking if the enclosing element is a package or class
        Element enclosingElement = element.getEnclosingElement();
        if (enclosingElement != null && enclosingElement.getKind() == ElementKind.CLASS) {
            // This is an inner class - find the package by walking up to a package element
            Element e = element;
            while (e.getEnclosingElement() != null && e.getEnclosingElement().getKind() != ElementKind.PACKAGE) {
                e = e.getEnclosingElement();
            }
            if (e.getEnclosingElement() != null && e.getEnclosingElement().getKind() == ElementKind.PACKAGE) {
                PackageElement pkg = (PackageElement) e.getEnclosingElement();
                desc.packageName = pkg.getQualifiedName().toString();
            } else {
                desc.packageName = "";
            }
        } else {
            // Regular class - extract package from binary name
            int lastDot = binaryName.lastIndexOf('.');
            if (lastDot > 0) {
                desc.packageName = binaryName.substring(0, lastDot);
            } else {
                desc.packageName = "";
            }
        }

        // Extract Javadoc
        String docComment = elements.getDocComment(element);
        desc.documentation = (docComment != null) ? docComment.trim() : "";

        List<MethodDescription> constructors = new ArrayList<>();
        List<MethodDescription> methods = new ArrayList<>();
        List<FieldDescription> fields = new ArrayList<>();
        List<MethodDescription> nativeMethods = new ArrayList<>();
        List<String> interfaces = new ArrayList<>();

        // Extract superclass
        TypeMirror superclass = element.getSuperclass();
        if (superclass.getKind() != TypeKind.NONE) {
            String superClassName = getTypeName(superclass, types);
            desc.superClass = superClassName;
        }

        // Extract interfaces
        for (TypeMirror interfaceType : element.getInterfaces()) {
            String interfaceName = getTypeName(interfaceType, types);
            interfaces.add(interfaceName);
        }

        // Analyze members
        for (Element enclosed : element.getEnclosedElements()) {
            if (enclosed.getKind() == ElementKind.FIELD) {
                FieldDescription field = analyzeField((VariableElement) enclosed, types, elements);
                if (field != null) {
                    fields.add(field);
                }
            } else if (enclosed.getKind() == ElementKind.CONSTRUCTOR) {
                ExecutableElement constructorElement = (ExecutableElement) enclosed;
                // Skip non-public constructors
                if (!constructorElement.getModifiers().contains(Modifier.PUBLIC)) {
                    continue;
                }
                MethodDescription method = analyzeMethod(constructorElement, types, elements, true);
                if (method != null) {
                    constructors.add(method);
                }
            } else if (enclosed.getKind() == ElementKind.METHOD) {
                ExecutableElement methodElement = (ExecutableElement) enclosed;
                boolean isNative = methodElement.getModifiers().contains(Modifier.NATIVE);

                MethodDescription method = analyzeMethod(methodElement, types, elements, false);
                if (method != null) {
                    if (isNative) {
                        nativeMethods.add(method);
                    } else {
                        methods.add(method);
                    }
                }
            }
        }

        desc.constructors = constructors.toArray(new MethodDescription[0]);
        desc.methods = methods.toArray(new MethodDescription[0]);
        desc.fields = fields.toArray(new FieldDescription[0]);
        desc.nativeMethods = nativeMethods.toArray(new MethodDescription[0]);
        desc.interfaces = interfaces.toArray(new String[0]);

        return desc;
    }

    private FieldDescription analyzeField(VariableElement element, Types types, Elements elements) {
        // Skip private fields
        if (element.getModifiers().contains(Modifier.PRIVATE)) {
            return null;
        }

        FieldDescription desc = new FieldDescription();
        desc.name = element.getSimpleName().toString();

        // Extract Javadoc
        String docComment = elements.getDocComment(element);
        desc.documentation = (docComment != null) ? docComment.trim() : "";

        TypeDescription typeDesc = createTypeDescription(element.asType(), types);
        desc.typeName = typeDesc.typeName;
        desc.descriptor = typeDesc.descriptor;
        desc.arrayDimensions = typeDesc.arrayDimensions;
        desc.isPrimitive = typeDesc.isPrimitive;
        desc.isStatic = element.getModifiers().contains(Modifier.STATIC);
        desc.isFinal = element.getModifiers().contains(Modifier.FINAL);
        desc.isDeprecated = element.getAnnotation(Deprecated.class) != null;

        return desc;
    }

    private MethodDescription analyzeMethod(ExecutableElement element, Types types, Elements elements,
            boolean isConstructor) {
        boolean isPublic = element.getModifiers().contains(Modifier.PUBLIC);
        boolean isPrivate = element.getModifiers().contains(Modifier.PRIVATE);

        // Skip private methods unless they are native (we want to track private native
        // methods)
        boolean isNative = element.getModifiers().contains(Modifier.NATIVE);
        if (isPrivate && !isNative) {
            return null;
        }

        MethodDescription desc = new MethodDescription();
        desc.name = isConstructor ? "<init>" : element.getSimpleName().toString();

        // Extract Javadoc
        String docComment = elements.getDocComment(element);
        desc.documentation = (docComment != null) ? docComment.trim() : "";

        desc.isStatic = element.getModifiers().contains(Modifier.STATIC);
        desc.isConstructor = isConstructor;
        desc.isNative = isNative;
        desc.isDeprecated = element.getAnnotation(Deprecated.class) != null;
        desc.isPublic = isPublic;

        // Build parameters list with type information
        List<ParameterDescription> parameters = new ArrayList<>();

        for (VariableElement param : element.getParameters()) {
            TypeMirror paramType = param.asType();

            ParameterDescription paramDesc = new ParameterDescription();
            paramDesc.name = param.getSimpleName().toString();
            TypeDescription typeDesc = createTypeDescription(paramType, types);
            paramDesc.typeName = typeDesc.typeName;
            paramDesc.descriptor = typeDesc.descriptor;
            paramDesc.arrayDimensions = typeDesc.arrayDimensions;
            paramDesc.isPrimitive = typeDesc.isPrimitive;
            parameters.add(paramDesc);
        }
        desc.parameters = parameters.toArray(new ParameterDescription[0]);

        // Add return type information
        TypeMirror returnType = element.getReturnType();
        desc.returnType = createTypeDescription(returnType, types);

        return desc;
    }

    private String getTypeDescriptor(TypeMirror type, Types types) {
        switch (type.getKind()) {
            case BOOLEAN:
                return "Z";
            case BYTE:
                return "B";
            case CHAR:
                return "C";
            case SHORT:
                return "S";
            case INT:
                return "I";
            case LONG:
                return "J";
            case FLOAT:
                return "F";
            case DOUBLE:
                return "D";
            case VOID:
                return "V";
            case ARRAY:
                ArrayType arrayType = (ArrayType) type;
                return "[" + getTypeDescriptor(arrayType.getComponentType(), types);
            case DECLARED:
                DeclaredType declaredType = (DeclaredType) type;
                TypeElement element = (TypeElement) declaredType.asElement();
                // Use getBinaryName to properly handle inner classes with $ separator
                String binaryName = getBinaryName(element);
                return "L" + binaryName.replace('.', '/') + ";";
            case TYPEVAR:
                // For type variables, use Object
                return "Ljava/lang/Object;";
            default:
                // Default to Object for unknown types
                return "Ljava/lang/Object;";
        }
    }

    /**
     * Create a TypeDescription with proper array dimension tracking.
     */
    private TypeDescription createTypeDescription(TypeMirror type, Types types) {
        TypeDescription desc = new TypeDescription();

        // Count array dimensions and get element type
        int dimensions = 0;
        TypeMirror elementType = type;
        while (elementType.getKind() == TypeKind.ARRAY) {
            dimensions++;
            elementType = ((ArrayType) elementType).getComponentType();
        }

        desc.arrayDimensions = dimensions;
        // TypeKind.VOID.isPrimitive() returns false, but we treat void as primitive for
        // DEX signatures
        desc.isPrimitive = elementType.getKind().isPrimitive() || elementType.getKind() == TypeKind.VOID;
        desc.typeName = getTypeName(elementType, types);
        desc.descriptor = getTypeDescriptor(type, types);

        return desc;
    }

    private String getTypeName(TypeMirror type, Types types) {
        switch (type.getKind()) {
            case BOOLEAN:
                return "boolean";
            case BYTE:
                return "byte";
            case CHAR:
                return "char";
            case SHORT:
                return "short";
            case INT:
                return "int";
            case LONG:
                return "long";
            case FLOAT:
                return "float";
            case DOUBLE:
                return "double";
            case VOID:
                return "void";
            case ARRAY:
                // For arrays, recurse to get the element type (strips all dimensions)
                ArrayType arrayType = (ArrayType) type;
                TypeMirror componentType = arrayType.getComponentType();
                // Recursively handle multi-dimensional arrays
                return getTypeName(componentType, types);
            case DECLARED:
                DeclaredType declaredType = (DeclaredType) type;
                TypeElement element = (TypeElement) declaredType.asElement();
                // Use getBinaryName to properly handle inner classes with $ separator
                return getBinaryName(element);
            case TYPEVAR:
                // For type variables, return the variable name
                return type.toString();
            default:
                return type.toString();
        }
    }

    private void findJavaFiles(File dir, List<File> javaFiles) {
        File[] files = dir.listFiles();
        if (files != null) {
            for (File file : files) {
                if (file.isDirectory()) {
                    findJavaFiles(file, javaFiles);
                } else if (file.getName().endsWith(".java")) {
                    javaFiles.add(file);
                }
            }
        }
    }

    // Data classes for JSON serialization
    // Data classes that will be accessed via JNI - must be public static
    public static class ClassDescription {
        public String className;
        public String packageName;
        public String simpleName;
        public String documentation; // Javadoc comment for the class
        public MethodDescription[] constructors;
        public MethodDescription[] methods;
        public FieldDescription[] fields;
        public MethodDescription[] nativeMethods;
        public String superClass;
        public String[] interfaces;
    }

    public static class MethodDescription {
        public String name;
        public String documentation; // Javadoc comment for the method/constructor
        public boolean isStatic;
        public boolean isConstructor;
        public boolean isNative;
        public boolean isDeprecated;
        public boolean isPublic;
        public ParameterDescription[] parameters;
        public TypeDescription returnType;
    }

    public static class ParameterDescription {
        public String name;
        public String typeName; // Element type name (without [] suffix)
        public String descriptor; // JNI descriptor
        public int arrayDimensions; // 0 for non-array, 1 for T[], 2 for T[][], etc.
        public boolean isPrimitive; // true for primitive element types
    }

    public static class TypeDescription {
        public String typeName; // Element type name (without [] suffix)
        public String descriptor; // JNI descriptor
        public int arrayDimensions; // 0 for non-array, 1 for T[], 2 for T[][], etc.
        public boolean isPrimitive; // true for primitive types (int, boolean, etc.)
    }

    public static class FieldDescription {
        public String name;
        public String documentation; // Javadoc comment for the field
        public String typeName; // Element type name (without [] suffix)
        public String descriptor; // JNI descriptor
        public int arrayDimensions; // 0 for non-array, 1 for T[], 2 for T[][], etc.
        public boolean isPrimitive; // true for primitive element types
        public boolean isStatic;
        public boolean isFinal;
        public boolean isDeprecated;
    }

    /**
     * Get the binary name of a TypeElement.
     * For inner classes, uses $ separator. For example: com.example.Outer$Inner
     * This properly handles the class hierarchy by walking up the enclosing
     * elements.
     */
    private String getBinaryName(TypeElement element) {
        // Build the name by walking up the enclosing elements
        List<String> nameParts = new ArrayList<>();
        Element current = element;

        // Collect all class names up to the package
        while (current != null && current.getKind() != ElementKind.PACKAGE) {
            if (current.getKind() == ElementKind.CLASS ||
                    current.getKind() == ElementKind.INTERFACE ||
                    current.getKind() == ElementKind.ENUM) {
                nameParts.add(0, current.getSimpleName().toString());
            }
            current = current.getEnclosingElement();
        }

        // Add package name if present
        if (current != null && current.getKind() == ElementKind.PACKAGE) {
            PackageElement pkg = (PackageElement) current;
            String pkgName = pkg.getQualifiedName().toString();
            if (!pkgName.isEmpty()) {
                // Package + class names separated by dot, inner classes by $
                StringBuilder result = new StringBuilder(pkgName);
                result.append('.');
                for (int i = 0; i < nameParts.size(); i++) {
                    if (i > 0) {
                        result.append('$');
                    }
                    result.append(nameParts.get(i));
                }
                return result.toString();
            }
        }

        // No package - default package
        StringBuilder result = new StringBuilder();
        for (int i = 0; i < nameParts.size(); i++) {
            if (i > 0) {
                result.append('$');
            }
            result.append(nameParts.get(i));
        }
        return result.toString();
    }

    /**
     * Convert a Java qualified name to JVM class name format.
     * Inner classes use $ separator instead of . separator.
     * For example: android.os.Build.Partition -> android.os.Build$Partition
     *
     * Uses a heuristic: parts that start with uppercase are likely class names.
     * The first uppercase part is the outer class, subsequent uppercase parts are
     * inner classes.
     */
    private String convertToJvmClassName(String qualifiedName) {
        String[] parts = qualifiedName.split("\\\\.");

        StringBuilder result = new StringBuilder();
        boolean foundFirstClass = false;

        for (int i = 0; i < parts.length; i++) {
            if (i > 0) {
                // Determine if this should be . or $
                // If we've found the first class and this part starts with uppercase, use $
                if (foundFirstClass && parts[i].length() > 0 && Character.isUpperCase(parts[i].charAt(0))) {
                    result.append('$');
                } else {
                    result.append('.');
                }
            }

            result.append(parts[i]);

            // Check if this is the first class name (starts with uppercase)
            if (!foundFirstClass && parts[i].length() > 0 && Character.isUpperCase(parts[i].charAt(0))) {
                foundFirstClass = true;
            }
        }

        return result.toString();
    }
}