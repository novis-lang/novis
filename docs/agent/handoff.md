# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `crates/nvs-stdlib/src/test.rs` has **no
`# Known gaps` section left**. Gap 1 is struck as a stated bound: the object row's refusal of a
`compareTo`-less class is written as module prose — a catchable throw naming `assertEqualsDeep`,
made when the comparison runs. Gap 2 is built. `python tools/owners.py --closes decided-closures`
names 12, down from 14.

**The class table is part of a host's wiring now, and the assertion is at the read rather than at
construction.** `crates/nvs-runtime/src/ctx/wiring.rs`'s module doc is the embedding contract's
home and states that a context with no exception class table is not a supported configuration;
`Ctx::pending_conforms_to` (`crates/nvs-runtime/src/ctx/error.rs:590`) carries the `debug_assert!`,
because a context takes its table *after* `Ctx::new` by design and there is no moment at
construction to check. A `#[cfg(debug_assertions)]` unit test sits beside it, and a release build
keeps `set_runtime_error_class`'s "never a crash". The playbook bullet under *Writing Novis itself*
owns the general shape.

## Next group

**Stage 4: `crates/nvs-stdlib/src/db/mod.rs`'s two gaps** — one file set, `Core\Db`'s entry points:
`crates/nvs-stdlib/src/db/mod.rs` alone, plus the fragment each item's rule owns.

- [ ] **`crates/nvs-stdlib/src/db/mod.rs:231` — gap 1, struck as a stated bound.** The `Decided:`
      sentence is *the defaults apply, and a deployment that wants bounds writes a block*, so the
      work is prose: write what the module does now — an `open` naming an endpoint no `[db.<name>]`
      block describes connects under the default pool bounds, and that is the limit — then delete
      the numbered item and its `— owner:` line. Amend `rule:core-classes/db-connection-is-named`'s
      fragment in the same slice if it promised a refusal instead.
- [ ] **`crates/nvs-stdlib/src/db/mod.rs:257` — gap 2, struck as a stated bound.** The `Decided:`
      sentence is *the split stands: a shape mismatch is a `ParseError`, as for `Json::decodeAs`*.
      `Db\DbError` sits outside spec § 10's error tree and declares no `issues`; state that split as
      the module's own prose, naming where each of the two lands, then delete the item and its
      `— owner:` line.
      `rule:core-classes/db-error` is the fragment to amend if it reads otherwise.

## Backlog

- `crates/nvs-stdlib/src/ast.rs:83` (a node gains a position, and no text) and `command.rs:74` (the
  help renderer reaches the handler's signature at render time) — two builds, separate file sets.
- `crates/nvs-stdlib/src/cli.rs:132` — struck as a bound; a served request answers empty, neutral
  values and the contract says so. Prose only, so it pairs cheaply with a build.
- `crates/nvs-stdlib/src/regex.rs:69` and `response.rs:197` — the two left in `nvs-stdlib` after
  `db/mod.rs`.
- `crates/nvs-syntax/src/lib.rs:96`, `crates/nvs-types/src/defaults.rs:58`,
  `crates/nvs-types/src/response.rs:29`, `crates/nvs-diagnostics/src/embedded.rs:30`,
  `crates/nvs-stdlib/src/html.rs:73` — the five outside `nvs-stdlib`'s library stage.
- `Core\BigInt` and `Core\Test`'s `double` half stay `unowned` ratchet keys — the user's call,
  indexed in `docs/agent/carried-gaps.md` § *Unowned*.
