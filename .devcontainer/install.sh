#!/usr/bin/env bash
set -euo pipefail

install_rust() {
    # install dependencies for building espflash from source.
    # at the time of writing the quickinstall binstall method is broken.
    apt-get install -y \
        libudev-dev \
        pkg-config

    # also required for esp
    apt-get install -y \
        clang \
        libclang-dev

    # specific deps copied from <https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/linux-macos-setup.html#for-linux-users>
    apt-get install -y \
        git wget flex bison gperf python3 python3-pip python3-venv cmake ninja-build ccache libffi-dev libssl-dev dfu-util libusb-1.0-0

    # install rustup with nightly toolchain and rust-src
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
        -y --default-toolchain nightly --profile minimal --component rust-src,rustfmt

    . "$HOME/.cargo/env"

    # install binstall
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

    cargo binstall --no-confirm \
        espflash \
        ldproxy
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
