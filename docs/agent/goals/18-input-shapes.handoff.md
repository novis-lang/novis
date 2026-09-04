# Handoff

## State

**Goal 18 — untrusted input becomes a declared shape, at one converter — has just started; nothing of it
has landed yet.** Goal 17's whole list is this goal's Stage 1 floor.

The scope line matters here: **this goal does not make array key access shape-checked.** It adds one
converter from `array<mixed>` to a declared shape and two request members over it. A session that finds
itself giving `array<T>` per-key types has left the goal — that design was considered and rejected in
*Standing decisions*, and ADR 0036 rejected the general form of it before that.

What the goal is really buying is one thing said twice. `Core\Request` answers `mixed` at every reader, so
nothing between the socket and the third frame of a handler is checked; and the type surface that would
check it — ADR 0036 § 3's inline shape — cannot describe a form, because every field it names is required
and a qualifier cannot reach it. Stage 2 fixes both, and stages 3 and 4 are then ordinary `Core` members.

## Next group

**Stage 2: the grammar** — one file set: `crates/nvs-syntax/src/parser/ty.rs`,
`crates/nvs-types/src/ty.rs`, `crates/nvs-types/src/expr/assign.rs`.

- [ ] **`Ty::Shape` gains a required bit per field** — `crates/nvs-types/src/ty.rs:250`. The bit exists
      one struct away: `CoreShapeField::required` (`:359`) is this for a Core options bag under ADR 0135,
      so reuse that representation rather than inventing a second. Parser: `{name?: T}` in
      `crates/nvs-syntax/src/parser/ty.rs`. `{a?: T}` and `{a: ?T}` stay **different types** — key may be
      absent, versus key present holding `null`.
- [ ] **`tainted {…}` desugars at parse time** — ADR 0024 § 1's grammar widened to admit a shape.
      Taint is variants, not an axis (`Ty::TaintedString` and friends, `ty.rs:48-62`), so the qualifier
      rewrites each text-carrying field to its tainted variant transitively and is gone before the
      checker. A shape naming no text-carrying field is a diagnostic, not a no-op.
- [ ] **`is_assignable` learns the bit** — `crates/nvs-types/src/expr/assign.rs`: missing optional
      satisfies, missing required does not, extra fields still satisfy (ADR 0036 § 3, unchanged).

## Backlog

- Stage 3 (`Core\Arr::shapeAs<T>`, the five edits, hydrating through `as` and collecting every failure
  into one `ParseError` the way ADR 0071's derived hydration does) is the second group. It shares
  `nvs-types` with stage 2 through `expr/args.rs` — the third written type argument — and adds
  `crates/nvs-stdlib/src/arr.rs` and `json.rs`.
- Stage 4 (`postAs`/`queryAs` beside `post`) is the third. `crate::uri::parse_query` already answers the
  whole array `post(name)` indexes into (`request.rs:1229`), so the whole-form spelling walks nothing new
  — and it must not open a public `post(): array<mixed>`.
- Stage 5 is the fixture, the reference pages and stage 2's diagnostic corpus.
- **One inherited question, settled in stage 2 rather than deferred**: goal 16 specifies
  `Core\Request::json(): tainted mixed`, which ADR 0024 § 1's grammar admits no more than it admits
  `tainted {…}`. Either the widening covers `mixed` too or goal 16's signature is corrected to what the
  grammar allows. Decided-and-recorded in ADR 0024's body, never `BLOCKED`.
- **When this goal's last check goes green the driver switches to goal 19** — ADR 0141's `Parses`, which
  opens the route-capture and command-argument roster to any class declaring it can be built from text.
  This goal's whole list becomes its floor.
