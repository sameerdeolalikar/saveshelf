# Contributing

Use current stable Rust. Run `cargo fmt --check`, `cargo test --locked`, and
`cargo clippy --all-targets --locked -- -D warnings` before proposing changes.

Protect the source and existing destination first. New filesystem behavior needs
tests for interrupted/failed operations, corrupt manifests, path boundaries, and
existing outputs. Explain when an operation becomes complete and what remains
after a crash. Never describe checksums as proof of trusted authorship.

Use focused commits with imperative subjects. Pull requests should explain the
problem, resulting behavior, data-loss implications, and validation. Fixtures must
be synthetic bytes, not real player saves, account information, or copied game assets.
Preserve third-party license notices when dependencies change.
