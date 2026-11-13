//! Project automation: `cargo xtask <command>` (see `cargo xtask help`).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const APP_NAME: &str = "Hegel Remote";
const BUNDLE_ID: &str = "no.arnix.hegeldt";
const BIN_NAME: &str = "hegeldt";
const MAC_ARM: &str = "aarch64-apple-darwin";
const MAC_X86: &str = "x86_64-apple-darwin";

type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let cmd = env::args().nth(1).unwrap_or_else(|| "help".into());
    let result = match cmd.as_str() {
        "fmt" => cargo(&["fmt", "--all"]),
        "lint" => lint(),
        "test" => test(),
        "check" => check(),
        "bundle" => bundle(),
        "dist" => dist(),
        "run-mock" => cargo(&["run", "--bin", "mock_amp", "--", "50999"]),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command `{other}`; try `cargo xtask help`")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() {
    println!(
        "cargo xtask <command>\n\n\
         fmt        cargo fmt --all\n\
         lint       cargo clippy --workspace --all-targets -- -D warnings\n\
         test       cargo test --workspace\n\
         check      fmt --check + lint + test\n\
         bundle     (macOS) release build and target/release/bundle/{APP_NAME}.app\n\
         dist       distributable archive(s) in target/dist/ for this OS\n\
                    macOS: universal .app in .zip + .dmg; Linux: .tar.gz; Windows: .zip\n\
         run-mock   run the fake amplifier on 127.0.0.1:50999"
    );
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives in the workspace root")
        .to_path_buf()
}

fn run(cmd: &mut Command) -> Result {
    cmd.current_dir(root());
    println!("$ {cmd:?}");
    let status = cmd.status().map_err(|e| format!("{cmd:?}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{cmd:?} failed with {status}"))
    }
}

fn cargo(args: &[&str]) -> Result {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    run(Command::new(cargo).args(args))
}

fn lint() -> Result {
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])
}

fn test() -> Result {
    cargo(&["test", "--workspace"])
}

fn check() -> Result {
    cargo(&["fmt", "--all", "--check"])?;
    lint()?;
    test()?;
    println!("\nxtask: all checks passed");
    Ok(())
}

/// The hegeldt package version, read from the root Cargo.toml `[package]` table.
fn version() -> Result<String> {
    let text = fs::read_to_string(root().join("Cargo.toml")).map_err(|e| e.to_string())?;
    let mut in_package = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package
            && let Some(rest) = line.strip_prefix("version")
            && let Some((_, v)) = rest.split_once('=')
        {
            return Ok(v.trim().trim_matches('"').to_string());
        }
    }
    Err("no version in the [package] table of Cargo.toml".into())
}

fn info_plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundleExecutable</key><string>{BIN_NAME}</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundleIdentifier</key><string>{BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleName</key><string>{APP_NAME}</string>
    <key>CFBundleDisplayName</key><string>{APP_NAME}</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>{version}</string>
    <key>CFBundleVersion</key><string>{version}</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSLocalNetworkUsageDescription</key>
    <string>Hegel Remote controls a Hegel amplifier on your local network and searches for it.</string>
    <key>NSPrincipalClass</key><string>NSApplication</string>
</dict>
</plist>
"#
    )
}

fn io<T>(r: std::io::Result<T>, what: &str) -> Result<T> {
    r.map_err(|e| format!("{what}: {e}"))
}

fn require_macos(what: &str) -> Result {
    if cfg!(target_os = "macos") {
        Ok(())
    } else {
        Err(format!(
            "`{what}` builds a macOS app bundle and only works on macOS; \
             use `cargo xtask dist` for the packaging of this OS"
        ))
    }
}

/// `cargo build --release --bin hegeldt [--target T]`; returns the path of the binary.
fn release_build(target: Option<&str>) -> Result<PathBuf> {
    let mut args = vec!["build", "--release", "--bin", BIN_NAME];
    if let Some(t) = target {
        args.extend(["--target", t]);
    }
    cargo(&args)?;
    let mut dir = root().join("target");
    if let Some(t) = target {
        dir.push(t);
    }
    let exe = if cfg!(windows) {
        "hegeldt.exe"
    } else {
        BIN_NAME
    };
    Ok(dir.join("release").join(exe))
}

/// Creates `<parent>/Hegel Remote.app` around `binary` (replacing an existing one) and
/// ad-hoc signs it. Shared by `bundle` and `dist`.
fn assemble_app(binary: &Path, parent: &Path, version: &str) -> Result<PathBuf> {
    let root = root();
    let app = parent.join(format!("{APP_NAME}.app"));
    if app.exists() {
        io(fs::remove_dir_all(&app), "remove old bundle")?;
    }
    let macos = app.join("Contents/MacOS");
    let resources = app.join("Contents/Resources");
    io(fs::create_dir_all(&macos), "create bundle dirs")?;
    io(fs::create_dir_all(&resources), "create bundle dirs")?;
    io(fs::copy(binary, macos.join(BIN_NAME)), "copy binary")?;
    io(
        fs::copy(
            root.join("assets/AppIcon.icns"),
            resources.join("AppIcon.icns"),
        ),
        "copy icon",
    )?;
    io(
        fs::write(app.join("Contents/Info.plist"), info_plist(version)),
        "write Info.plist",
    )?;
    // Ad-hoc signature (after any lipo) so Gatekeeper/local network prompts treat the bundle as one unit.
    run(Command::new("codesign")
        .args(["--force", "--deep", "--sign", "-"])
        .arg(&app))?;
    Ok(app)
}

fn bundle() -> Result {
    require_macos("bundle")?;
    let binary = release_build(None)?;
    let parent = root().join("target/release/bundle");
    io(fs::create_dir_all(&parent), "create bundle dir")?;
    let app = assemble_app(&binary, &parent, &version()?)?;
    println!("\nxtask: created {}", app.display());
    Ok(())
}

fn dist() -> Result {
    let dist = root().join("target/dist");
    io(fs::create_dir_all(&dist), "create target/dist")?;
    let artifacts = if cfg!(target_os = "macos") {
        dist_macos(&dist)?
    } else if cfg!(windows) {
        dist_windows(&dist)?
    } else {
        dist_linux(&dist)?
    };
    println!("\nxtask: artifacts");
    for a in artifacts {
        let size = fs::metadata(&a).map(|m| m.len()).unwrap_or(0);
        println!("  {}  ({:.1} MB)", a.display(), size as f64 / 1_048_576.0);
    }
    Ok(())
}

/// Builds for both Mac architectures (falling back to the host one with a warning), merges
/// them with `lipo`, signs the bundle and writes a .zip and a .dmg.
fn dist_macos(dist: &Path) -> Result<Vec<PathBuf>> {
    let version = version()?;
    let host = if std::env::consts::ARCH == "aarch64" {
        MAC_ARM
    } else {
        MAC_X86
    };
    let other = if host == MAC_ARM { MAC_X86 } else { MAC_ARM };
    let host_bin = release_build(Some(host))?;
    let other_bin = run(Command::new("rustup").args(["target", "add", other]))
        .and_then(|()| release_build(Some(other)));

    let staging = dist.join("dmg-staging");
    if staging.exists() {
        io(fs::remove_dir_all(&staging), "clean staging")?;
    }
    io(fs::create_dir_all(&staging), "create staging")?;

    let (binary, label) = match other_bin {
        Ok(other_bin) => {
            let universal = dist.join(BIN_NAME);
            run(Command::new("lipo")
                .arg("-create")
                .arg(&host_bin)
                .arg(&other_bin)
                .arg("-output")
                .arg(&universal))?;
            (universal, "universal".to_string())
        }
        Err(e) => {
            eprintln!(
                "\nxtask: WARNING: could not build {other} ({e}).\n\
                 xtask: WARNING: falling back to a {} only build; it will not run on the other Mac architecture.\n",
                arch_name(host)
            );
            (host_bin, arch_name(host).to_string())
        }
    };

    let app = assemble_app(&binary, &staging, &version)?;
    let stem = format!("HegelRemote-{version}-macos-{label}");
    let zip = dist.join(format!("{stem}.zip"));
    let dmg = dist.join(format!("{stem}.dmg"));
    for f in [&zip, &dmg] {
        if f.exists() {
            io(fs::remove_file(f), "remove old artifact")?;
        }
    }
    // ditto keeps resource forks and the code signature intact (plain zip does not).
    run(Command::new("ditto")
        .args(["-c", "-k", "--keepParent"])
        .arg(&app)
        .arg(&zip))?;
    io(
        symlink("/Applications", &staging.join("Applications")),
        "link /Applications",
    )?;
    run(Command::new("hdiutil")
        .args(["create", "-volname", APP_NAME, "-srcfolder"])
        .arg(&staging)
        .args(["-ov", "-format", "UDZO"])
        .arg(&dmg))?;
    Ok(vec![zip, dmg])
}

#[cfg(unix)]
fn symlink(target: &str, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn symlink(_target: &str, _link: &Path) -> std::io::Result<()> {
    Err(std::io::Error::other("symlinks are only used on macOS"))
}

fn arch_name(target: &str) -> &str {
    target.split('-').next().unwrap_or(target)
}

/// UNTESTED (written on a Mac): tarball with binary, .desktop file, icon and an install note.
fn dist_linux(dist: &Path) -> Result<Vec<PathBuf>> {
    let version = version()?;
    let binary = release_build(None)?;
    let name = format!("HegelRemote-{version}-linux-{}", std::env::consts::ARCH);
    let dir = dist.join(&name);
    if dir.exists() {
        io(fs::remove_dir_all(&dir), "clean staging")?;
    }
    io(fs::create_dir_all(&dir), "create staging")?;
    io(fs::copy(&binary, dir.join(BIN_NAME)), "copy binary")?;
    io(
        fs::copy(
            root().join("assets/icon-256.png"),
            dir.join("hegel-remote.png"),
        ),
        "copy icon",
    )?;
    io(
        fs::write(
            dir.join("hegel-remote.desktop"),
            format!(
                "[Desktop Entry]\nType=Application\nName={APP_NAME}\n\
                 Comment=Remote control for Hegel amplifiers\n\
                 Exec=hegeldt\nIcon=hegel-remote\nTerminal=false\nCategories=AudioVideo;Audio;\n"
            ),
        ),
        "write .desktop file",
    )?;
    io(
        fs::write(
            dir.join("INSTALL.txt"),
            "Hegel Remote (Linux, untested build)\n\n\
             Needs a Wayland or X11 session with Vulkan (libvulkan1) and fontconfig.\n\n\
             Install for the current user:\n  \
             install -Dm755 hegeldt ~/.local/bin/hegeldt\n  \
             install -Dm644 hegel-remote.png ~/.local/share/icons/hicolor/256x256/apps/hegel-remote.png\n  \
             install -Dm644 hegel-remote.desktop ~/.local/share/applications/hegel-remote.desktop\n\n\
             Or just run ./hegeldt from this directory.\n",
        ),
        "write INSTALL.txt",
    )?;
    let tar = dist.join(format!("{name}.tar.gz"));
    run(Command::new("tar")
        .arg("-czf")
        .arg(&tar)
        .arg("-C")
        .arg(dist)
        .arg(&name))?;
    Ok(vec![tar])
}

/// UNTESTED (written on a Mac): zip with hegeldt.exe, made with PowerShell's Compress-Archive.
fn dist_windows(dist: &Path) -> Result<Vec<PathBuf>> {
    let version = version()?;
    let binary = release_build(None)?;
    let name = format!("HegelRemote-{version}-windows-{}", std::env::consts::ARCH);
    let dir = dist.join(&name);
    if dir.exists() {
        io(fs::remove_dir_all(&dir), "clean staging")?;
    }
    io(fs::create_dir_all(&dir), "create staging")?;
    io(fs::copy(&binary, dir.join("hegeldt.exe")), "copy binary")?;
    let zip = dist.join(format!("{name}.zip"));
    if zip.exists() {
        io(fs::remove_file(&zip), "remove old zip")?;
    }
    run(Command::new("powershell")
        .args(["-NoProfile", "-Command"])
        .arg(format!(
            "Compress-Archive -Path '{}' -DestinationPath '{}'",
            dir.display(),
            zip.display()
        )))?;
    Ok(vec![zip])
}
