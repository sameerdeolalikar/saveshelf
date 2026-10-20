# Snapshot manager

The Rust binary's `serve WORKSPACE` command hosts embedded HTML/CSS/JavaScript with
`tiny_http`. API handlers call the existing snapshot, verify, and restore functions.
The graphical flow selects a source, captures a checkpoint, verifies it, displays its
manifest, and restores into a newly named directory.

Filesystem access is scoped to `sources/`, `vault/`, and `restores/` inside the chosen
workspace. Names cannot contain path separators; symlinked workspace directories are
rejected. The server is loopback-only and checks Host/Origin plus a custom request
header. It is a single-user local tool, not a hosted service. Do not mutate the vault
or workspace boundaries concurrently with operations.

The UI indicates activity but does not claim percentage progress. Operations are
currently serialized and cannot be cancelled from the UI. This browser-based version
is not a Tauri/native desktop package.

## Next development scenarios

- Add queued jobs with accurate progress and cancellation. Verify readable state changes,
  preserved snapshots, and recovery after interruption.
- Add restore comparisons. Test understandable file differences, keyboard navigation,
  destination validation, and preservation of existing data.
- Add retention previews and protected checkpoints. Test explicit deletion plans,
  recoverable failures, and consistent disk-usage reporting.

These ordinary application tasks expose differences in assistant-produced UI clarity,
Rust integration, filesystem reasoning, and recovery design. SaveShelf itself is not
an AI evaluation or benchmark tool.

## Verification limits

Chromium tests exercise the full flow with synthetic files and a 390px viewport.
Core tests cover corruption and filesystem boundaries. Native desktop packaging,
Windows/macOS reliability, full accessibility, and real-game compatibility remain
unverified and are on the roadmap.
