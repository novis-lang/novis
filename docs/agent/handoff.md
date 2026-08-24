# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**`Core\Json::decodeAs<T>` runs**, so spec § 6 is whole and `examples/json.mwl` produces its frozen
output. Stage 3 is six of seven fixtures; `examples/collect.mwl` is the first that does not.

- **A `Core` member can now be handed the class its call site wrote.** The roster is
  `mwl_stdlib::registry::WRITTEN_CLASS_MEMBERS` (one entry), and its docs own the ABI: the descriptor is
  argument 0, so the helper's `args: [N]` is one more than the row's `params`. The checker records it on
  `ResolvedCall::written_class`; `mwl-ir` emits an `InstKind::ClassDescConst` ahead of the arguments.
- **`mwl_runtime::CodecField` is the one shared codec record** — key, slot, constructor position, erased
  type, nullability — produced by `mwl_types::derive`, joined to the slot order in
  `mwl_ir::lower::lower_file`, consumed by `mwl_stdlib::json`. `mwl_types::derive::DerivedField` is the
  declaration-side half (it carries the property *name*, not a slot).
- **`mwl_runtime::construct` is how native code runs a class's constructor** — the transfer-direction
  twin of `call_closure`'s borrow direction; its own doc comment owns the ownership rule.
- What the decoder still owes is `mwl_stdlib::json`'s gaps 2, 3 and 6: no enum/`decimal`/`Instant`/
  `array`/nested-class field, no parameter default making a key optional, no dotted issue path.
- Verified: `python tools/verify.py` green, 1291 tests, 324 `.mwlt` cases. The new refcount edges are
  `valgrind`-clean on the success path; a *throwing* `decodeAs` in a loop still loses one block per
  iteration, sized exactly `literal + 16`, which is the backlog item below and not this edge.

## Next

**`Core\Path` — spec § 11.** It is the cheapest slice inside `examples/collect.mwl`: `join` (variadic,
which exists), `basename({withoutExtension})`, `extension(): ?string`, `SEPARATOR`, and no new
dependency. `docs/agent/loop-goal.md` § *Standing decisions* has the two-legs rule for `SEPARATOR` — a
case asserting a built path must normalize it.

## Backlog

- **`Core\Encoding`, `Core\Hash`, `Core\Uuid`, `Core\Csv`, `Core\Validate`, `Core\Random`, `Core\Out`,
  `Core\Uri::parseQuery`** — the rest of `examples/collect.mwl`; each needs a dependency picked under
  [ADR 0051](../adr/0051-standard-library-tiers.md) § 4 and its three obligations.
- **`Core\ObjectSet`/`ObjectMap`** — spec § 8, and the one item that needs *language* work first:
  `new Core\X<T>()` does not parse. `mwl_runtime::identity` is the comparison they need.
- **`Core\Time\Date`, `Core\Time\TimeOfDay`, `Core\Month`, `DateTime::date`/`timeOfDay`/`withTime`** —
  `time.rs`'s gap 1; the machinery all exists, so each is a registry row and a body.
- **`Core\Str::compare`, `chunk`, `lines`, `graphemes`, `codePoints`, `replaceAll`, `replaceRange`,
  `fold`, `normalize`, `fromCodePoint(s)`** — the rest of § 1. `mwl-stdlib`'s gap 1 is the list.
- **ADR 0069's `overlay`/`overlayDeep`/`underlay`/`appendAll`, plus `Arr::append`/`prepend`** — all
  declare the variadic that exists, so each is a registry row and a body.
- **A shape property read does not lower** — `mwl-ir`'s ADR 0036 § 4 gap: `$e->issues[0]->path` panics
  naming that ADR rather than reading slot 1, so an issue's own fields are unreadable from MWL.
- **A `Core` call that throws leaks a fresh string argument** — `mwl_ir::lower::landing_block`'s own
  *Known gap*: 50 loop iterations of a throwing `Core\Json::decode("…")` inside a `try` lose 50 blocks.
