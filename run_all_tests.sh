#!/usr/bin/env bash
# Runs all Rust and Java tests

# Fail immediately in case of errors and/or unset variables
set -eu -o pipefail

# Set required environment variables:
source tests_profile.sh

# Run all Rust tests
#
# --all-features is used to enable 'invocation' and 'backtrace' feature in 'jni'
# crate because passing them directly is not supported:
# https://github.com/rust-lang/cargo/issues/5364
cargo test --all-features

# Run Java integration tests. Since benchmarks do not have tests, we skip them.
mvn verify --projects java-tests --also-make
