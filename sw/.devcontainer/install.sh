#!/usr/bin/env bash
set -euo pipefail

main() {
    set -x

    # install dependencies for building espflash from source.
    # at the time of writing the quickinstall binstall method is broken.
    apt-get update && apt-get install -y \
        libudev-dev \
        pkg-config

    # install rustup with nightly toolchain and rust-src
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
        -y --default-toolchain nightly --profile minimal --component rust-src

    . "$HOME/.cargo/env"

    # install binstall
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

    cargo binstall --no-confirm espflash

    rm -rf "$0"
}

main
