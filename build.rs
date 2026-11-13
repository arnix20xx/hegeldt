//! Windows only: embed the application icon in the executable. A no-op everywhere else.
//! Untested: needs a Windows host with a resource compiler (rc.exe from the MSVC build tools).

fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/hegeldt.rc");
        println!("cargo:rerun-if-changed=assets/AppIcon.ico");
        embed_resource::compile("assets/hegeldt.rc", embed_resource::NONE)
            .manifest_optional()
            .unwrap();
    }
}
