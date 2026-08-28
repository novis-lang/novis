# Handoff

## State

**M4's Stage 8, depth.** The tree is at **837 conformance plus 189 differential**, all green.
Nothing is blocked.

`Core\Debug` was `gaps.py`'s thinnest class (depth 3.5, floor 2) and both its members gained a
case over `crates/nvs-stdlib/src/debug.rs`'s own doc comments.

- **`dump` is `render` per argument plus a newline.** Seven subjects — a control byte, three
  container shapes, a `secret` property and the same ring twice — go through `render` on standard
  output and through `dump` on standard error, once as one variadic call and once as seven, and
  the two expectation sections hold the same bytes twice over. The ring pair is the row a
  record-wide cycle table would fail: `dump`'s ids restart per argument exactly as `render`'s do
  per call. Counted beside it: none of the seven renderings ends in a newline and six hold one
  inside, so the trim is trailing-only; and a `dump` inside `Core\Out::capture` is not swallowed.
- **A repeat is a cycle only when it is an ancestor.** Six graphs — a self-loop, a two-node loop,
  a loop closing on the *middle* node, and three sibling repeats through a property, a container
  and a container behind a property — asked one question and counted: three hold a marker, six
  have every marker's id printed above it, and the two sibling repeats render the object whole
  both times rather than dropping the second. `Seen::leave`'s one-sentence rule, which the
  existing self-cycle row cannot distinguish from "any repeat is a cycle".

Two harness facts this cost time to find are now playbook bullets: an error expectation forces a
failing run, and `fn (): void => call()` panics `nvs-codegen` where the braced form does not.

`orient.py`'s pack was complete for this group; nothing outside it was read.

The gap sixteen handoffs back still stands: **no `Core` class reaches
`nvs_hir::implements_interface`**, so `Core\Uri::compareTo` exists while `$a < $b` over two `Uri`s
is `E0411`. It is in the backlog and still deserves a session of its own.

## Next group

**`Core\Uri`**, `gaps.py`'s next floor-2 class after `Core\Debug` (depth 4.0, floor 2). One shared
file set: `crates/nvs-stdlib/src/uri.rs` plus `tests/conformance/core/`. Re-run
`python tools/gaps.py` first — if these three have moved off the floor, take its top class instead.

- [ ] **`Core\Uri::tryParse`** (2 cases) — registry row `crates/nvs-stdlib/src/uri.rs:354`, helper
      `crates/nvs-stdlib/src/uri.rs:1452`. Read its doc comment for the rule no case observes; the
      *agreement* shape is unspent here — `tryParse` answering `null` exactly where `parse` throws,
      over a sweep of subjects, counted.
- [ ] **`Core\Uri::buildQuery`** (3 cases) — row `uri.rs:396`, helper `uri.rs:1851`. Invariance over
      a sweep: what a built query round-trips back to.
- [ ] **`Core\Uri::compareTo`** (3 cases) — row `uri.rs:506`, helper `uri.rs:1689`. Take this only
      if the context gate still allows it, and note that `$a < $b` over two `Uri`s is the `E0411`
      in the backlog rather than something a case can assert.

## Backlog

- **No `Core` class reaches `nvs_hir::implements_interface`** — `Comparable` on a `Core` class is
  unreachable from source; ADR 0013, and a session of its own.
- **A `.nvst` cannot assert the stderr of a run that exits 0** — the playbook bullet above says why;
  a section meaning that would be a new one, owned by `crates/nvs-test`'s module doc.
- **`fn (): void => call()` panics `nvs-codegen`** — `crates/nvs-ir` or `crates/nvs-codegen`, not
  a test-suite problem; the braced form is the workaround.
- **`secret` in an `array<T>` element or an ADR 0036 shape field is not redacted** —
  `nvs_stdlib::debug`'s known gap 1, ADR 0033's unmodelled container axis.
- **An enum case dumps as its backing integer** — `nvs_stdlib::debug`'s known gap 2, ADR 0010 § 5.
- **54 of `loop-goal.toml`'s 156 guard test names match nothing** — `docs/agent/guard-name-debt.md`.
