#!/usr/bin/env bash

# Fail immediately in case of errors and/or unset variables
set -eu -o pipefail

# Echo commands so that the progress can be seen in CI server logs.
set -x

# Install cargo-audit if it's not already.
cargo audit --version || cargo install cargo-audit --force

# Install clippy and rustfmt.
rustup component add clippy
rustup component add rustfmt
rustfmt -V
cargo clippy -V

echo 'Performing checks over the rust code'
# Check the formatting.
cargo fmt --all -- --check

# Run clippy static analysis.
cargo clippy --all --tests --all-features -- -D warnings

# Run audit of vulnerable dependencies.
cargo audit

# Run all tests
JAVA_HOME="${JAVA_HOME:-$(java -XshowSettings:properties -version \
    2>&1 > /dev/null |\
    grep 'java.home' |\
    awk '{print $3}')}"
LIBJVM_PATH="$(find -L ${JAVA_HOME} -type f -name libjvm.* | xargs -n1 dirname)"

export LD_LIBRARY_PATH="${LIBJVM_PATH}"

cargo test --features=backtrace,invocation