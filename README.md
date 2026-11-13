# hegeldt

A macOS desktop remote for Hegel amplifiers (H95, H120, H190, H390, H590, Röst), written in Rust
with [GPUI](https://github.com/zed-industries/zed). It speaks the amplifier's IP control protocol
(TCP port 50001) and is modelled on the HegelRemote Android app.

## Build and run

```sh
cargo run                       # debug build of the app
cargo test                      # unit tests
cargo run --bin mock_amp -- 50999   # fake amplifier on 127.0.0.1:50999 for trying things out
```

Point the app at the mock by setting the address to `127.0.0.1:50999` in Settings.
Settings has a **Search for amplifiers** button (SSDP discovery, about 4 seconds); click a result
to use its address and model. `cargo run --example discover` runs the same search on the command line.

## xtask

```sh
cargo xtask fmt        # cargo fmt --all
cargo xtask lint       # clippy on the whole workspace, warnings are errors
cargo xtask test       # cargo test --workspace
cargo xtask check      # fmt --check + lint + test
cargo xtask bundle     # release build + target/release/bundle/Hegel Remote.app
cargo xtask run-mock   # same as running the mock amplifier above
```

## App bundle and icon

`cargo xtask bundle` creates `target/release/bundle/Hegel Remote.app` (bundle id
`no.arnix.hegeldt`, with `Info.plist`, icon and an ad-hoc code signature). Start it with
`open "target/release/bundle/Hegel Remote.app"`.

The icon source is `assets/icon.svg`; `assets/make-icns.sh` regenerates `assets/AppIcon.icns`
(needs `swiftc` and `iconutil`). UI icons are in `assets/icons/` and embedded in the binary.

## Menu bar mode

Turn on **Show in menu bar** in Settings. A status item appears in the menu bar and the Dock icon
is hidden. Left click shows or hides the window, right click (or ctrl-click) opens a menu with
Show / Hide and Quit. Closing the window only hides it; quit with Cmd-Q or from the status menu.

## Configuration

`~/Library/Application Support/hegeldt/config.json` (override with the `HEGELDT_CONFIG`
environment variable). It stores the address, amplifier model, maximum volume, menu bar mode and
the per-model input labels.

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| Up / Down | Volume up / down |
| M | Mute |
| P | Power |
| 1 to 9 | Select the n-th enabled input |
| Esc | Close the input dropdown |
| Cmd-, | Open / close settings |
| Cmd-Q | Quit |
