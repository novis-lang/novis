# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**Stage 0 still comes before `Core` breadth, and this session did not touch it.** The catch-up list in
[loop-goal.md](loop-goal.md) § *Stage 0* is unchanged and item 1 — ADR 0090 § 1's lexer removal — is still
the next thing to build. This session was documentation only: two new ADRs and their folds. `verify.py` is
green and no Rust changed.

**[ADR 0091](../adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) gives MWL a run
mode, which it did not have.** Two closed values, `development` and `production`, and **with nothing
configured the mode is production**. It is set by `[mode] default` in root-owned `mwl.toml` and by
`mwl serve --mode=`, which overrides the file; **no environment variable is ever read for it**. A mode
selects the defaults of **four** directives and governs nothing else — `[debug] inline`, `[log] format`,
`[log] level`, `[http.errors] detail` — each still individually settable, which is what keeps it
enumerable rather than a `NODE_ENV`-style bundle. `Core\Env::mode()` reads it and `Core\Config::set` flips
it per request, bounded by `[mode] ceiling`, a `System` directive that **defaults to the mode the server
started in** — so a production host is unreachable from code with nothing written, and one host serving
mixed applications is one line. It reuses [0005](../adr/0005-config-changeability.md)'s `[limits]` /
`[limits.hard]` shape and asserts there will be no third instance of it.

**[ADR 0092](../adr/0092-one-diagnostic-record-three-renderings.md) makes every developer-facing output
one record with three renderings.** Plaintext, JSON and HTML, **chosen by the sink already in force** —
[0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 3's binding table gains a
column and § 5's carrier rule is reused, so there is **no new carrier type and no format argument
anywhere**. Five producers share the model: `Core\Log`, `Core\Debug::dump`, a `Throwable` and its trace, a
`#[Test]` result, and a compiler diagnostic. Redaction, control-byte substitution, bidi and elision are
decided **once, in the model**, which is why the scope is five and not two. `Log\Level` is five cases
(`Debug`/`Info`/`Warn`/`Error`/`Critical`) with a fixed syslog mapping; a dump goes to the log by default
and reaches a response body only under `[debug] inline`, **never** for a `Response::json` body.

**[ADR 0093](../adr/0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md) arrived in the tree
from another session while this one was working** and is committed alongside these two; it is not this
session's work and was not reviewed here.

**None of 0091–0093 is catch-up.** They invalidate no built behaviour and no written fixture, unlike 0090.
`loop-goal.md`'s *Not in this stage, deliberately* paragraph names them and the milestone each piece
belongs to. **Do not start any of them while Stage 0 is open.**

## Next

**ADR 0090 § 1 — delete the two spellings** (Stage 0 item 1, M1), unchanged from the last handoff. The
lexer stops producing `===`/`!==` with a diagnostic in the E00xx band naming `==`/`!=`, the shape ADR
0034/0045 already use; `BinaryOp::Identical`/`NotIdentical` come out of the AST with them; and the 45
`.mwl`/`.mwlt` files that write the rejected spelling are rewritten in the same commit. Name the guard
test `a_rejected_equality_spelling_is_a_compile_error`, which [loop-goal.toml](loop-goal.toml) already
requires.

## Backlog

- **The rest of Stage 0, in [loop-goal.md](loop-goal.md)'s order** — ADR 0090 § 2's disjoint-operand
  refusal and § 3's narrowing plus three helpers, ADR 0047 § 4's atoms, `private`/`protected`,
  `Comparable`/`Stringable`'s member signatures, ADR 0061's `autoload`, ADR 0069's `array + array`.
- **`Core\Path` — spec § 11** — the cheapest slice inside `examples/collect.mwl` and the first thing after
  Stage 0: `join` (variadic, which exists), `basename({withoutExtension})`, `extension(): ?string`,
  `SEPARATOR`, no new dependency. loop-goal.md § *Standing decisions* has the two-legs rule for `SEPARATOR`.
- **`Core\Encoding`, `Hash`, `Uuid`, `Csv`, `Validate`, `Random`, `Out`, `Uri::parseQuery`** — the rest of
  that fixture; each needs a dependency picked under [ADR 0051](../adr/0051-standard-library-tiers.md) § 4.
- **`Core\ObjectSet`/`ObjectMap`** — spec § 8, and the one item needing *language* work first:
  `new Core\X<T>()` does not parse. `mwl_runtime::identity` is the comparison they need, and is also what
  ADR 0090 § 3's object row lowers to.
- **The registry's qualifier classification** — [0088](../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
  § 2: a per-parameter field on `mwl-stdlib`'s member rows, the fail-closed default, and the test that
  refuses an unclassified member. Lands with M4S's remaining sections.
- **`Core\Time\Date`/`TimeOfDay`/`Month`, `DateTime::date`/`timeOfDay`/`withTime`** — `time.rs`'s gap 1;
  the machinery exists, so each is a registry row and a body. Same for the rest of § 1 and ADR 0069's
  combination members.
- **0092's own follow-ons, both deliberately deferred and both named in its § 8** — a compile-time log
  field schema (`mwl check` refusing two call sites that use one field name with two types) and scoped
  context fields (`Log::with`). The record model is shaped so neither is a rewrite.
