#!/bin/bash
# Build script for jbindgen-annotations

set -e

cd "$(dirname "$0")"

echo "Building jbindgen-annotations..."

# Check if Maven is available
if command -v mvn &> /dev/null; then
    mvn clean package
    echo "Build complete! JAR available at: target/jbindgen-annotations-0.1.0.jar"
else
    echo "Maven not found. Falling back to javac..."

    # Create output directories
    mkdir -p target/classes
    mkdir -p target

    # Compile
    javac -d target/classes \
        src/main/java/io/github/jni_rs/jbindgen/*.java

    # Create JAR
    cd target/classes
    jar cf ../jbindgen-annotations-0.1.0.jar io/github/jni_rs/jbindgen/*.class
    cd ../..

    echo "Build complete! JAR available at: target/jbindgen-annotations-0.1.0.jar"
fi
