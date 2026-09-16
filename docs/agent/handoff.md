# Handoff

## State

**Goal `unowned-closures`. The register is `unowned: 15` and goal-owned is down to 51**
(`python tools/owners.py`), `--deferrals` green. The 15 unowned are the scheduling questions,
none of them this goal's own gaps, so `unowned: 0` still turns on answers only the user can give.

**`crates/nvs-types/src/intrinsics.rs` holds no `# Known gaps` block any more**: gap 3 was the
last one and it is struck, built rather than deferred. An unterminated string literal in a
literal query is now refused while compiling — `rule:core-classes/db-literal-query-checking`'s
third clause, which the file used to decline because `nvs_db::sql` declined it in the other
direction.

**The disagreement was settled in the rewriter's favour by making the rewriter refuse too.**
`nvs_db::sql`'s scan now tells a closed region from one the text never leaves (`Region`), and
`rewrite` returns the `LogicError` rather than bind a statement against what fits inside an
opening delimiter. `holds_an_unterminated_region` is the same question asked of a text with no
arguments beside it, which is what `nvs_stdlib::db::check_closed_regions` needs for a call whose
params array the checker could not read whole. All four dialects, first acceptance ends it — so
an unterminated backtick is MySQL's refusal alone and no compile-time one.

**What it covers is every region, not only the literal**: a `'…'`, a `"…"`, a backtick- or
bracket-quoted name, a `$tag$…$tag$` body and a `/*…*/` comment. A `--` or `#` comment opens
nothing. The module doc's *Malformed SQL is the server's diagnosis* paragraph is rewritten
around that carve-out and owns the reasoning.

**Nothing on the request path costs more**: the scan already entered these regions, and the
refusal is the branch it used to take silently. The goal's one ADR slot is spent (0189), and this
item opened none.

## Next group

**Stage 5: the server and the cache** — one file set: `crates/nvs-db/src/catalog.rs`,
`crates/nvs-db/src/schema.rs`, `crates/nvs-db/src/ddl.rs`. Gaps 1 and 2 share one `Decided:`
sentence, so they are one design taken in two slices; gap 3 is a vocabulary case in the same
files. `rule:core-classes/schema-introspection` is what all three sit inside.

- [ ] **Fold both sides of the diff through the dialect's map before comparing, and make SQL
      Server's assembled spelling carry `DATETIME_PRECISION`** — `crates/nvs-db/src/catalog.rs:78`
      (gap 1) is the slot; `assemble` is where the precision is dropped and
      `crates/nvs-db/src/ddl.rs:82` (gap 1) is the write direction that emits one precision for
      every instant column. The `Decided:` sentence is one normalisation pass owning every lossy
      case, with the diff left a plain equality.
- [ ] **Normalise the declared side the same way, so a `uint32` written and an `int64` read back
      are an empty plan** — `crates/nvs-db/src/catalog.rs:86` (gap 2), over
      `crates/nvs-db/src/catalog.rs:499` (`scalar_type`, whose doc names every case) and
      `crates/nvs-db/src/schema.rs:320` (`ColumnDefault`). Same `Decided:` sentence as gap 1, so
      the pass written there is the one this slice finishes.
- [ ] **Add the opaque, read-only `ColumnDefault` case and narrow the unquoted fallback to
      MySQL** — `crates/nvs-db/src/catalog.rs:95` (gap 3), landing in
      `crates/nvs-db/src/catalog.rs:846` (`unquote`) and `crates/nvs-db/src/catalog.rs:734`
      (`column_default`), with the case declared at `crates/nvs-db/src/schema.rs:320`. Compared
      verbatim, never emitted, and not constructible from a program.

## Backlog

- The `[context] modules` manifest gained `crates/nvs-db/src/schema.rs` and `ddl.rs` for the group
  above; `crates/nvs-db/src/sql.rs` and `crates/nvs-stdlib/src/db/check.rs` are still outside it
  and were read by anchor.
- `crates/nvs-stdlib/src/path.rs` gaps 1 and 2 (UNC, drive-relative) are one file set for a later
  group — `docs/agent/carried-gaps.md`.
- `crates/nvs-stdlib/src/zip.rs` gaps 1 and 2 (Zip64, entry CRC) likewise.
