//! Local save snapshots. No source deletion, automatic retention, or live-save overwrite.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 10000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub created_unix_seconds: u64,
    pub directories: Vec<String>,
    pub files: Vec<Entry>,
}

#[derive(Debug, PartialEq, Eq)]
struct SourceFile {
    path: String,
    bytes: u64,
    modified: SystemTime,
}
#[derive(Debug, PartialEq, Eq)]
struct Inventory {
    directories: Vec<String>,
    files: Vec<SourceFile>,
}

fn relative(value: &str) -> Result<PathBuf> {
    if value.is_empty() || value.contains(['\\', ':', '\0']) {
        return Err("invalid relative path".into());
    }
    let path = Path::new(value);
    if value
        .split('/')
        .any(|p| p.is_empty() || p == "." || p == "..")
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err("snapshot paths must be plain relative paths without traversal".into());
    }
    Ok(path.to_path_buf())
}

fn directory(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err("expected a real directory, not a symlink".into());
    }
    Ok(())
}

fn regular(path: &Path) -> Result<fs::Metadata> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("expected a regular file, not a symlink".into());
    }
    Ok(meta)
}

fn inventory(root: &Path) -> Result<Inventory> {
    directory(root)?;
    let mut result = Inventory {
        directories: Vec::new(),
        files: Vec::new(),
    };
    let mut pending = vec![root.to_path_buf()];
    let mut bytes = 0_u64;
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            let meta = fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                return Err(format!("symlink refused: {}", path.display()).into());
            }
            let name = path
                .strip_prefix(root)?
                .to_str()
                .ok_or("filenames must be valid UTF-8")?
                .replace(std::path::MAIN_SEPARATOR, "/");
            relative(&name)?;
            if meta.is_dir() {
                result.directories.push(name);
                pending.push(path);
            } else if meta.is_file() {
                bytes = bytes.checked_add(meta.len()).ok_or("size overflow")?;
                if bytes > MAX_BYTES {
                    return Err("snapshot exceeds 2 GiB limit".into());
                }
                result.files.push(SourceFile {
                    path: name,
                    bytes: meta.len(),
                    modified: meta.modified()?,
                });
            } else {
                return Err("special files are not supported".into());
            }
            if result.files.len() + result.directories.len() > MAX_ENTRIES {
                return Err("snapshot exceeds 10,000 entries".into());
            }
        }
    }
    result.directories.sort();
    result.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

fn digest(path: &Path, mut output: Option<&mut File>) -> Result<(u64, String)> {
    regular(path)?;
    let mut input = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 65536];
    let mut bytes = 0_u64;
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        if bytes > MAX_BYTES {
            return Err("file exceeds 2 GiB limit".into());
        }
        hasher.update(&buffer[..read]);
        if let Some(target) = output.as_deref_mut() {
            target.write_all(&buffer[..read])?;
        }
    }
    Ok((bytes, format!("{:x}", hasher.finalize())))
}

struct NewDirectory {
    path: PathBuf,
    keep: bool,
}
impl NewDirectory {
    fn create(path: &Path) -> Result<Self> {
        fs::create_dir(path)?;
        Ok(Self {
            path: path.to_path_buf(),
            keep: false,
        })
    }
}
impl Drop for NewDirectory {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn resolved_target(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return Ok(fs::canonicalize(path)?);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(fs::canonicalize(parent)?.join(path.file_name().ok_or("invalid destination name")?))
}

pub fn snapshot(source: &Path, vault: &Path) -> Result<PathBuf> {
    directory(source)?;
    let source = fs::canonicalize(source)?;
    let resolved_vault = resolved_target(vault)?;
    if resolved_vault.starts_with(&source) || source.starts_with(&resolved_vault) {
        return Err("source and vault must not contain each other".into());
    }
    fs::create_dir_all(vault)?;
    directory(vault)?;
    let vault = fs::canonicalize(vault)?;
    let before = inventory(&source)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    let path = vault.join(format!(
        "snapshot-{}-{}",
        now.as_nanos(),
        std::process::id()
    ));
    let mut guard = NewDirectory::create(&path)?;
    let files = path.join("files");
    fs::create_dir(&files)?;
    for dir in &before.directories {
        fs::create_dir_all(files.join(relative(dir)?))?;
    }
    let mut manifest = Manifest {
        version: 1,
        created_unix_seconds: now.as_secs(),
        directories: before.directories.clone(),
        files: Vec::new(),
    };
    for item in &before.files {
        let rel = relative(&item.path)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(files.join(&rel))?;
        let (bytes, sha256) = digest(&source.join(&rel), Some(&mut output))?;
        output.sync_all()?;
        if bytes != item.bytes {
            return Err("source changed during snapshot; stop the game and try again".into());
        }
        manifest.files.push(Entry {
            path: item.path.clone(),
            bytes,
            sha256,
        });
    }
    if inventory(&source)? != before {
        return Err("source changed during snapshot; stop the game and try again".into());
    }
    write_new(
        &path.join("manifest.json"),
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    write_new(&path.join("COMPLETE"), b"saveshelf v1\n")?;
    guard.keep = true;
    Ok(path)
}

pub fn verify(snapshot: &Path) -> Result<Manifest> {
    directory(snapshot)?;
    regular(&snapshot.join("COMPLETE"))?;
    let manifest_path = snapshot.join("manifest.json");
    if regular(&manifest_path)?.len() > 5_000_000 {
        return Err("manifest exceeds 5 MB".into());
    }
    let manifest: Manifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
    if manifest.version != 1 {
        return Err("unsupported manifest version".into());
    }
    if manifest.files.len() + manifest.directories.len() > MAX_ENTRIES {
        return Err("too many manifest entries".into());
    }
    let mut names = HashSet::new();
    for dir in &manifest.directories {
        relative(dir)?;
        if !names.insert(dir) {
            return Err("duplicate path in manifest".into());
        }
    }
    let mut total = 0_u64;
    for item in &manifest.files {
        relative(&item.path)?;
        if !names.insert(&item.path) {
            return Err("duplicate path in manifest".into());
        }
        total = total.checked_add(item.bytes).ok_or("size overflow")?;
        if total > MAX_BYTES
            || item.sha256.len() != 64
            || !item.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("invalid size or checksum in manifest".into());
        }
    }
    let root = snapshot.join("files");
    let actual = inventory(&root)?;
    let mut expected_dirs = manifest.directories.clone();
    expected_dirs.sort();
    let mut expected_files: Vec<&str> = manifest.files.iter().map(|i| i.path.as_str()).collect();
    expected_files.sort();
    if actual.directories != expected_dirs
        || actual
            .files
            .iter()
            .map(|i| i.path.as_str())
            .collect::<Vec<_>>()
            != expected_files
    {
        return Err("snapshot inventory differs from manifest".into());
    }
    for item in &manifest.files {
        let (bytes, hash) = digest(&root.join(relative(&item.path)?), None)?;
        if bytes != item.bytes || hash != item.sha256 {
            return Err(format!("checksum mismatch: {}", item.path).into());
        }
    }
    Ok(manifest)
}

pub fn restore(snapshot: &Path, destination: &Path) -> Result<PathBuf> {
    let manifest = verify(snapshot)?;
    if resolved_target(destination)?.starts_with(fs::canonicalize(snapshot)?) {
        return Err("restore destination must be outside the snapshot".into());
    }
    // Reserve a brand-new container atomically. Existing directories are never replaced.
    let mut guard = NewDirectory::create(destination)?;
    let stage = destination.join(".incomplete");
    fs::create_dir(&stage)?;
    for dir in &manifest.directories {
        fs::create_dir_all(stage.join(relative(dir)?))?;
    }
    for item in &manifest.files {
        let rel = relative(&item.path)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(stage.join(&rel))?;
        let (bytes, hash) = digest(&snapshot.join("files").join(&rel), Some(&mut output))?;
        output.sync_all()?;
        if bytes != item.bytes || hash != item.sha256 {
            return Err("snapshot changed while restoring".into());
        }
    }
    let final_path = destination.join("files");
    fs::rename(stage, &final_path)?;
    write_new(&destination.join("RESTORE_COMPLETE"), b"saveshelf v1\n")?;
    guard.keep = true;
    Ok(final_path)
}

pub fn list(vault: &Path) -> Result<Vec<PathBuf>> {
    directory(vault)?;
    let mut paths = Vec::new();
    for entry in fs::read_dir(vault)? {
        let entry = entry?;
        if entry.file_type()?.is_dir()
            && entry.file_name().to_string_lossy().starts_with("snapshot-")
            && regular(&entry.path().join("COMPLETE")).is_ok()
        {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}
