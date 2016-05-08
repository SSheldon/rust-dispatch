#! /usr/bin/env sh

set -eu

if [ -n "$IOS_ARCHS" ]; then
    ./rust-test-ios
elif [ "$TRAVIS_OS_NAME" = "linux" ]; then
    cargo build --verbose
    # The Ubuntu libdispatch doesn't seem to be in great shape,
    # so just run a quick smoke test of the basic cases.
    cargo test --verbose test_serial_queue
else
    cargo build --verbose
    cargo test --verbose
fi
