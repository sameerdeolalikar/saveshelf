# SaveShelf roadmap

The initial CLI provides local checksummed snapshots and new-directory restores.
The estimates below are future work, not completed paid sessions or claimed adoption.

| Work | Acceptance criteria | Estimate |
| --- | --- | --- |
| Restore previews and comparisons | Report added/changed/missing files against a chosen folder without modifying it; explicit conflict decisions | 7–10 hours |
| Retention policies | Dry-run plans, protected snapshots, disk budget rules, and interruption-safe deletion with tests | 8–12 hours |
| Recovery from interruption | Detect and explain partial operations; safe cleanup/recovery commands; fault-injection tests | 8–12 hours |
| Storage efficiency | Deduplicate unchanged file content, preserve verification, and account for disk usage correctly | 7–10 hours |
| Game profiles and exclusions | User-defined paths and exclusion patterns; no scraping or private account data; synthetic compatibility fixtures | 5–8 hours |
| Cross-platform reliability | Windows/macOS path rules, metadata policy, long-path behavior, and documented restore exercises | 7–10 hours |

**Estimated remaining work: 42–62 hours.** Keep the project focused on a player's
own local single-player saves. Multiplayer account data, anti-cheat bypasses, game
exploitation, and cloud credential management are outside scope.
