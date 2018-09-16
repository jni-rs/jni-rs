#!/usr/bin/env bash
# Sets LD_LIBRARY_PATH to the directory containing libjvm
# required to run tests with feature 'invocation' (depending on a JVM).
# Uses JAVA_HOME if it is set; otherwise queries the "java.home"
# system property from the JVM on the PATH.


function set_ld_library_path() {
  # Unfortunately, a simple `which java` will not work for some users (e.g., jenv),
  # hence this a bit complex thing.
  local JAVA_HOME="${JAVA_HOME:-$(java -XshowSettings:properties -version 2>&1 > /dev/null | grep 'java.home' | awk '{print $3}')}"

  local JAVA_LIB_DIR="$(find ${JAVA_HOME} -type f -name libjvm.\* | xargs -n1 dirname)"

  local NEW_LD_LIBRARY_PATH="${JAVA_LIB_DIR}"

  # Print a warning if it's already set and is distinct from the new value
  if [[ "${LD_LIBRARY_PATH:-}" != "" && "${LD_LIBRARY_PATH}" != "${NEW_LD_LIBRARY_PATH}" ]]; then
      echo "[WARNING] LD_LIBRARY_PATH variable is set and will be overridden:"
      echo "[WARNING] Old LD_LIBRARY_PATH=${LD_LIBRARY_PATH}"
      echo "[WARNING] New LD_LIBRARY_PATH=${NEW_LD_LIBRARY_PATH}"
  fi
  export LD_LIBRARY_PATH="${NEW_LD_LIBRARY_PATH}"
}

set_ld_library_path
echo "[INFO] LD_LIBRARY_PATH=${LD_LIBRARY_PATH}"
