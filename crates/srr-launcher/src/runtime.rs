use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MANIFEST: &str = ".srr-manifest";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub size: u64,
    pub mtime: u64,
}

pub type Listing = BTreeMap<String, Stamp>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub copy: Vec<String>,
    pub manifest: Listing,
}

/// The marker `SRRLauncher` leaves next to the DXVK `d3d9.dll` it installs.
pub const DXVK_MARKER: &str = ".srr-dxvk";

/// Files the launcher owns in the runtime copy and a sync must never
/// overwrite: the `dxvk.conf` it writes and its own bookkeeping, always;
/// `d3d9.dll` only while the launcher manages DXVK there (`dxvk_managed`:
/// `--linux`, or its marker is present). On native Windows a player's own
/// `d3d9.dll` (ReShade, dgVoodoo, DXVK for Windows) is copied like any
/// other game file.
pub fn is_owned(rel: &str, dxvk_managed: bool) -> bool {
    const OWNED: [&str; 3] = ["dxvk.conf", MANIFEST, DXVK_MARKER];
    OWNED.iter().any(|name| rel.eq_ignore_ascii_case(name))
        || (dxvk_managed && rel.eq_ignore_ascii_case("d3d9.dll"))
}

/// Decides which files to copy: a file missing from the runtime or changed
/// in the source since the last copy (per the manifest), never one the
/// launcher owns.
pub fn plan(
    source: &Listing,
    runtime_has: &dyn Fn(&str) -> bool,
    manifest: &Listing,
    dxvk_managed: bool,
) -> Plan {
    let mut copy = Vec::new();
    let mut next = Listing::new();
    for (rel, stamp) in source {
        if is_owned(rel, dxvk_managed) {
            continue;
        }
        let present = runtime_has(rel);
        let changed = manifest.get(rel) != Some(stamp);
        if !present || changed {
            copy.push(rel.clone());
        }
        next.insert(rel.clone(), *stamp);
    }
    Plan {
        copy,
        manifest: next,
    }
}

pub fn parse_manifest(text: &str) -> Listing {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(3, '\t');
            let size = parts.next()?.parse().ok()?;
            let mtime = parts.next()?.parse().ok()?;
            Some((parts.next()?.to_owned(), Stamp { size, mtime }))
        })
        .collect()
}

pub fn format_manifest(listing: &Listing) -> String {
    listing
        .iter()
        .map(|(rel, s)| format!("{}\t{}\t{}\n", s.size, s.mtime, rel))
        .collect()
}

fn mtime_secs(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs())
}

pub fn list(root: &Path) -> io::Result<Listing> {
    fn walk(root: &Path, dir: &Path, out: &mut Listing) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let meta = entry.metadata()?;
            if meta.is_dir() {
                walk(root, &path, out)?;
            } else if meta.is_file() {
                let rel = path
                    .strip_prefix(root)
                    .expect("walk stays below root")
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(
                    rel,
                    Stamp {
                        size: meta.len(),
                        mtime: mtime_secs(&meta),
                    },
                );
            }
        }
        Ok(())
    }
    let mut out = Listing::new();
    walk(root, root, &mut out)?;
    Ok(out)
}

pub fn native(root: &Path, rel: &str) -> PathBuf {
    rel.split('/')
        .fold(root.to_path_buf(), |p, part| p.join(part))
}

/// `.` and `..` removed lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() && !normalized.has_root() {
                    normalized.push("..");
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

/// Resolves a path even when its final components do not exist yet (a
/// first-run runtime folder). Canonicalizing the nearest existing ancestor
/// resolves symlinks and Windows path aliases; the missing suffix is then
/// normalized lexically. Ported from the CMR04 launcher.
pub fn resolve(path: &Path, cwd: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };
    let mut ancestor = absolute.as_path();
    let mut suffix = Vec::new();
    loop {
        match fs::canonicalize(ancestor) {
            Ok(mut resolved) => {
                for component in suffix.iter().rev() {
                    resolved.push(component);
                }
                return Ok(normalize(&resolved));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match ancestor.components().next_back() {
                    Some(std::path::Component::Normal(name)) => suffix.push(name.to_os_string()),
                    Some(std::path::Component::ParentDir) => suffix.push("..".into()),
                    Some(std::path::Component::CurDir) => {}
                    _ => return Err(format!("cannot resolve {}: {error}", path.display())),
                }
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| format!("cannot resolve {}: {error}", path.display()))?;
            }
            Err(error) => return Err(format!("cannot resolve {}: {error}", path.display())),
        }
    }
}

fn check_separate_from(game: &Path, runtime: &Path, cwd: &Path) -> Result<(), String> {
    let absolute_game = if game.is_absolute() {
        game.to_path_buf()
    } else {
        cwd.join(game)
    };
    let game = fs::canonicalize(&absolute_game)
        .map_err(|error| format!("game folder {}: {error}", game.display()))?;
    let runtime = resolve(runtime, cwd)?;
    if runtime.starts_with(&game) || game.starts_with(&runtime) {
        return Err("the runtime folder must be outside the game folder".into());
    }
    Ok(())
}

/// Refuses a runtime folder inside the game folder (or the reverse), even
/// when the runtime does not exist yet or is reached through `..` or a
/// symlink: the player's game folder is read only.
pub fn check_separate(game: &Path, runtime: &Path) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|error| format!("current directory: {error}"))?;
    check_separate_from(game, runtime, &cwd)
}

fn make_writable(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions)
}

/// Brings the runtime copy up to date with the game folder. `linux` means
/// the launcher installs its DXVK `d3d9.dll` in this run.
pub fn sync(game: &Path, runtime: &Path, linux: bool) -> io::Result<usize> {
    let source = list(game)?;
    fs::create_dir_all(runtime)?;
    let dxvk_managed = linux || runtime.join(DXVK_MARKER).is_file();
    let manifest_path = runtime.join(MANIFEST);
    let manifest = fs::read_to_string(&manifest_path)
        .map(|text| parse_manifest(&text))
        .unwrap_or_default();
    // A file the launcher renamed out of the way still counts as present.
    let present =
        |rel: &str| native(runtime, rel).is_file() || native(runtime, &disabled(rel)).is_file();
    let plan = plan(&source, &present, &manifest, dxvk_managed);
    for rel in &plan.copy {
        let from = native(game, rel);
        let to = native(runtime, rel);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir)?;
        }
        if to.exists() {
            make_writable(&to)?;
        }
        fs::copy(&from, &to)?;
        make_writable(&to)?;
        let stamp = source[rel];
        fs::OpenOptions::new()
            .write(true)
            .open(&to)?
            .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(stamp.mtime))?;
    }
    fs::write(&manifest_path, format_manifest(&plan.manifest))?;
    Ok(plan.copy.len())
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
    fn unchanged_files_are_not_recopied() {
        let source = BTreeMap::from([("SEGA Rally.exe".into(), stamp(10, 20))]);
        assert!(plan(&source, &|_| true, &source, false).copy.is_empty());
        assert_eq!(
            plan(&source, &|_| false, &source, false).copy,
            ["SEGA Rally.exe"]
        );
        let changed = BTreeMap::from([("SEGA Rally.exe".into(), stamp(11, 20))]);
        assert_eq!(
            plan(&changed, &|_| true, &source, false).copy,
            ["SEGA Rally.exe"]
        );
    }

    #[test]
    fn manifest_round_trips_with_spaces() {
        let listing = BTreeMap::from([("folder/a file.ini".into(), stamp(10, 20))]);
        assert_eq!(parse_manifest(&format_manifest(&listing)), listing);
    }

    #[test]
    fn sync_copies_changes_and_preserves_generated_files() {
        let game = temp("game");
        let runtime = temp("runtime");
        fs::create_dir_all(game.join("Bootup/video/Rally")).unwrap();
        fs::write(game.join("SEGA Rally.exe"), b"one").unwrap();
        fs::write(game.join("Bootup/video/Rally/SEGA_Logo720p.wmv"), b"vid").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap(), 2);
        fs::write(runtime.join("dxvk.conf"), b"ours").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap(), 0);
        fs::write(game.join("SEGA Rally.exe"), b"two!").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap(), 1);
        assert_eq!(fs::read(runtime.join("dxvk.conf")).unwrap(), b"ours");
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
            let copied = sync(&game, &runtime, false).unwrap();
            assert_eq!(copied, usize::from(launch == 0));
            assert_eq!(disable_bootup_videos(&runtime), usize::from(launch == 0));
            assert!(!native(&runtime, video).exists());
            assert!(native(&runtime, &disabled(video)).is_file());
        }
        // A changed source is copied fresh and switched off again.
        fs::write(native(&game, video), b"new video").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap(), 1);
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
        let source = BTreeMap::from([
            ("d3d9.dll".into(), stamp(1, 1)),
            ("DXVK.conf".into(), stamp(1, 1)),
            ("SEGA Rally.exe".into(), stamp(1, 1)),
        ]);
        // with our DXVK: d3d9.dll is ours
        let managed = plan(&source, &|_| false, &Listing::new(), true);
        assert_eq!(managed.copy, ["SEGA Rally.exe"]);
        // native Windows: the player's d3d9.dll is a game file like any other
        let windows = plan(&source, &|_| false, &Listing::new(), false);
        assert_eq!(windows.copy, ["SEGA Rally.exe", "d3d9.dll"]);
        for managed in [false, true] {
            assert!(is_owned(MANIFEST, managed) && is_owned(DXVK_MARKER, managed));
            assert!(is_owned("dxvk.conf", managed));
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
        assert_eq!(sync(&game, &runtime, true).unwrap(), 1);
        fs::write(runtime.join("d3d9.dll"), b"dxvk").unwrap();
        fs::write(runtime.join(DXVK_MARKER), b"").unwrap();
        fs::write(game.join("d3d9.dll"), b"changed wrapper").unwrap();
        // the marker alone keeps it ours, even without --linux
        assert_eq!(sync(&game, &runtime, false).unwrap(), 0);
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
        assert_eq!(sync(&game, &runtime, false).unwrap(), 1);
        assert_eq!(fs::read(runtime.join("d3d9.dll")).unwrap(), b"reshade");
        fs::write(game.join("d3d9.dll"), b"reshade 2").unwrap();
        assert_eq!(sync(&game, &runtime, false).unwrap(), 1);
        assert_eq!(fs::read(runtime.join("d3d9.dll")).unwrap(), b"reshade 2");
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
    }

    #[test]
    fn missing_relative_and_dotdot_runtimes_inside_the_game_are_refused() {
        let game = temp("relative-overlap");
        fs::create_dir_all(&game).unwrap();
        let game = fs::canonicalize(&game).unwrap();
        let accepted = resolve(Path::new("../outside-runtime"), &game).unwrap();
        assert!(accepted.is_absolute());
        assert_eq!(accepted, game.parent().unwrap().join("outside-runtime"));
        assert!(!accepted.exists());
        assert!(check_separate_from(&game, Path::new("child"), &game).is_err());
        assert!(check_separate_from(&game, Path::new("missing/../child"), &game).is_err());
        assert!(check_separate_from(&game, &game.join("a/b/../c"), &game).is_err());
        assert!(!game.join("child").exists());
        assert!(check_separate_from(&game, Path::new("../outside"), &game).is_ok());
        fs::remove_dir_all(game).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_into_the_game_folder_is_refused() {
        let game = temp("link-game");
        let links = temp("link-dir");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&links).unwrap();
        std::os::unix::fs::symlink(&game, links.join("alias")).unwrap();
        // the runtime does not exist yet; only its parent is a link
        assert!(check_separate(&game, &links.join("alias").join("runtime")).is_err());
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(links).unwrap();
    }

    #[test]
    fn overlapping_folders_are_refused() {
        let game = temp("overlap");
        fs::create_dir_all(&game).unwrap();
        assert!(check_separate(&game, &game.join("runtime")).is_err());
        assert!(check_separate(&game, &temp("elsewhere")).is_ok());
        fs::remove_dir_all(game).unwrap();
    }
}
