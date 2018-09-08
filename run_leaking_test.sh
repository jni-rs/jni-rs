#!/usr/bin/env bash
# Runs a leaking test — open the process manager and see the memory size grow.
# If you patch the test to use with_local_frame on each iteration, the memory
# usage will remain about the same.


# Unfortunately, a simple `which java` will not work for some users (e.g., jenv),
# hence this a bit complex thing.
JAVA_HOME="$(java -XshowSettings:properties -version 2>&1 > /dev/null | grep 'java.home' | awk '{print $3}')"
echo "JAVA_HOME=${JAVA_HOME}"

# Find the directory containing libjvm (the relative path has changed in Java 9).
JAVA_LIB_DIR="$(find ${JAVA_HOME} -type f -name libjvm.\* | xargs -n1 dirname)"
echo "JAVA_LIB_DIR=${JAVA_LIB_DIR}"

export LD_LIBRARY_PATH="${JAVA_LIB_DIR}"
cargo test class_lookup_leaks_local_references --features=backtrace,invocation -- --nocapture
