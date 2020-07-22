#!/usr/bin/env bash

# Fail immediately in case of errors and/or unset variables
set -eu -o pipefail

# Echo commands so that the progress can be seen in CI server logs.
set -x

# Check the shell scripts
shellcheck .travis/run_travis_job.sh
shellcheck test_profile

# Setup LD_LIBRARY_PATH for ITs
# shellcheck source=/dev/null
source test_profile

# Install clippy
if [[ ${TRAVIS_RUST_VERSION} == "nightly" ]];
then
    # Install nightly clippy
    rustup component add clippy --toolchain=nightly || cargo install --git https://github.com/rust-lang/rust-clippy/ --force clippy
else
    # Install stable clippy
    rustup component add clippy
fi
cargo clippy -V

# Install rustfmt
rustup component add rustfmt
rustfmt -V

echo 'Performing checks over the rust code'
# Check the formatting.
cargo fmt --all -- --check

# Run clippy static analysis.
cargo clippy --all --tests --all-features -- -D warnings

# Run tests with default features (stable-only)
if [[ ${TRAVIS_RUST_VERSION} == "stable" ]]; then
  cargo test
fi

# Run all tests with invocation feature (enables JavaVM ITs)
cargo test --features=invocation
