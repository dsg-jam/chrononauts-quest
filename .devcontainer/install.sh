#!/usr/bin/env bash
set -euo pipefail

install_rust() {
    # install dependencies for building espflash from source.
    # at the time of writing the quickinstall binstall method is broken.
    apt-get install -y \
        libudev-dev \
        pkg-config

    # install rustup with nightly toolchain and rust-src
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
        -y --default-toolchain nightly --profile minimal --component rust-src

    . "$HOME/.cargo/env"

    # install binstall
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

    cargo binstall --no-confirm espflash
}

install_jekyll() {
    apt-get install -y \
        ruby \
        ruby-dev \
        ruby-rubygems

    # https://github.com/ntkme/sass-embedded-host-ruby/issues/130
    gem install sass-embedded -v 1.62.1
    gem install bundler jekyll github-pages
}

main() {
    set -x

    export DEBIAN_FRONTEND=noninteractive

    apt-get update

    install_jekyll
    install_rust

    rm -rf "$0"
}

main
