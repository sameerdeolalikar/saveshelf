# SaveShelf

**Keep a known copy. Verify it. Restore somewhere new.**

SaveShelf is a local snapshot manager with a browser interface, a Rust backend,
and a companion CLI for single-player game save folders.
It records file sizes and SHA-256 checksums, verifies stored bytes, and restores
into a new directory without overwriting live game saves. It is intended for
players keeping their own local checkpoints before trying a different game path.

This is a new project with an initial snapshot/verify/restore implementation.
It has no accounts, telemetry, cloud sync, game-process hooks, or external network calls at
runtime. The optional UI uses a loopback HTTP server. Dependencies are downloaded when building through Cargo.

## Open the snapshot manager

```bash
cargo run -- serve /path/to/a/new/saveshelf-workspace
```

Open **http://127.0.0.1:7413**. The server creates `sources/`, `vault/`, and
`restores/` under that workspace. Use “Create fictional demo save” to try the full
capture → verify → restore flow. For your own local files, place a **copy** of a save
folder under `sources/`; source folder names must use letters, numbers, hyphens,
or underscores. Close the game before capture.

The UI lists completed checkpoints, verifies selected snapshots, previews file names,
sizes, and checksum prefixes, and restores into a new named folder under `restores/`.
It shows activity, validation errors, and the completed recovery location. The first
100 files are displayed; verification still checks every file. Only one operation
runs at a time. It does not yet provide cancellation, percentage progress, comparison
against live saves, or automatic retention. Leave the server running until an operation
finishes. Do not concurrently edit workspace directories.

This is a local browser application, **not a packaged native desktop application**.
Desktop packaging and native folder selection remain on the roadmap. The server binds
only to loopback and rejects cross-origin mutation requests; do not expose it through
a public proxy. Original CLI commands remain available for standalone workflows.

## Try it with fictional data first

Install current stable Rust and Cargo:

```bash
git clone https://github.com/sameerdeolalikar/saveshelf.git
cd saveshelf
mkdir demo-save
printf 'level=3\ncoins=42\n' > demo-save/slot1.sav
cargo run -- snapshot demo-save --vault demo-vault
cargo run -- list --vault demo-vault
```

The snapshot command prints a path such as
`demo-vault/snapshot-TIMESTAMP-PID`. Use the actual printed path:

```bash
cargo run -- verify SNAPSHOT_PATH
cargo run -- restore SNAPSHOT_PATH --output recovered
```

Recovered content is in **`recovered/files`**, with a `RESTORE_COMPLETE` marker in
the container. `recovered` must not already exist; even an existing empty directory
is refused. The parent directory must exist. Inspect and test recovered data before
manually replacing anything in a real game's save location.

`cargo install --path .` installs the `saveshelf` command from this checkout.
There is no crates.io release yet. Exit code 2 means a validation or I/O failure.

## Snapshot layout

```text
snapshot-TIMESTAMP-PID/
  manifest.json    # version, timestamp, relative paths, sizes, SHA-256 digests
  files/           # copied file bytes and empty directories
  COMPLETE         # written only after the snapshot finishes
```

Source and vault directories cannot contain each other. Symlinks, special files,
non-UTF-8 names, and traversal-style manifest paths are rejected. The initial limits
are 10,000 file/directory entries and 2 GiB of total file data. Paths must not contain
backslashes or colons. Empty directories are retained; permissions, timestamps,
extended attributes, and sparse-file structure are not preserved.

## Integrity and recovery boundaries

- Close the game before snapshotting. Metadata checks detect common concurrent
  changes, but copying several files is not a transactional filesystem snapshot.
- Verification checks the inventory and bytes; missing, additional, altered, or
  symlinked files are rejected. SHA-256 is a corruption check, not authentication
  against a person who can also edit the manifest.
- Restore verifies first and rechecks copied bytes. It reserves a brand-new output
  container atomically, so an existing save directory cannot be selected for overwrite.
- Normal errors clean up the new partial directory. A process crash can leave an
  incomplete snapshot or restore container. Do not treat either as usable without
  its completion marker. `list` omits snapshots without a completion marker, but
  does not verify all checksums; use `verify` explicitly.
- File data is synced before completion markers, but full power-loss durability
  across filesystems has not been established. Keep independent backups of valuable
  saves. Do not concurrently edit the vault or restore destination while a command runs.

Automatic retention/deletion, restore-in-place, encryption, compression, and cloud
storage are not implemented. Real game compatibility is not yet validated; all tests
and examples use synthetic save bytes. Do not commit real save files or player data.

## Development

```bash
cargo test --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
```

There are 12 core integration tests and a workspace-name boundary test on Linux, including snapshot round trips,
empty directories, corruption, inventory changes, output protection, incomplete
snapshots, traversal rejection, versioning, and symlinks. The symlink test is Unix-only.
Initial checks use stable Rust on Linux; cross-platform validation remains planned.

See [ROADMAP.md](ROADMAP.md), [CONTRIBUTING.md](CONTRIBUTING.md), and
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). GitHub Actions is not configured.
Licensed under [MIT](LICENSE).

## Browser verification

Start `cargo run -- serve /tmp/saveshelf-ui-test-workspace` against a **disposable**
workspace. The browser test adds fictional data, snapshots, and recovery folders there.
With Node.js 20+ available, run in another terminal:

```bash
npm ci
npx playwright install chromium
npm run test:ui
```

The test checks capture, verification, restored bytes, refusal to overwrite an existing
destination, traversal rejection, request-origin checks, and a 390px layout. Current
browser validation uses Chromium on Linux. See [UI.md](UI.md) for architecture and
future development scenarios. Never point automated UI tests at a real-save workspace.
