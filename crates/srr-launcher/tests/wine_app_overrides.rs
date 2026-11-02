//! Under Wine (sdk/scripts/wine-test): the launch plan's per-exe DLL
//! overrides reach the registry Wine's loader reads, and taking them back
//! leaves the player's own values alone.
#![cfg(windows)]

use std::process::Command;

use rea_launcher_win32::wine::{self, DllMode};
use srr_launcher::edition::SSE1_EXE;

fn query(exe: &str, dll: &str) -> Option<String> {
    let key = format!(r"HKCU\Software\Wine\AppDefaults\{exe}\DllOverrides");
    let out = Command::new("reg")
        .args(["query", &key, "/v", dll])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.lines()
        .find(|line| line.trim_start().starts_with(dll))
        .map(|line| line.split_whitespace().last().unwrap_or("").to_string())
}

#[test]
fn per_exe_overrides_round_trip_in_the_registry() {
    if !wine::is_wine() {
        return; // native Windows: nothing is written
    }
    let ours = [
        ("d3d9", DllMode::NativeThenBuiltin),
        ("d3dx9_35", DllMode::NativeThenBuiltin),
    ];
    let player_key = format!(r"HKCU\Software\Wine\AppDefaults\{SSE1_EXE}\DllOverrides");
    // A value the player set stays.
    Command::new("reg")
        .args(["add", &player_key, "/v", "d3dx9_35", "/d", "builtin", "/f"])
        .status()
        .unwrap();
    let applied = wine::app_dll_overrides(SSE1_EXE, &ours).unwrap();
    assert_eq!(applied.written, ["d3d9"]);
    assert_eq!(query(SSE1_EXE, "d3d9").as_deref(), Some("native,builtin"));
    assert_eq!(query(SSE1_EXE, "d3dx9_35").as_deref(), Some("builtin"));
    // Taken back: ours goes, the player's stays.
    let removed = wine::remove_app_dll_overrides(SSE1_EXE, &ours).unwrap();
    assert_eq!(removed, ["d3d9"]);
    assert_eq!(query(SSE1_EXE, "d3d9"), None);
    assert_eq!(query(SSE1_EXE, "d3dx9_35").as_deref(), Some("builtin"));
}
