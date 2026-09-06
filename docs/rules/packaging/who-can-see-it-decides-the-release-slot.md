One question classifies a change a dependency forced: **who can see it?** Not how big the diff was,
not how many crates moved.

| The update changes… | Ships as |
|---|---|
| Only Novis's internals — call sites, an adapter, a data structure, a Rust API | **patch** |
| Novis's behaviour back into agreement with its own spec (a bug fix) | **patch**, and the release note names it |
| Behaviour a program could plausibly have relied on, even though it was wrong | **minor**, with the note |
| Anything additive on the versioned surface — a new `Core` member, config key, CLI flag, diagnostic | **minor** |
| MSRV, a build-time requirement, a CI or developer tool | **patch** — no user of a released binary can observe it |
| A removal, rename or incompatible change anywhere on the versioned surface | **major**, and only after the ladder |
| Dropping a supported runtime platform or database server version | **major** |
| A security fix that necessarily changes observable behaviour | **whatever release ships soonest**, including a patch; the note says plainly what changed |

**The asymmetry is the point.** An internal break is cheap and needs no approval — that is what an
implementation *is*. Reaching for a major because a dependency's Rust API changed is a category
error. The surface the table refers to is `rule:packaging/the-versioned-surface-is-enumerated`; the
ladder its last rows send a change down is `rule:packaging/a-dependency-break-is-absorbed-never-forwarded`.
Nothing checks the classification; it is addressed to whoever runs the sweep, and the commit message
is where it becomes visible (`rule:packaging/one-bump-one-commit`).
