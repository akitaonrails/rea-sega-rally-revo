//! The Windows executable's version information, icon and manifest
//! (rea-launcher-build): a named, versioned program with the launcher's
//! icon, instead of an anonymous one that antivirus heuristics distrust
//! (sdk/docs/STANDARDS.md, "Antivirus"). Does nothing for other targets.
fn main() {
    rea_launcher_build::Resources::new(
        "Sega Rally Revo Launcher",
        "Launcher for Sega Rally Revo on modern systems",
    )
    .company("AkitaOnRails")
    .file_name("SRRLauncher.exe")
    .icon("../../packaging/appimage/srr-launcher.png")
    .embed()
    .unwrap();
}
