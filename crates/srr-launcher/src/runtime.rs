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

pub fn plan(source: &Listing, runtime_has: &dyn Fn(&str) -> bool, manifest: &Listing) -> Plan {
    let mut copy = Vec::new();
    let mut next = Listing::new();
    for (rel, stamp) in source {
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

pub fn check_separate(game: &Path, runtime: &Path) -> Result<(), String> {
    let game = fs::canonicalize(game).map_err(|e| format!("game folder {game:?}: {e}"))?;
    let runtime = fs::canonicalize(runtime).unwrap_or_else(|_| runtime.to_path_buf());
    if runtime.starts_with(&game) || game.starts_with(&runtime) {
        return Err("the runtime folder must be outside the game folder".into());
    }
    Ok(())
}

fn make_writable(path: &Path) -> io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    permissions.set_readonly(false);
    fs::set_permissions(path, permissions)
}

pub fn sync(game: &Path, runtime: &Path) -> io::Result<usize> {
    let source = list(game)?;
    fs::create_dir_all(runtime)?;
    let manifest_path = runtime.join(MANIFEST);
    let manifest = fs::read_to_string(&manifest_path)
        .map(|text| parse_manifest(&text))
        .unwrap_or_default();
    // A file the launcher renamed out of the way still counts as present.
    let present =
        |rel: &str| native(runtime, rel).is_file() || native(runtime, &disabled(rel)).is_file();
    let plan = plan(&source, &present, &manifest);
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
        assert!(plan(&source, &|_| true, &source).copy.is_empty());
        assert_eq!(plan(&source, &|_| false, &source).copy, ["SEGA Rally.exe"]);
        let changed = BTreeMap::from([("SEGA Rally.exe".into(), stamp(11, 20))]);
        assert_eq!(plan(&changed, &|_| true, &source).copy, ["SEGA Rally.exe"]);
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
        assert_eq!(sync(&game, &runtime).unwrap(), 2);
        fs::write(runtime.join("dxvk.conf"), b"ours").unwrap();
        assert_eq!(sync(&game, &runtime).unwrap(), 0);
        fs::write(game.join("SEGA Rally.exe"), b"two!").unwrap();
        assert_eq!(sync(&game, &runtime).unwrap(), 1);
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
            let copied = sync(&game, &runtime).unwrap();
            assert_eq!(copied, usize::from(launch == 0));
            assert_eq!(disable_bootup_videos(&runtime), usize::from(launch == 0));
            assert!(!native(&runtime, video).exists());
            assert!(native(&runtime, &disabled(video)).is_file());
        }
        // A changed source is copied fresh and switched off again.
        fs::write(native(&game, video), b"new video").unwrap();
        assert_eq!(sync(&game, &runtime).unwrap(), 1);
        assert_eq!(disable_bootup_videos(&runtime), 1);
        assert_eq!(
            fs::read(native(&runtime, &disabled(video))).unwrap(),
            b"new video"
        );
        fs::remove_dir_all(game).unwrap();
        fs::remove_dir_all(runtime).unwrap();
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
