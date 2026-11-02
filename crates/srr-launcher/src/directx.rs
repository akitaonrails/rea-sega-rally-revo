//! Microsoft's DirectX 9 extension libraries (`d3dx9_24.dll` …
//! `d3dx9_43.dll`, `D3DCompiler_*.dll`), fetched on the player's request.
//!
//! The game links one of these; Windows 10/11 do not ship them, and the
//! game's own installer put them in with Microsoft's DirectX runtime. The
//! community Linux setup installed them with `winetricks d3dx9` to stop a
//! main-menu crash (Wine's built-in copies are what runs otherwise).
//!
//! They cannot be bundled: the only package holding them, Microsoft's
//! *DirectX End-User Runtime* (June 2010), is licensed to the person who
//! installs it ("you may not publish the software for others to copy",
//! its EULA §2), and the DirectX SDK's redistribution rights require
//! Microsoft's DirectSetup installer (`docs/RESEARCH.md`). So the window
//! has a Download button (and `--download-directx`): the launcher fetches
//! that package from Microsoft, pinned by SHA-256, takes the 32-bit DLLs
//! out of its cabinets and keeps them in its own folder; each launch puts
//! them next to the game in the private copy (through the component
//! ledger, so a file of the player's is never replaced) and, under Wine,
//! asks for them before Wine's own (`n,b`).

use std::fs;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};

use rea_launcher::download::{self, Fetch, FreeSpace, Job, Phase, Pin};
use rea_launcher::ledger::{Ledger, Outcome};

/// Microsoft's June 2010 DirectX End-User Runtime (re-signed by Microsoft
/// since; this is the file download.microsoft.com serves today). Fields:
/// URL, SHA-256, size, the unpacked DLLs' size.
pub const PIN: &str = "https://download.microsoft.com/download/8/4/A/84A35BF1-DAFE-4AE8-82AF-AD2AE20B6B14/directx_Jun2010_redist.exe \
053f76dcbb28802e23341b6a787e3b0791c0fa5c8d4d011b1044172dbf89c73b 100275120 80000000";

pub fn pin() -> Pin {
    Pin::parse(PIN).expect("a valid pin")
}

/// The ledger of what the launcher put into the private copy.
pub const LEDGER: &str = ".srr-components";

/// What the window shows for the row.
pub const LABEL: &str = "Microsoft DirectX 9 files";

/// The 32-bit cabinets wanted from the package: every D3DX9 and
/// D3DCompiler version (the game names one of them; which one is not
/// worth guessing, they are small).
pub fn wanted_cab(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with("_x86.cab") && (lower.contains("_d3dx9_") || lower.contains("_d3dcompiler_"))
}

/// The DLLs wanted from an inner cabinet.
fn wanted_dll(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".dll")
        && (lower.starts_with("d3dx9_") || lower.starts_with("d3dcompiler_"))
        && !lower.contains(['/', '\\'])
}

fn cab_names<R: Read + io::Seek>(cabinet: &cab::Cabinet<R>) -> Vec<String> {
    cabinet
        .folder_entries()
        .flat_map(|folder| folder.file_entries().map(|file| file.name().to_string()))
        .collect()
}

/// The outer cabinet inside the self-extracting package: the first
/// `MSCF` signature that parses as a cabinet.
fn outer_cabinet(package: &[u8]) -> Option<cab::Cabinet<Cursor<&[u8]>>> {
    package
        .windows(4)
        .enumerate()
        .filter(|(_, window)| *window == b"MSCF")
        .find_map(|(at, _)| cab::Cabinet::new(Cursor::new(&package[at..])).ok())
}

/// The wanted DLLs of a package: `(file name, bytes)`, sorted by name.
pub fn extract(package: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut outer = outer_cabinet(package).ok_or("no cabinet in the DirectX package")?;
    let mut dlls = Vec::new();
    for name in cab_names(&outer).into_iter().filter(|n| wanted_cab(n)) {
        let mut inner_bytes = Vec::new();
        outer
            .read_file(&name)
            .and_then(|mut file| file.read_to_end(&mut inner_bytes))
            .map_err(|error| format!("{name}: {error}"))?;
        let mut inner = cab::Cabinet::new(Cursor::new(&inner_bytes[..]))
            .map_err(|error| format!("{name}: {error}"))?;
        for dll in cab_names(&inner).into_iter().filter(|n| wanted_dll(n)) {
            let mut bytes = Vec::new();
            inner
                .read_file(&dll)
                .and_then(|mut file| file.read_to_end(&mut bytes))
                .map_err(|error| format!("{name}/{dll}: {error}"))?;
            dlls.push((dll, bytes));
        }
    }
    if dlls.is_empty() {
        return Err("the DirectX package holds none of the D3DX9 files".into());
    }
    dlls.sort_by_key(|(name, _)| name.to_ascii_lowercase());
    Ok(dlls)
}

/// The extractor `download::install_with` runs on the package: the wanted
/// DLLs go into `into`, reported as unpacked bytes through `job`.
pub fn extract_into(archive: &Path, into: &Path, job: &Job) -> Result<(), String> {
    let package = fs::read(archive).map_err(|error| format!("{}: {error}", archive.display()))?;
    let dlls = extract(&package)?;
    let total = dlls.iter().map(|(_, bytes)| bytes.len() as u64).sum();
    let mut done = 0;
    job.set_phase(Phase::Unpacking { done, total });
    for (name, bytes) in &dlls {
        if job.cancelled() {
            return Err("cancelled".into());
        }
        let path = into.join(name);
        fs::write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))?;
        done += bytes.len() as u64;
        job.set_phase(Phase::Unpacking { done, total });
    }
    Ok(())
}

/// Downloads the package (resumable, verified) and unpacks the DLLs into
/// `download::installed(pin, root)` (SDK `install_with`, which deletes the
/// package afterwards). Runs on a worker thread; the window polls `job`.
pub fn install(
    pin: &Pin,
    root: &Path,
    free: FreeSpace,
    fetch: &dyn Fetch,
    job: &Job,
) -> Result<PathBuf, String> {
    fs::create_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))?;
    match download::install_with(pin, root, free, fetch, job, &extract_into) {
        Phase::Done(dir) => Ok(dir),
        Phase::Failed(error) => Err(error),
        Phase::Cancelled => Err("cancelled".to_string()),
        other => Err(format!("the download stopped ({other:?})")),
    }
}

/// The DLL names in an unpacked folder, sorted.
pub fn dlls(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| wanted_dll(name))
        .collect();
    names.sort_by_key(|name| name.to_ascii_lowercase());
    names
}

/// Wine DLL names (lower case, no `.dll`) of DLL file names.
pub fn override_names(dlls: &[String]) -> Vec<String> {
    dlls.iter()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.strip_suffix(".dll").unwrap_or(&lower).to_string()
        })
        .collect()
}

/// What [`place`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Placed {
    /// In the private copy now, by file name.
    pub installed: Vec<String>,
    /// Already in the private copy with the same bytes and recorded in
    /// the ledger: not written again.
    pub unchanged: Vec<String>,
    /// Left alone because the private copy holds someone else's file.
    pub not_ours: Vec<String>,
    /// Taken out (the component was removed).
    pub removed: Vec<String>,
}

/// Puts the unpacked DLLs (`dir`, or none) next to the game in the private
/// copy through the component ledger, and takes out the ones it put there
/// earlier that are no longer wanted. A player's own file is never
/// replaced or deleted. A file the ledger already records with the same
/// bytes is left alone, and the ledger is saved once (SDK `batch`).
pub fn place(game: &Path, runtime: &Path, dir: Option<&Path>) -> io::Result<Placed> {
    let mut ledger = Ledger::load(game, runtime, LEDGER);
    let wanted = dir.map(dlls).unwrap_or_default();
    ledger.batch(|ledger| {
        let mut placed = Placed::default();
        for name in &wanted {
            let bytes = fs::read(dir.expect("listed from dir").join(name))?;
            let recorded = ledger.names().any(|owned| owned == name);
            if recorded && fs::read(runtime.join(name)).is_ok_and(|current| current == bytes) {
                placed.unchanged.push(name.clone());
                continue;
            }
            match ledger.install(name, &bytes, &[])? {
                Outcome::NotOurs => placed.not_ours.push(name.clone()),
                _ => placed.installed.push(name.clone()),
            }
        }
        let stale: Vec<String> = ledger
            .names()
            .filter(|name| !wanted.iter().any(|w| w.eq_ignore_ascii_case(name)))
            .map(str::to_string)
            .collect();
        for name in stale {
            if ledger.take_out(&name, &[])? == Outcome::Restored {
                placed.removed.push(name);
            }
        }
        Ok(placed)
    })
}

/// What a launch does with the DirectX 9 files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Launch {
    /// The DLLs Wine is asked to load before its own (`n,b`); empty when
    /// they could not be placed.
    pub dlls: Vec<String>,
    /// Lines for `launcher.log` (warnings start with `warning:`).
    pub log: Vec<String>,
}

/// [`place`] for a launch: a failure is a logged warning and the game
/// starts without the files (Wine's own DirectX 9 pieces, or whatever the
/// private copy holds), never a refusal to play.
pub fn for_launch(game: &Path, runtime: &Path, dir: Option<&Path>) -> Launch {
    let mut launch = Launch::default();
    match place(game, runtime, dir) {
        Ok(placed) => {
            launch.dlls = dir.map(dlls).unwrap_or_default();
            if !placed.installed.is_empty() {
                launch.log.push(format!(
                    "directx: {} D3DX9 file(s) put in the private copy",
                    placed.installed.len()
                ));
            }
            if !placed.unchanged.is_empty() {
                launch.log.push(format!(
                    "directx: {} D3DX9 file(s) already in the private copy",
                    placed.unchanged.len()
                ));
            }
            for name in &placed.not_ours {
                launch.log.push(format!(
                    "directx: {name} in the private copy is not ours; kept"
                ));
            }
            if !placed.removed.is_empty() {
                launch.log.push(format!(
                    "directx: {} file(s) taken out of the private copy",
                    placed.removed.len()
                ));
            }
        }
        Err(error) => launch.log.push(format!(
            "warning: cannot place the DirectX 9 files ({error}); the game starts without them"
        )),
    }
    launch
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("srr-directx-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// A cabinet holding `files` (uncompressed).
    fn cabinet(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = cab::CabinetBuilder::new();
        let folder = builder.add_folder(cab::CompressionType::None);
        for (name, _) in files {
            folder.add_file(*name);
        }
        let mut writer = builder.build(Cursor::new(Vec::new())).unwrap();
        let mut index = 0;
        while let Some(mut file) = writer.next_file().unwrap() {
            io::Write::write_all(&mut file, files[index].1).unwrap();
            index += 1;
        }
        writer.finish().unwrap().into_inner()
    }

    /// A stand-in for the self-extracting package: some bytes, then the
    /// outer cabinet holding inner cabinets (as Microsoft's does).
    fn package() -> Vec<u8> {
        let d3dx35 = cabinet(&[("d3dx9_35.dll", b"d3dx 35"), ("d3dx9_35_x86.inf", b"inf")]);
        let compiler = cabinet(&[("D3DCompiler_43.dll", b"compiler 43")]);
        let x64 = cabinet(&[("d3dx9_35.dll", b"64-bit")]);
        let xact = cabinet(&[("xactengine2_9.dll", b"xact")]);
        let outer = cabinet(&[
            ("AUG2007_d3dx9_35_x86.cab", &d3dx35),
            ("AUG2007_d3dx9_35_x64.cab", &x64),
            ("Jun2010_D3DCompiler_43_x86.cab", &compiler),
            ("Jun2010_XACT_x86.cab", &xact),
            ("DXSETUP.exe", b"MZ setup"),
        ]);
        let mut package = b"MZ\x90\x00 self-extractor MSCF not a cabinet ".to_vec();
        package.extend(outer);
        package
    }

    #[test]
    fn the_pin_is_microsofts_package() {
        let pin = pin();
        assert!(pin.url.starts_with("https://download.microsoft.com/"));
        assert_eq!(pin.file_name(), "directx_Jun2010_redist.exe");
        assert_eq!(pin.size, 100_275_120);
    }

    #[test]
    fn only_the_32_bit_d3dx9_and_compiler_files_come_out() {
        let dlls = extract(&package()).unwrap();
        let names: Vec<&str> = dlls.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["D3DCompiler_43.dll", "d3dx9_35.dll"]);
        assert_eq!(dlls[1].1, b"d3dx 35");
        assert!(extract(b"no cabinet here").is_err());
        assert!(wanted_cab("Apr2005_d3dx9_25_x86.cab"));
        assert!(!wanted_cab("Apr2005_d3dx9_25_x64.cab"));
        assert!(!wanted_cab("APR2007_d3dx10_33_x86.cab"));
    }

    #[test]
    fn the_folder_is_named_after_the_package() {
        let root = Path::new("root");
        assert_eq!(pin().dir(root), root.join("directx_Jun2010_redist"));
    }

    /// Serves `body` (honouring a resume offset).
    struct Serve(Vec<u8>);

    impl Fetch for Serve {
        fn fetch(
            &self,
            _url: &str,
            from: u64,
            start: &mut dyn FnMut(bool),
            chunk: &mut dyn FnMut(&[u8]) -> bool,
        ) -> Result<(), String> {
            start(true);
            chunk(&self.0[from as usize..]);
            Ok(())
        }
    }

    #[test]
    fn install_downloads_verifies_unpacks_and_cleans_up() {
        let root = temp("install");
        let body = package();
        let mut pin = pin();
        pin.sha256 = rea_launcher::identify::sha256(&body);
        pin.size = body.len() as u64;
        pin.unpacked_size = 100;
        let job = Job::default();
        let dir = install(&pin, &root, &|_| None, &Serve(body), &job).unwrap();
        assert_eq!(job.phase(), Phase::Done(dir.clone()));
        assert_eq!(download::installed(&pin, &root), Some(dir.clone()));
        assert_eq!(dlls(&dir), ["D3DCompiler_43.dll", "d3dx9_35.dll"]);
        assert_eq!(fs::read(dir.join("d3dx9_35.dll")).unwrap(), b"d3dx 35");
        assert_eq!(override_names(&dlls(&dir)), ["d3dcompiler_43", "d3dx9_35"]);
        assert!(!root.join(pin.file_name()).exists(), "package deleted");
        // Remove takes the folder and any leftovers.
        download::remove(&pin, &root).unwrap();
        assert_eq!(download::installed(&pin, &root), None);
        // A wrong download is refused and nothing is unpacked.
        let root2 = temp("install-bad");
        let job = Job::default();
        let error = install(&pin, &root2, &|_| None, &Serve(b"tampered".to_vec()), &job);
        assert!(error.is_err());
        assert!(matches!(job.phase(), Phase::Failed(_)));
        assert_eq!(download::installed(&pin, &root2), None);
        // A package without the DLLs fails the unpack; nothing is marked.
        let empty = cabinet(&[("DXSETUP.exe", b"MZ")]);
        pin.sha256 = rea_launcher::identify::sha256(&empty);
        pin.size = empty.len() as u64;
        let job = Job::default();
        assert!(install(&pin, &root2, &|_| None, &Serve(empty), &job).is_err());
        assert_eq!(download::installed(&pin, &root2), None);
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(root2).unwrap();
    }

    #[test]
    fn place_installs_through_the_ledger_and_takes_out_on_removal() {
        let base = temp("place");
        let (game, runtime, dir) = (base.join("game"), base.join("runtime"), base.join("dlls"));
        for folder in [&game, &runtime, &dir] {
            fs::create_dir_all(folder).unwrap();
        }
        fs::write(dir.join("d3dx9_35.dll"), b"ms").unwrap();
        fs::write(dir.join("d3dx9_36.dll"), b"ms 36").unwrap();
        // The player's own d3dx9_36.dll in the private copy stays.
        fs::write(runtime.join("d3dx9_36.dll"), b"player's").unwrap();
        let placed = place(&game, &runtime, Some(&dir)).unwrap();
        assert_eq!(placed.installed, ["d3dx9_35.dll"]);
        assert_eq!(placed.not_ours, ["d3dx9_36.dll"]);
        assert_eq!(fs::read(runtime.join("d3dx9_35.dll")).unwrap(), b"ms");
        assert_eq!(fs::read(runtime.join("d3dx9_36.dll")).unwrap(), b"player's");
        // Removed: ours go, the player's stays.
        let placed = place(&game, &runtime, None).unwrap();
        assert_eq!(placed.removed, ["d3dx9_35.dll"]);
        assert!(!runtime.join("d3dx9_35.dll").exists());
        assert!(runtime.join("d3dx9_36.dll").exists());
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn place_leaves_recorded_identical_files_alone() {
        let base = temp("place-same");
        let (game, runtime, dir) = (base.join("game"), base.join("runtime"), base.join("dlls"));
        for folder in [&game, &runtime, &dir] {
            fs::create_dir_all(folder).unwrap();
        }
        fs::write(dir.join("d3dx9_35.dll"), b"ms").unwrap();
        assert_eq!(
            place(&game, &runtime, Some(&dir)).unwrap().installed,
            ["d3dx9_35.dll"]
        );
        // Every later launch: nothing written.
        let placed = place(&game, &runtime, Some(&dir)).unwrap();
        assert!(placed.installed.is_empty(), "{placed:?}");
        assert_eq!(placed.unchanged, ["d3dx9_35.dll"]);
        // A changed file in the private copy is ours by the ledger only if
        // its bytes match; a new download replaces our old one.
        fs::write(dir.join("d3dx9_35.dll"), b"ms 2").unwrap();
        let placed = place(&game, &runtime, Some(&dir)).unwrap();
        assert_eq!(placed.installed, ["d3dx9_35.dll"]);
        assert_eq!(fs::read(runtime.join("d3dx9_35.dll")).unwrap(), b"ms 2");
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn a_launch_that_cannot_place_the_files_warns_and_goes_on() {
        let base = temp("place-fails");
        let (game, runtime, dir) = (base.join("game"), base.join("runtime"), base.join("dlls"));
        for folder in [&game, &runtime, &dir] {
            fs::create_dir_all(folder).unwrap();
        }
        // An unreadable download: a folder where a DLL should be.
        fs::create_dir_all(dir.join("d3dx9_35.dll").join("blocked")).unwrap();
        let launch = for_launch(&game, &runtime, Some(&dir));
        assert!(
            launch.dlls.is_empty(),
            "no n,b override for files not placed"
        );
        assert_eq!(launch.log.len(), 1);
        assert!(launch.log[0].starts_with("warning: cannot place the DirectX 9 files"));
        assert!(launch.log[0].ends_with("the game starts without them"));
        // Once it reads, the overrides name every DLL.
        fs::remove_dir_all(dir.join("d3dx9_35.dll")).unwrap();
        fs::write(dir.join("d3dx9_35.dll"), b"ms").unwrap();
        let launch = for_launch(&game, &runtime, Some(&dir));
        assert_eq!(launch.dlls, ["d3dx9_35.dll"]);
        assert_eq!(
            launch.log,
            ["directx: 1 D3DX9 file(s) put in the private copy"]
        );
        fs::remove_dir_all(base).unwrap();
    }
}
