#!/usr/bin/env bash
# Fail immediately in case of errors and/or unset variables
set -eu -o pipefail

echo "JAVA_HOME=${JAVA_HOME}"

# Find the directory containing libjvm (the relative path has changed in Java 9)
export LD_LIBRARY_PATH="$(find ${JAVA_HOME} -name libjvm.* -printf '%h\n')"
echo "LD_LIBRARY_PATH=${LD_LIBRARY_PATH}"

# Clean this package to make sure we always build with a currently set JDK version.
cargo clean -p jni

# Run the integration tests
cargo test --features=backtrace,invocation
