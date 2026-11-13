//! Project automation: `cargo xtask <command>` (see `cargo xtask help`).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const APP_NAME: &str = "Hegel Remote";
const BUNDLE_ID: &str = "no.arnix.hegeldt";
const BIN_NAME: &str = "hegeldt";

type Result<T = ()> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let cmd = env::args().nth(1).unwrap_or_else(|| "help".into());
    let result = match cmd.as_str() {
        "fmt" => cargo(&["fmt", "--all"]),
        "lint" => lint(),
        "test" => test(),
        "check" => check(),
        "bundle" => bundle(),
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
         bundle     release build and target/release/bundle/{APP_NAME}.app\n\
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

fn info_plist() -> String {
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
"#,
        version = env!("CARGO_PKG_VERSION")
    )
}

fn bundle() -> Result {
    cargo(&["build", "--release", "--bin", BIN_NAME])?;
    let root = root();
    let app = root
        .join("target/release/bundle")
        .join(format!("{APP_NAME}.app"));
    if app.exists() {
        fs::remove_dir_all(&app).map_err(|e| e.to_string())?;
    }
    let macos = app.join("Contents/MacOS");
    let resources = app.join("Contents/Resources");
    fs::create_dir_all(&macos).map_err(|e| e.to_string())?;
    fs::create_dir_all(&resources).map_err(|e| e.to_string())?;
    fs::copy(
        root.join("target/release").join(BIN_NAME),
        macos.join(BIN_NAME),
    )
    .map_err(|e| format!("copy binary: {e}"))?;
    fs::copy(
        root.join("assets/AppIcon.icns"),
        resources.join("AppIcon.icns"),
    )
    .map_err(|e| format!("copy icon: {e}"))?;
    fs::write(app.join("Contents/Info.plist"), info_plist()).map_err(|e| e.to_string())?;
    // Ad-hoc signature so Gatekeeper/local network prompts treat the bundle as one unit.
    if cfg!(target_os = "macos") {
        let _ = Command::new("codesign")
            .args(["--force", "--deep", "--sign", "-"])
            .arg(&app)
            .status();
    }
    println!("\nxtask: created {}", app.display());
    Ok(())
}
