# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**Spec § 10's `issues` on `ParseError` is built end to end** — ADR 0071 § 5's list, which the generated
decoder will report *through*. Stage 3 is still five of seven fixtures; `examples/json.mwl` now reports
only `decodeAs<T>`.

- **`ParseError` is the one class in the tree with state of its own.**
  `mwl_hir::errors::OWN_PROPERTIES` is that roster and its docs say why the root does not carry the slot
  instead. A class with own properties also gets its own synthesized constructor
  (`mwl_ir::lower::exception::synthesized_exception_constructors`), because ADR 0022 needs the slot
  definitely assigned and `array<Issue>` cannot read `null`.
- **`Core\Issue` is ADR 0036's shape, not a class** — `{path: string, message: string}`, typed in
  `mwl_types::error_lib::issue_shape` and built in `mwl_stdlib::issue`, whose docs own the slot order
  (sorted, because the interner canonicalizes a shape's fields) and the one gap: `$issue->path` is a
  shape property read, which `mwl-ir` does not lower.
- **A helper reports one** with `Fault::thrown_with_issues`, which `run_helper` hands to
  `Ctx::raise_with_issues` — that builds the exception eagerly rather than leaving an owned reference in
  the pending state; `mwl_runtime::abi`'s variant docs say why. `Core\Json::decode` on malformed syntax
  records one issue.
- Verified: `python tools/verify.py` green, 409 `.mwlt` cases pass, acceptance reaches the same fixture it
  did before. The new refcount edge is `valgrind`-clean (`tools/leak-check.sh`); the 22 bytes a
  `Core\Json::decode("{oops}")` inside a `try` still loses is the *fresh string argument* backlog item
  below, not this edge.

## Next

**`Core\Json::decodeAs<T>`**, the last of spec § 6's four members. Three pieces, in this order:
give `derive::CodecField` the declared type a decoder checks against; reach the target class from native
code — the call site knows it, and `InstKind::ClassDescConst` already rides a descriptor in a `Value`'s
payload under `Tag::Null` (`mwl_codegen::ty::tag_of`), so a closed roster in `mwl_stdlib::registry` naming
the members that take one plus a `written_class` on `ExprTypeTable`'s `ResolvedCall` is the cheapest route
that touches no other registry row; then the decoder itself, accumulating `issue::list` entries and
throwing once before `ClassDesc::method("constructor")` runs (ADR 0071 § 5).

## Backlog

- **`Core\Time\Date`, `Core\Time\TimeOfDay`, `Core\Month`, and `DateTime::date`/`timeOfDay`/`withTime`** —
  `time.rs`'s gap 1; the machinery all exists, so each is a registry row and a body.
- **`Core\Str::compare`, `chunk`, `lines`, `graphemes`, `codePoints`, `replaceAll`, `replaceRange`,
  `fold`, `normalize`, `fromCodePoint(s)`** — the rest of § 1. `mwl-stdlib`'s gap 1 is the list.
- **ADR 0069's `overlay`/`overlayDeep`/`underlay`/`appendAll`, plus `Arr::append`/`prepend`** — all four
  declare the variadic that exists, so each is a registry row and a body.
- **`Arr::diff`/`intersect`** — want a `Core\SetOn { Values, Keys, Both }` in `registry::ENUMS` and an
  `{on?, by?, comparator?}` bag; `docs/spec/01-core-library.md` § 2 *Combining* has the rules.
- **A shape property read does not lower** — `mwl-ir`'s ADR 0036 § 4 gap, now reachable from `Core`:
  `$e->issues[0]->path` panics naming that ADR rather than reading slot 1.
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*: 50 loop iterations of a throwing `Core\Json::decode("{oops}")` inside a `try` lose 50
  blocks, one per string literal argument.
