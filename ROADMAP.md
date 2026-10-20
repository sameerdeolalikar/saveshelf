# SaveShelf roadmap

Implemented: local Rust-backed browser UI, source selection, checkpoint listing,
verification with file preview, guided protected restore, and the original CLI.

Remaining work estimates describe development still to do, not completed or paid hours.

| Work | Acceptance criteria | Estimate |
| --- | --- | --- |
| Desktop packaging | Native folder selection, installation lifecycle, clear workspace boundaries, platform checks | 8–12 hours |
| Jobs and interruption recovery | Background queue, truthful progress, cancellation, incomplete-operation recovery, fault injection | 10–15 hours |
| Restore comparison | Compare snapshot against a chosen folder, explain changed/missing files, keyboard-accessible review | 7–10 hours |
| Retention interface | Dry-run deletion plans, protected snapshots, disk budgets, interruption-safe deletion | 8–12 hours |
| Efficient storage | Content deduplication, reference-safe cleanup, integrity checks, disk-usage display | 8–12 hours |
| Profiles and accessibility | Exclusions, source labels, screen-reader support, Windows/macOS path and restore testing | 7–10 hours |

**Estimated remaining work: 48–71 hours.** Use synthetic save fixtures during development.
No real-game compatibility claims yet. Multiplayer accounts, anti-cheat bypasses,
exploitation, cloud credentials, and other people's personal data remain outside scope.
