#!/usr/bin/env bash
set -euo pipefail

_apt_install=(apt-get -o 'DPkg::Lock::Timeout=120' install -y)

install_rust() {
    # install dependencies for building espflash from source.
    # at the time of writing the quickinstall binstall method is broken.
    "${_apt_install[@]}" \
        libudev-dev \
        pkg-config

    # also required for esp
    "${_apt_install[@]}" \
        clang \
        libclang-dev

    # specific deps copied from <https://docs.espressif.com/projects/esp-idf/en/latest/esp32/get-started/linux-macos-setup.html#for-linux-users>
    "${_apt_install[@]}" \
        git wget flex bison gperf python3 python3-pip python3-venv cmake ninja-build ccache libffi-dev libssl-dev dfu-util libusb-1.0-0

    # install rustup with nightly toolchain and rust-src
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- \
        -y --default-toolchain nightly --profile minimal --component clippy,rust-src,rustfmt

    . "$HOME/.cargo/env"

    # install binstall
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

    cargo binstall --no-confirm \
        espflash \
        ldproxy
}

install_jekyll() {
    "${_apt_install[@]}" \
        ruby \
        ruby-dev \
        ruby-rubygems

    # https://github.com/ntkme/sass-embedded-host-ruby/issues/130
    gem install sass-embedded -v 1.62.1
    gem install bundler jekyll github-pages
}

install_gcloud() {
    "${_apt_install[@]}" \
        python3

    local installer_path="/tmp/install-gcloud.sh"
    curl https://sdk.cloud.google.com >"$installer_path"
    bash "$installer_path" --disable-prompts
    bash /root/google-cloud-sdk/install.sh --quiet --path-update=true

    rm -f "$installer_path"
}

install_node() {
    "${_apt_install[@]}" \
        nodejs \
        npm
}

main() {
    export DEBIAN_FRONTEND=noninteractive
    apt-get update

    set -x

    install_jekyll &
    local jekyll_pid=$!
    install_rust &
    local rust_pid=$!
    install_gcloud &
    local gcloud_pid=$!
    install_node &
    local node_pid=$!

    for name in "jekyll" "rust" "gcloud" "node"; do
        local pid_name pid
        pid_name="${name}_pid"
        pid="${!pid_name}"
        echo "Waiting for $name"
        wait "$pid"
    done

    rm -rf "$0"
}

main
