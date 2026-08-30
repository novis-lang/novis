# Handoff

## State

**Goal 3, Stage 2 has started landing.** `crates/nvs-config` now exists with two modules and seven
green tests: `directive.rs` is the registry — a directive is three fields, `key`, `class` (ADR 0005)
and `apply` (ADR 0078 § 2's `Reload`/`Boot`, orthogonal to the class) — and `file.rs` reads one file
through `toml`+`serde`, refusing a duplicate key as `E0604` with the offending line as a span.

**A registry row governs the keys beneath it**, longest-prefix on dot boundaries, so `limits` is one
row and `limits.hard` overrides it. That is how the ADRs state the classes ("every `[[schedule]]`
key", "`[server]` — Boot"), and `directive.rs`'s module doc is its only home. The registry is
therefore **not** the list of legal keys: refusing an unknown one is `serde`'s
`deny_unknown_fields` over a typed tree that does not exist yet, which is the next slice.

Nothing is blocked. Goals 1 and 2 and M4 remain the floor; the acceptance set claim for Stage 6 is
unchanged.

## Next group

**The typed block tree, then the tree of files** — Stage 2's remainder. One file set:
`crates/nvs-config/src/{file.rs,directive.rs}` plus new modules beside them, and their `tests/`.

- [ ] **The typed block tree, and an unknown key refused naming its block.** ADR 0064 §§ 2a, 3 —
      § 2a's table (`docs/adr/0064-configuration-file-format.md:129`) names the owning ADR of every
      block, and each block's fields have to come from that ADR rather than be invented, which is
      the whole cost of this slice. `crates/nvs-config/src/file.rs:38`'s `parse` is already generic
      over the tree type with `toml::Table` as the stand-in, so nothing above it changes. Two
      blocks are already transcribed: `[server]` in ADR 0097 § 5 and `[limits]`/`[limits.hard]` in
      ADR 0005. Needs `derive` on the workspace `serde` (`Cargo.toml:199`, currently featureless).
- [ ] **The tree resolves.** ADR 0103 §§ 1–5: a root named by repeatable `--config`, else
      `./nvs.toml`, else the shipped defaults; includes depth-first in list order; later wins with
      **both** origins recorded; a value array replaces where a `[[table]]` appends; a relative path
      resolves against the file it is written in. `file.rs:38` returns the `SourceId` for exactly
      this — an override has to be able to name where the value it kept came from.
- [ ] **Ownership is the trust boundary.** ADR 0103 § 6, printed in full in this goal's orientation
      pack: owner-or-root and not group/world-writable, for every file *and* its directory, and for
      the directory that *would* hold an absent `optional` include. `E0604` is taken; next free in
      the band is `E0605` (`crates/nvs-diagnostics/src/lib.rs:1179`).
- [ ] **`nvs config check <file>` prints the refusal.** Stage 2's one `command` check runs it over
      `tests/config/duplicate-key.toml`, which is on disk. `nvs-cli` is a file set this group has
      not loaded — take it last or leave it to the next session.

## Backlog

- ADR 0091 § 3a's table spells `Boot` in a **Class** column, which ADR 0078 § 2 split into two
  fields; the registry reads those two rows as `System` + `Boot`. One ADR edit, owned by 0091.
- ADR 0042 (~line 170) spells the artifact-cache directory `opcache.file_cache_dir` where ADRs 0005
  and 0078 § 2 spell it `cache.dir`. One of the two is the home; the registry took `cache.dir`.
- ADR 0078 § 2's `Boot` set names "the thread-per-core count" and no ADR spells it as a key, so it
  has no registry row. ADR 0106's is the block it would live in.
- `[debug]`, `[db.<name>]` and `[http.errors]` have no registry row: their owning ADRs state fields,
  not a changeability class. Whoever transcribes the typed tree can settle all three at once.
- `orient.py` warned that `[context] modules` pattern `crates/nvs-host/src/budget.rs` matches
  nothing — it moved or the glob is wrong. The `crates/nvs-config/src/*.rs` warning is now stale and
  will clear by itself.
- Item 18's `Core\Secret::reveal()`, `Live::admit`'s same-class check and item 22's `Core\Script`
  members are goal 2's three recorded gaps, each in its own crate's module doc.
