# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `html.rs`, `response.rs` and `cli.rs` have **no
`# Known gaps` section left**. `python tools/owners.py --closes decided-closures` names **7**, down
from 10; the other two stage-6 gates are already green (`playbook.py --closes` names no row,
`owners.py --check --past-is-an-error` reports `past-milestone: 0`).

**`html.rs` gap 1 and `response.rs` gap 1 closed as *built*, not struck.** The goal sheet
(`docs/agent/loop-goal.md:113`) prices them as one gap still to build — "the sink in force selects a
rendering" — and that is behind the code. The automatic lift is on disk and tested:
`crates/nvs-runtime/src/helpers.rs:2515`'s `write_rendered` picks `nvs_render::html::escape` off
`ctx.carrier()` with no call at the site, `crates/nvs-host/src/isolate.rs:648` builds the
`OutputSink::Body` that names that carrier, and it is guarded twice — Rust
`helpers.rs:3285` and `tests/conformance/core/echo-in-a-request-escapes-a-string-and-writes-markup-as-it-is.nvst`.
So route 1 applied: the items are deleted and each module doc now says what is true.
`rule:core-classes/html-auto-escape` and `rule:security/response-body-is-one-typed-member` needed no
amendment — neither promises a rendering over a body member's bytes.

**`response.rs`'s four "gap 1" citations now name the section instead of a number**
(`:75`, `:1052`, `:1084`, `:1721`, `:1791`), because a section that is a bound outlives a gap list.

**One index row deleted, outside the goal**: `docs/agent/carried-gaps.md` claimed
`rule:core-classes/html-to-source`'s computed `$reason` is not refused. It is —
`crates/nvs-types/src/reasons.rs:120` emits `E0805` and `crates/nvs-types/tests/tainted.rs:304`,
`:319`, `:344` assert it — and the row also pointed at the `§ *Known gaps*` this session removed.

## Next group

**Stage 4: the three remaining `crates/nvs-stdlib` gaps** — one file set:
`crates/nvs-stdlib/src/regex.rs`, `ast.rs` and `command.rs`, each one module doc, plus
`crates/nvs-types/src/commands.rs` if the third turns out to need that crate's own gap 1 first.
Take them in this order: the disposition is cheap and the two builds are not.

- [ ] **`crates/nvs-stdlib/src/regex.rs:69` — gap 1, the cross-request compiled-pattern cache.**
      `CACHE` holds an `Rc` the compiling request holds too, so no accounting bracket can put the
      allocation and the release on one balance, and the doc already argues that bracketing it the
      way `Core\Cache`'s tier is bracketed would be *worse*. Read `crates/nvs-stdlib/src/regex.rs:69-90`
      for the rest of the sentence the register truncates ("it waits on per-request provenance,
      which is …") and the goal's disposition line for gap 1, then strike it as a stated bound or
      defer it to a milestone at M9+ whose plan states the scope — `rule:programs/memory-priority`
      is the rule it is written against, `rule:core-classes/regex-two-tiers` the one it must not
      contradict. Note that `docs/agent/loop-goal.md:110` disposes of regex gaps **2 and 3**, not
      this one.
- [ ] **`crates/nvs-stdlib/src/ast.rs:83` — gap 1, a build.** `Decided: Position
      (line/column/offset) only`, so a `#[Test]` walking the tree can name the `file:line` it failed
      about and no text comes back out, which keeps the input qualifier-neutral.
      `rule:core-classes/ast-is-inert` is what the addition is measured against, and it says what
      inertness costs. This is a real build with a `.nvst` case and a reference card per new member —
      expect it to spend the 120k by itself.
- [ ] **`crates/nvs-stdlib/src/command.rs:74` — gap 1, a build with a dependency.** `Decided: The
      help renderer reaches the handler's signature at render time (row holds a reference, not a
      copy)`, so the table points at the signature rather than copying it. The row's declared type
      is absent because spec § 6's table does not carry one — `nvs_types::commands`'s own gap 1 — so
      check whether that gap must move first; `rule:security/capability-check-at-the-door` is what
      the handed-in table is written against.

## Backlog

- The four gaps outside `nvs-stdlib`: `crates/nvs-diagnostics/src/embedded.rs:30`,
  `crates/nvs-syntax/src/lib.rs:96`, `crates/nvs-types/src/defaults.rs:58`,
  `crates/nvs-types/src/response.rs:29` — the register is `owners.py --closes decided-closures`.
- `docs/agent/loop-goal.md:113` is behind the code on `html.rs`/`response.rs` gap 1; harmless now
  that both are off the register, and left alone rather than edited under a running driver.
- Two retired goal files still cite sections this session removed:
  `docs/agent/goals/29-xml-tree.md:9,23` and `docs/agent/goals/57-m7-server-surface.md:51`.
- **`[context] modules` gap**: verifying this item needed `crates/nvs-runtime/src/helpers.rs`,
  `crates/nvs-runtime/src/ctx/output.rs`, `crates/nvs-render/src/html.rs` and
  `crates/nvs-host/src/isolate.rs`, none of which the pack's 33 patterns cover.
- **`[context] rules` gap**: `rule:tooling/echo-always-has-a-sink` is the table both closed gaps
  read their answer off, and the pack did not print it.
