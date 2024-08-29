# `chrononauts-board` firmware

## Flashing firmware

- [Install Rust](https://rustup.rs)
- [Setup ESP32 RISC-V target](https://esp-rs.github.io/book/installation/riscv.html)
- Install espflash: `cargo install espflash`
- Flash firmware: `cargo run --release`

## Regenerating `style.css`

- Install depdendencies: `npm i`
- Regenerate stylesheet: `npx tailwindcss -i input.css -o src/web/style.css --minify` (add `--watch` to wait for changes)

## Options
Compile firmware with custom Wi-Fi config:
- `SSID="name" SSID_PASSWORD="password" cargo build --release`

## Credits
- Boilerplate [`esp32-captive-portal`](https://github.com/dotcypress/esp32-captive-portal)
