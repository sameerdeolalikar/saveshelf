use saveshelf::{list, restore, snapshot, verify, Manifest};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "saveshelf-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::create_dir(path.join("source")).unwrap();
        fs::write(
            path.join("source/slot1.sav"),
            b"fictional game state\x00\x01",
        )
        .unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn snap(&self) -> PathBuf {
        snapshot(&self.path("source"), &self.path("vault")).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn snapshot_verify_restore_roundtrip_including_empty_directories() {
    let f = Fixture::new();
    fs::create_dir_all(f.path("source/nested/empty")).unwrap();
    fs::write(f.path("source/nested/slot2.sav"), b"other state").unwrap();
    let snap = f.snap();
    assert_eq!(verify(&snap).unwrap().files.len(), 2);
    let restored = restore(&snap, &f.path("restored")).unwrap();
    assert_eq!(
        fs::read(restored.join("slot1.sav")).unwrap(),
        fs::read(f.path("source/slot1.sav")).unwrap()
    );
    assert!(restored.join("nested/empty").is_dir());
    assert!(f.path("restored/RESTORE_COMPLETE").is_file());
}

#[test]
fn corrupt_snapshot_is_rejected_before_destination_creation() {
    let f = Fixture::new();
    let snap = f.snap();
    fs::write(snap.join("files/slot1.sav"), b"corrupted").unwrap();
    assert!(verify(&snap).is_err());
    assert!(restore(&snap, &f.path("restored")).is_err());
    assert!(!f.path("restored").exists());
}

#[test]
fn missing_or_extra_file_invalidates_snapshot() {
    let f = Fixture::new();
    let snap = f.snap();
    fs::write(snap.join("files/untracked.sav"), b"extra").unwrap();
    assert!(verify(&snap).is_err());
    fs::remove_file(snap.join("files/untracked.sav")).unwrap();
    fs::remove_file(snap.join("files/slot1.sav")).unwrap();
    assert!(verify(&snap).is_err());
}

#[test]
fn existing_restore_target_and_source_are_never_overwritten() {
    let f = Fixture::new();
    let snap = f.snap();
    assert!(restore(&snap, &f.path("source")).is_err());
    assert_eq!(
        fs::read(f.path("source/slot1.sav")).unwrap(),
        b"fictional game state\x00\x01"
    );
}

#[test]
fn nested_vault_is_rejected_without_modifying_source() {
    let f = Fixture::new();
    assert!(snapshot(&f.path("source"), &f.path("source/vault")).is_err());
    assert!(!f.path("source/vault").exists());
}

#[test]
fn cannot_restore_into_snapshot() {
    let f = Fixture::new();
    let snap = f.snap();
    assert!(restore(&snap, &snap.join("restored")).is_err());
    assert!(verify(&snap).is_ok());
}

#[test]
fn incomplete_snapshots_are_not_listed_or_restored() {
    let f = Fixture::new();
    let snap = f.snap();
    fs::remove_file(snap.join("COMPLETE")).unwrap();
    assert!(list(&f.path("vault")).unwrap().is_empty());
    assert!(restore(&snap, &f.path("restored")).is_err());
}

fn edit_manifest(snapshot: &Path, edit: impl FnOnce(&mut Manifest)) {
    let path = snapshot.join("manifest.json");
    let mut manifest: Manifest = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    edit(&mut manifest);
    fs::write(path, serde_json::to_vec(&manifest).unwrap()).unwrap();
}

#[test]
fn manifest_paths_cannot_escape_restore_root() {
    for path in [
        "../outside",
        "/tmp/outside",
        "a/../../b",
        "a\\b",
        "C:/outside",
        "./slot1.sav",
    ] {
        let f = Fixture::new();
        let snap = f.snap();
        edit_manifest(&snap, |m| m.files[0].path = path.into());
        assert!(verify(&snap).is_err(), "accepted {path}");
        assert!(restore(&snap, &f.path("restored")).is_err());
        assert!(!f.path("restored").exists());
    }
}

#[test]
fn duplicate_manifest_paths_and_unknown_versions_fail() {
    let f = Fixture::new();
    let snap = f.snap();
    edit_manifest(&snap, |m| m.files.push(m.files[0].clone()));
    assert!(verify(&snap).is_err());
    let other = f.snap();
    edit_manifest(&other, |m| m.version = 99);
    assert!(verify(&other).is_err());
}

#[test]
fn multiple_snapshots_remain_independent() {
    let f = Fixture::new();
    let first = f.snap();
    fs::write(f.path("source/slot1.sav"), b"next game state").unwrap();
    let second = f.snap();
    assert_ne!(first, second);
    assert_eq!(list(&f.path("vault")).unwrap().len(), 2);
    assert_ne!(
        verify(&first).unwrap().files[0].sha256,
        verify(&second).unwrap().files[0].sha256
    );
}

#[cfg(unix)]
#[test]
fn source_and_snapshot_symlinks_are_refused() {
    let f = Fixture::new();
    std::os::unix::fs::symlink(f.path("source/slot1.sav"), f.path("source/link.sav")).unwrap();
    assert!(snapshot(&f.path("source"), &f.path("vault")).is_err());
    fs::remove_file(f.path("source/link.sav")).unwrap();
    let snap = f.snap();
    fs::remove_file(snap.join("files/slot1.sav")).unwrap();
    std::os::unix::fs::symlink(f.path("source/slot1.sav"), snap.join("files/slot1.sav")).unwrap();
    assert!(verify(&snap).is_err());
}

#[test]
fn cli_failure_returns_nonzero_without_writing_output() {
    let f = Fixture::new();
    let snap = f.snap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_saveshelf"))
        .arg("restore")
        .arg(&snap)
        .arg("--output")
        .arg(f.path("source"))
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(
        fs::read(f.path("source/slot1.sav")).unwrap(),
        b"fictional game state\x00\x01"
    );
}
