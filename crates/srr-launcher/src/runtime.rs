//! The private runtime copy of the game folder: Sega Rally Revo's
//! [`Policy`] on the SDK's runtime engine (`rea_launcher::runtime`), which
//! lists, plans, copies as whole-file swaps and keeps the manifest.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rea_launcher::paths::native;
use rea_launcher::runtime::{self, Policy, Stamp, SyncReport};

/// The manifest of the last copy (`size\tmtime\trel` lines, the SDK's
/// format, which this launcher already wrote).
pub const MANIFEST: &str = ".srr-manifest";

/// The marker `SRRLauncher` leaves next to the DXVK `d3d9.dll` it installs.
pub const DXVK_MARKER: &str = ".srr-dxvk";

/// The record of the `config.ini` values the launcher wrote
/// (`rea_launcher::ini::Owned`).
pub const CONFIG_OWNED: &str = ".srr-config-owned";

/// Files the launcher owns in the runtime copy and a sync must never
/// overwrite: the `dxvk.conf` it writes, the close hook DLL the runtime
/// exes import and its own bookkeeping, always;
/// `d3d9.dll` only while the launcher manages DXVK there (`dxvk_managed`:
/// `--linux`, or its marker is present). On native Windows a player's own
/// `d3d9.dll` (ReShade, dgVoodoo, DXVK for Windows) is copied like any
/// other game file.
pub fn is_owned(rel: &str, dxvk_managed: bool) -> bool {
    const OWNED: [&str; 6] = [
        "dxvk.conf",
        crate::registry::HOOK_DLL,
        MANIFEST,
        DXVK_MARKER,
        CONFIG_OWNED,
        crate::directx::LEDGER,
    ];
    OWNED.iter().any(|name| rel.eq_ignore_ascii_case(name))
        || (dxvk_managed && rel.eq_ignore_ascii_case("d3d9.dll"))
}

/// What Sega Rally Revo's runtime copy needs: nothing is linked (the exes
/// are patched there), the launcher's own files are never copied, and a
/// boot video the launcher switched off still counts as present.
pub struct Srr {
    pub dxvk_managed: bool,
}

impl Policy for Srr {
    fn owned(&self, rel: &str, _source: Stamp) -> bool {
        is_owned(rel, self.dxvk_managed)
    }

    fn present(&self, _rel: &str, path: &Path) -> bool {
        let regular = |path: &Path| fs::symlink_metadata(path).is_ok_and(|m| m.is_file());
        regular(path) || regular(&disabled_path(path))
    }
}

/// `path` with the switched-off suffix.
fn disabled_path(path: &Path) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(crate::gamecfg::DISABLED_SUFFIX);
    name.into()
}

/// Brings the runtime copy up to date with the game folder. `linux` means
/// the launcher installs its DXVK `d3d9.dll` in this run.
pub fn sync(game: &Path, runtime: &Path, linux: bool) -> io::Result<SyncReport> {
    let dxvk_managed = linux || runtime.join(DXVK_MARKER).is_file();
    runtime::sync(game, runtime, &Srr { dxvk_managed }, MANIFEST)
}

/// The name a runtime file gets when the launcher switches it off.
pub fn disabled(rel: &str) -> String {
    format!("{rel}{}", crate::gamecfg::DISABLED_SUFFIX)
}

/// Rename the bootup logo videos out of the way: Wine's Quartz path stalls
/// on them and they only delay startup. Safe to run on every launch; a
/// fresh copy synced over a changed source is switched off again.
pub fn disable_bootup_videos(runtime: &Path) -> usize {
    let mut renamed = 0;
    for rel in crate::gamecfg::BOOTUP_VIDEOS {
        let video = native(runtime, rel);
        if video.is_file() && fs::rename(&video, native(runtime, &disabled(rel))).is_ok() {
            renamed += 1;
        }
    }
    renamed
}

#[cfg(test)]
mod tests {
    use super::*;
    use rea_launcher::runtime::{Listing, plan};
    use std::collections::BTreeMap;

    fn stamp(size: u64, mtime: u64) -> Stamp {
        Stamp { size, mtime }
    }

    fn temp(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("srr-launcher-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn sync_copies_changes_and_preserves_generated_files() {
        let game = temp("game");
        let runtime = temp("runtime");
        fs::create_dir_all(game.join("Bootup/video/Rally")).unwrap();
        fs::write(game.join("SEGA Rally.exe"), b"one").unwrap();
        fs::write(game.join("Bootup/video/Rally/SEGA_Logo720p.wmv"), b"vid").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 2);
        fs::write(runtime.join("dxvk.conf"), b"ours").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 0);
        fs::write(game.join("SEGA Rally.exe"), b"two!").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 1);
        assert_eq!(fs::read(runtime.join("SEGA Rally.exe")).unwrap(), b"two!");
        assert_eq!(fs::read(runtime.join("dxvk.conf")).unwrap(), b"ours");
        // Nothing is linked: the exes are patched in the runtime.
        assert_eq!(sync(&game, &runtime, false).unwrap().linked, 0);
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn a_manifest_written_by_an_earlier_release_keeps_working() {
        let game = temp("old-manifest-game");
        let runtime = temp("old-manifest-runtime");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        fs::write(game.join("a file.txt"), b"data").unwrap();
        fs::write(runtime.join("a file.txt"), b"data").unwrap();
        let source = runtime::list(&game).unwrap()["a file.txt"];
        // the format earlier releases wrote: size, mtime, path
        fs::write(
            runtime.join(MANIFEST),
            format!("{}\t{}\ta file.txt\n", source.size, source.mtime),
        )
        .unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 0);
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn disabled_videos_stay_disabled_across_launches() {
        let game = temp("videos-game");
        let runtime = temp("videos-runtime");
        let video = crate::gamecfg::BOOTUP_VIDEOS[0];
        fs::create_dir_all(native(&game, video).parent().unwrap()).unwrap();
        fs::write(native(&game, video), b"vid").unwrap();
        for launch in 0..2 {
            let copied = sync(&game, &runtime, false).unwrap().brought;
            assert_eq!(copied, usize::from(launch == 0));
            assert_eq!(disable_bootup_videos(&runtime), usize::from(launch == 0));
            assert!(!native(&runtime, video).exists());
            assert!(native(&runtime, &disabled(video)).is_file());
        }
        // A changed source is copied fresh and switched off again.
        fs::write(native(&game, video), b"new video").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 1);
        assert_eq!(disable_bootup_videos(&runtime), 1);
        assert_eq!(
            fs::read(native(&runtime, &disabled(video))).unwrap(),
            b"new video"
        );
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn launcher_owned_files_are_never_synced() {
        let source: Listing = BTreeMap::from([
            ("d3d9.dll".into(), stamp(1, 1)),
            ("DXVK.conf".into(), stamp(1, 1)),
            ("SEGA Rally.exe".into(), stamp(1, 1)),
        ]);
        // with our DXVK: d3d9.dll is ours
        let managed = plan(
            &source,
            &|_| false,
            &Listing::new(),
            &Srr { dxvk_managed: true },
        );
        assert_eq!(managed.copy, ["SEGA Rally.exe"]);
        // native Windows: the player's d3d9.dll is a game file like any other
        let windows = plan(
            &source,
            &|_| false,
            &Listing::new(),
            &Srr {
                dxvk_managed: false,
            },
        );
        assert_eq!(windows.copy, ["SEGA Rally.exe", "d3d9.dll"]);
        for managed in [false, true] {
            assert!(is_owned(MANIFEST, managed) && is_owned(DXVK_MARKER, managed));
            assert!(is_owned("dxvk.conf", managed) && is_owned("rea_close_hook.dll", managed));
        }
        assert!(
            !is_owned("Data/d3d9.dll", true),
            "only the top-level slot is ours"
        );
    }

    #[test]
    fn a_sync_keeps_the_installed_dxvk() {
        let game = temp("owned-game");
        let runtime = temp("owned-runtime");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("d3d9.dll"), b"player's wrapper").unwrap();
        fs::write(game.join("SEGA Rally.exe"), b"exe").unwrap();
        assert_eq!(sync(&game, &runtime, true).unwrap().brought, 1);
        fs::write(runtime.join("d3d9.dll"), b"dxvk").unwrap();
        fs::write(runtime.join(DXVK_MARKER), b"").unwrap();
        fs::write(game.join("d3d9.dll"), b"changed wrapper").unwrap();
        // the marker alone keeps it ours, even without --linux
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 0);
        assert_eq!(fs::read(runtime.join("d3d9.dll")).unwrap(), b"dxvk");
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn a_windows_players_own_d3d9_is_copied_and_updated() {
        let game = temp("windows-game");
        let runtime = temp("windows-runtime");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("d3d9.dll"), b"reshade").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 1);
        assert_eq!(fs::read(runtime.join("d3d9.dll")).unwrap(), b"reshade");
        fs::write(game.join("d3d9.dll"), b"reshade 2").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap().brought, 1);
        assert_eq!(fs::read(runtime.join("d3d9.dll")).unwrap(), b"reshade 2");
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn overlapping_folders_are_refused() {
        // The SDK check the launcher calls before every sync.
        let game = temp("overlap");
        fs::create_dir_all(&game).unwrap();
        assert!(rea_launcher::paths::check_separate(&game, &game.join("runtime")).is_err());
        assert!(rea_launcher::paths::check_separate(&game, &temp("elsewhere")).is_ok());
        fs::remove_dir_all(game).unwrap();
    }
}
