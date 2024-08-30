# `chrononauts-board` firmware

## Flashing firmware

- Erase flash (to also void the NVS partition): `espflash erase-flash`
- Flash firmware:
    - WiFi-connected board: `CHRONONAUTS_ID=1 cargo run -- --no-skip`
    - Other board: `CHRONONAUTS_ID=0 cargo run -- --no-skip`

The Access Point credentials are (by default):
- SSID: `Chrononauts-Board`
- Password: `paradoxium2`

## Regenerating `style.css`

- Install depdendencies: `npm i`
- Regenerate stylesheet: `npx tailwindcss -i input.css -o src/web/style.css --minify` (add `--watch` to wait for changes)

## Options
Compile firmware with custom Wi-Fi config:
- `SSID="name" SSID_PASSWORD="password" cargo build --release`

## Credits
- Boilerplate [`esp32-captive-portal`](https://github.com/dotcypress/esp32-captive-portal)
