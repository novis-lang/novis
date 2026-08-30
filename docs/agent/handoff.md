# Handoff

## State

**Goal 3, Stage 2 is landed: `crates/nvs-config` reads a configuration tree.** `tree.rs` is ADR 0064
§ 2a's whole block roster as typed structs with `deny_unknown_fields` on every one — 30 blocks, swept
by count in `tests/tree.rs` — and `resolve.rs` is ADR 0103 §§ 1-5: root selection, `[[include]]`
expansion depth-first with a cycle refusal and the depth cap, § 4's replace-vs-append split, § 5's
relative resolution, and one ordered stream where a later assignment wins **and every override is
recorded with both origins**. 31 tests green in the crate. The workspace `serde` gained `derive`
(`Cargo.toml:199`), which the comment above it now explains: `deny_unknown_fields` is an attribute
rather than machinery, so the macro replaces code here instead of adding any.

**Two new codes, and one that was not needed.** `E0605 E_UNREADABLE_CONFIG` and `E0606
E_INCLUDE_CYCLE` are in `nvs-diagnostics`. The item asked for an E06xx for the *unknown key* as well;
ADR 0064 § 3 settles that as the existing `E0601`, so the block name is a note on that diagnostic
instead (`file.rs`'s `block_at`) and the playbook bullet records why. ADR 0064 § 2a's table gained
`[cache]`, `[control]` and `[opcache]`, which the tree names and the table did not.

**Two things are deliberately not done yet.** § 6's ownership check belongs on the `Files` reader and
is the next slice, so `Disk` reads without one today and `resolve.rs`'s module doc says so. Cycle
detection is *lexical* (`normalize`, not `fs::canonicalize`), so a cycle built out of symlinks is
caught by `MAX_INCLUDE_DEPTH` rather than by the path comparison — § 6 has to `stat` every file
anyway, and canonicalization comes with it.

The acceptance check still fails on `examples/config.nvs` — `Core\Config` has no `get`. That is
Stage 3's snapshot and Stage 4's members, unwritten, not a regression. Nothing is blocked on the user.

## Next group

**The trust boundary, then the two indirections that sit on it.** One file set:
`crates/nvs-config/src/resolve.rs`, `crates/nvs-config/src/tree.rs`, `docs/adr/0103`, `docs/adr/0104`.

- [ ] **Ownership is the trust boundary.** ADR 0103 § 6 — owner-or-root, not group- or
      world-writable, on every file *and* its containing directory; an absent `optional` include puts
      the check on the directory that would hold it. It goes on the reader, not the resolver: `Files`
      is `crates/nvs-config/src/resolve.rs:51` and `Disk` its impl at `:73`, and the absent-optional
      branch that owes the directory check is `include_targets` at `:242`. Windows is an ACL check
      and § 6 says which ACEs it accepts is M6's to state — decide it and record it in that ADR.
      Canonicalize there and feed `same_file` (`:461`) the result, which is what closes a symlinked
      cycle properly; the standing decision says one path comparison, so write it once.
- [ ] **`password_file`.** ADR 0103 § 7 — the named file's whole content is the value, with **one**
      trailing newline stripped. The field is already on the tree (`crates/nvs-config/src/tree.rs:420`,
      `Database::password_file`) and exactly one of the pair may be set. It reads a file, so it is the
      first consumer of the check above and belongs right after it.
- [ ] **`[[app]]` matches and layers.** ADR 0104 § 2 — the entry path canonicalized, then prefix-matched
      on **path-component** boundaries, every matching block layered least-specific first, a block may
      grant as well as narrow, bounded by the global `[limits.hard]`. The struct is
      `crates/nvs-config/src/tree.rs:138`. Same canonicalise-then-prefix implementation as above.

## Backlog

- `Core\Config::get`/`set` and the snapshot a request clones — Stage 3, `docs/plan/m6.md`; this is what
  the failing acceptance check is waiting on.
- `[[server.mount]]` root containment and the mount's own origin — ADR 0097 § 4.
- ADR 0067 never writes a `[db.<name>]` block out, so `tree.rs:420`'s roster is prose-derived and its
  doc comment says so; the fix is an example in that ADR.
- `orient.py`'s `[context] modules` has a pattern for `crates/nvs-host/src/budget.rs` that matches
  nothing — the pack printed the warning itself. Fix or drop it in `docs/agent/loop-goal.toml`.
- `[context] adrs` printed 0103 §§ 3 and 6 only; every config slice needs **0064 §§ 2a and 3**, and
  this one also needed 0103 §§ 1, 2, 4, 5. Adding those five is worth roughly ten reads a session.
- ADR 0078 § 2 names "the thread-per-core count" as `Boot` and no ADR spells it as a key, so it is in
  neither the registry nor the tree — `directive.rs`'s module doc records it.
