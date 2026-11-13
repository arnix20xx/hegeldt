# hegeldt

A desktop remote for Hegel amplifiers (H95, H120, H190, H390, H590, Röst), written in Rust
with [GPUI](https://github.com/zed-industries/zed). It speaks the amplifier's IP control protocol
(TCP port 50001) and is modelled on the HegelRemote Android app.

This is an unofficial hobby project. It is not affiliated with or endorsed by Hegel Music
Systems.

## Project status: take it and run with it

This repository is shared as-is, as a starting point for anyone who wants a desktop remote for
their Hegel amplifier. It is not an actively maintained project.

* **Fork it.** If you want to change, fix or extend something, fork the repository and work in
  your own copy. You do not need to ask.
* **No support.** I do not track bug reports or feature requests, and I do not review or merge
  pull requests. Issues and pull requests opened here will most likely go unanswered.
* **No promises.** It works for me with an H390 on macOS. The other models use the input tables
  from the Android app and are untested here; Linux and Windows are prepared but have never been
  built or run (see [Platforms](#platforms)).
* **Builds.** There are no official releases. Build it yourself with `cargo xtask dist` (see
  [Distribution](#distribution)).

If you make something better out of it, feel free to share your fork with the other users.

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
cargo xtask bundle     # macOS: release build + target/release/bundle/Hegel Remote.app
cargo xtask dist       # distributable archives in target/dist/ (see Distribution)
cargo xtask run-mock   # same as running the mock amplifier above
```

## Layout

* `crates/hegel-core`: protocol, model, connection, config, SSDP discovery and the mock amplifier.
  No GUI dependencies, so it type-checks for every platform (`cargo check -p hegel-core --target ...`).
  `mock_amp` and `cargo run --example discover` live here.
* `.` (`hegeldt`): the GPUI app (`src/`), plus the macOS-only menu bar code (`src/menubar.rs`).
* `xtask`: project automation.

## App bundle and icon

`cargo xtask bundle` creates `target/release/bundle/Hegel Remote.app` (bundle id
`no.arnix.hegeldt`, with `Info.plist`, icon and an ad-hoc code signature). Start it with
`open "target/release/bundle/Hegel Remote.app"`.

The icon source is `assets/icon.svg`; `assets/make-icns.sh` regenerates `assets/AppIcon.icns`
(needs `swiftc` and `iconutil`) and `assets/make-png-ico.sh` regenerates `assets/icon-256.png`
(Linux) and `assets/AppIcon.ico` (Windows). UI icons are in `assets/icons/` and embedded in the binary.

## Distribution

`cargo xtask dist` writes to `target/dist/`:

* macOS: `HegelRemote-<version>-macos-universal.zip` and `.dmg` containing an arm64 + x86_64
  `Hegel Remote.app` (both targets are built, merged with `lipo`, then ad-hoc signed). Needs
  `rustup target add aarch64-apple-darwin x86_64-apple-darwin` (done automatically). If the
  non-host architecture cannot be built, it warns and produces a host-arch-only artifact named
  with the arch instead of `universal`.
* Linux: `HegelRemote-<version>-linux-<arch>.tar.gz` (binary, `.desktop` file, 256px icon,
  install note). Untested, see Platforms.
* Windows: `HegelRemote-<version>-windows-<arch>.zip` with `hegeldt.exe`. Untested.

### Installing on another Mac

Open the `.dmg` and drag **Hegel Remote** to Applications (or unzip the `.zip`). The app is only
**ad-hoc signed and not notarized**, so Gatekeeper blocks a copy that was downloaded or
transferred with quarantine set ("cannot be opened because the developer cannot be verified").
Either right-click the app, choose Open, then Open again, or remove the quarantine flag:

```sh
xattr -dr com.apple.quarantine "/Applications/Hegel Remote.app"
```

On first start macOS asks for local network access; allow it, otherwise the amplifier cannot be
reached.

Proper distribution without those warnings needs an Apple Developer Program membership: sign with
a "Developer ID Application" certificate (`codesign --options runtime --timestamp`), submit the
zip/dmg with `xcrun notarytool submit --wait`, then `xcrun stapler staple` the app/dmg. Not
implemented here.

## Platforms

| Platform | Status |
| --- | --- |
| macOS (Apple silicon and Intel) | Supported and tested. |
| Linux | Prepared, **never built or run**. |
| Windows | Prepared, **never built or run**. |

What was prepared for Linux/Windows: shortcuts use Cmd on macOS and Ctrl elsewhere (`secondary-`
bindings), `src/menubar.rs` is a no-op stub off macOS and the setting is hidden there, the config
lives in the platform config dir (`dirs::config_dir()`: `~/.config/hegeldt/config.json`,
`%APPDATA%\hegeldt\config.json`), GPUI features are chosen per target (X11 + Wayland on Linux),
the Windows release build uses the GUI subsystem (no console window; debug builds keep the
console for logs) and embeds `assets/AppIcon.ico`, and `cargo xtask dist` has Linux and Windows
packaging.

What was verified from a Mac: `hegel-core` passes `cargo check --all-targets` for
`x86_64-unknown-linux-gnu` and `x86_64-pc-windows-msvc`. The GUI crate cannot be built off-platform
here: Linux stops in `freetype-sys` (needs a Linux sysroot/C headers), Windows in `psm` (needs
MSVC `lib.exe`). Nothing of the GUI, the packaging or `.github/workflows/ci.yml` has run on those
systems.

Known gaps:

* No menu bar / tray mode on Linux and Windows; closing the window quits the app.
* Packaging (`.tar.gz`, `.zip`) is unverified; no installer, no code signing, no AppImage/MSI.
* Linux needs system packages for building (`build-essential pkg-config libfontconfig-dev
  libwayland-dev libx11-xcb-dev libxkbcommon-dev libxkbcommon-x11-dev`) and Vulkan
  (`libvulkan1` plus a driver) at run time; GPUI renders through Vulkan/wgpu.
* Windows needs the MSVC build tools (Rust `x86_64-pc-windows-msvc`); the icon is embedded by
  `build.rs` through the `embed-resource` crate, which needs `rc.exe`.
* `Cargo.lock` pins `libc` to 0.2.189: `xattr` 0.2.3 (via GPUI's `http_client`) does not compile
  against newer libc on Linux (`ENOATTR`). Do not run a blanket `cargo update`.
* Window sizing, fonts, key handling and the SSDP socket code are untested off macOS.

To try it: on a Linux or Windows machine with the prerequisites above, `cargo xtask check` and
`cargo xtask dist`, or `cargo run`; or run the (equally untested)
`.github/workflows/ci.yml` workflow by hand from the repository's Actions tab ("Run workflow").
It only runs when started manually, and builds all three platforms and uploads the artifacts.

## Menu bar mode (macOS only)

Turn on **Show in menu bar** in Settings. A status item appears in the menu bar and the Dock icon
is hidden. Left click shows or hides the window, right click (or ctrl-click) opens a menu with
Show / Hide and Quit. Closing the window only hides it; quit with Cmd-Q or from the status menu.

## Configuration

`~/Library/Application Support/hegeldt/config.json` on macOS, `~/.config/hegeldt/config.json` on
Linux, `%APPDATA%\hegeldt\config.json` on Windows (override with the `HEGELDT_CONFIG`
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
| Cmd-, / Ctrl-, | Open / close settings |
| Cmd-Q / Ctrl-Q | Quit |

(Cmd on macOS, Ctrl on Linux and Windows.)

## License

MIT, see [LICENSE](LICENSE). GPUI and the other dependencies have their own licenses.
