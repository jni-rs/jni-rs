#!/usr/bin/env bash
#
# This is a script for checking that various examples in the book are at least
# compile. Since we are building up small examples over time, we use feature
# flags to check ourselves along each step of the way.
#
# TODO: Figure out ignore specific warnings

set -euxo pipefail

cargo build

for i in {0..3}
do
    cargo build --features division_$i
done

cargo build --features division_complete
cargo build --features link_0
cargo build --features link_complete
cargo build --features counter_discussion
