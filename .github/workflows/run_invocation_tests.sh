#!/usr/bin/env bash

# Fail immediately in case of errors and/or unset variables
set -eu -o pipefail

# Echo commands so that the progress can be seen in CI server logs.
set -x

# Check the shell scripts
shellcheck .github/workflows/run_invocation_tests.sh
shellcheck test_profile

# Setup LD_LIBRARY_PATH for ITs
# shellcheck source=/dev/null
source test_profile

# Run all tests with invocation feature (enables JavaVM ITs)
cargo test --features=invocation
