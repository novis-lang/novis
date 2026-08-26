# Handoff

## State

**Stage 4's two counts are the frontier — conformance 447 of 600, differential 90 of 150** — and the gap
is behavioural depth per member, not coverage: every registered member already has a case, and both of
Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean) and
`mwl test tests/conformance` is **447 passed, 0 failed** — run it as well as `verify.py`, which executes
no `.mwlt` case at all (playbook, twice).

**`Core\Validate` is done to depth at 3 cases and `Core\Out` at 2.** Validate's four length limits — a
label of 63, a hostname of 253, a local part of 64, an address of 254 — are each asserted on *both* sides
and built with `Core\Str::repeat`, so the case pins the boundary rather than a constant near it; the
address limit is shown to be its own by asking `isDomain` about the domain of the address just refused.
Beside that: the empty label as one rule wherever it falls, a non-ASCII domain refused against the punycode
that passes, `isIp`'s families each refusing the other's spelling (`::ffff:192.0.2.1` is v6, is not v4),
v4's leading-zero refusal against v6's ordinary `0db8`, and `isAscii`/`isPrintable` crossing at `DEL` and
at `é`. `Core\Out::capture` pins that the closure's *result* is discarded, that a captured carrier outlives
the next capture, that a throw caught inside an enclosing capture leaves that level collecting, and that
`{through:}` runs after the level has closed — so its own `echo` reaches the enclosing sink, which is
visible in the expected output's ordering.

A `for` header, `!`, a ternary, `Core\Str::repeat`/`slice`/`startsWith` and a closure with any return
type passed as a `callable` all lower, so a *built* subject and an invariance sweep are available to a
`.mwlt` case wherever a member has a boundary or an identity to compare against.

## Next group

Three thin sections, each its own domain module plus its `tests/conformance/core/<name>-*.mwlt`. The file
set they share is `crates/mwl-stdlib/src/{heap,uuid,path}.rs` and `tests/conformance/core/`. Item 3 lives
inside the standing decision about `Path::SEPARATOR` differing between the two legs, so a case must
normalize a built path or assert something separator-free.

- [ ] **`Core\Heap` depth** — `crates/mwl-stdlib/src/heap.rs:95` `constructor`, `:112` `push`, `:119`
      `peek`, `:126` `pop`, `:133` `count`, `:140` `isEmpty`; spec § 9. Two cases today
      (`heap-iterates-in-pop-order.mwlt`, `heap-orders-by-comparable-or-by-its-comparator.mwlt`). What is
      thin is the *edges*: `peek`/`pop` on an empty heap, equal keys, a `count` that survives interleaved
      push/pop, and whether an element pushed twice is held twice.
- [ ] **`Core\Uuid` depth** — `crates/mwl-stdlib/src/uuid.rs:121` `v4`, `:128` `v7`, `:135` `parse`,
      `:142` `tryParse`, `:150` `toString`; spec § 11. Two cases today. `parse`/`tryParse` want the same
      malformed subjects each way — the pair is the point — and `v7`'s time ordering wants more than two
      draws.
- [ ] **`Core\Path` depth** — `crates/mwl-stdlib/src/path.rs:589` `join`, `:664` `normalize`; spec § 7.
      Two cases today. `normalize` wants `..` at and past the root, a trailing separator, `.` alone; `join`
      wants an absolute right-hand side and an empty segment.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder and ADR 0071's non-scalar fields — `mwl_stdlib::json` gap 2.
- ADR 0088's registry-wide qualifier classification for every `Core` member row — `mwl_stdlib::hash` doc.
- ADR 0090 § 3's string/array/object equality helpers — the plan's `Open now`.
- `do`/`while` does not lower — `mwl-ir`'s own module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- Differential is 90 of 150 and no session has moved it lately — `docs/plan/m4s.md`.
