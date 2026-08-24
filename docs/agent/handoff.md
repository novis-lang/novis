# Next session prompt

Where the work stands right now. This file is **state**, overwritten every session and never appended to.
The traps and recipes that outlive a session are in [playbook.md](playbook.md); the rules that bind every
agent are in [AGENTS.md](../../AGENTS.md); `python tools/brief.py` is the rest of the orientation.

## State

**The live path is unchanged: `Core` breadth for Stage 3.** Six of seven fixtures produce their frozen
output; `examples/collect.mwl` is the first that does not. **No code changed this session** — the commit is
documentation only, and it carries the work of **two sessions at once** (see *Note on the commit* below).

- **[ADR 0086](../adr/0086-core-cli-terminal-is-a-sink.md) is written and nothing is built.** It designs
  `Core\Cli` — which is four members today against a milestone titled *a usable CLI language*. Five
  decisions: terminal output is an [0024](../adr/0024-taint-tracking-for-injection-sinks.md) sink that
  substitutes control bytes with **visible** glyphs (`ESC` → `␛`) in every value regardless of qualifier;
  styling is the `Cli\Text`/`Style`/`Color` value types, never a fifth
  [0063](../adr/0063-core-api-conventions.md) R11 grammar; the five prompts are `Core` members because raw
  mode is unreachable with [0052](../adr/0052-closed-doors.md)'s FFI door shut; in-place output is a scoped
  `live`/`progress` region; and `#[Command]` builds the argument table while compiling, the
  [0077](../adr/0077-compile-time-routing.md) mechanism with the route table swapped out.
- **Why the sink is a default rather than a refusal.** HTML auto-escaping transforms *visible* text
  (`&`→`&amp;`) and 0024 § 5 paid for that surprise. A terminal control sequence is not text — today it is
  consumed by the terminal and shown to nobody — so substituting it visibly makes `echo` *more* faithful,
  not less. That asymmetry is the whole argument, and 0024 § 5 now carries it.
- It **amends five ADRs**, each folded into its body: [0024](../adr/0024-taint-tracking-for-injection-sinks.md)
  § 4 (the sink roster) and § 5 (no longer "the one exception"); [0033](../adr/0033-secret-qualifier-for-confidential-values.md)
  § 1 (`Cli::secret` is the first `Core` member that *originates* the qualifier);
  [0020](../adr/0020-error-escalation-ladder.md) § 4 (the floor restores the terminal — a `finally` cannot,
  since § 5's panics bypass user code); [0051](../adr/0051-standard-library-tiers.md) § 3 (roster gains
  `Core\Command`); [0071](../adr/0071-derived-codecs.md) § 1 (three more compiler-recognized attributes).
- **Do not start it while Stage 3 is open.** Its M4S slice is the `#[Command]` table beside `#[Route]`'s;
  everything else is M8, since neither argv nor a terminal is reachable before capabilities exist at M6.
- Verified: `python tools/check-links.py` clean across 93 files, `python tools/verify.py` 4/4 green. The
  plan's status block was deliberately **not** edited — 0086 is scheduled, not in flight, and `Open now` is
  already 8× its size target.

## Note on the commit

ADRs **0080–0085** and the `0067` § 13 rewrite in this commit are **another session's work**, written
concurrently into the same tree and committed here because the doc set is only internally consistent with
all of it present — 0086 cites 0080 and 0082, and their README rows cite each other. They were not reviewed
by the session that wrote 0086. If something in that range looks half-finished, `git log` will not
distinguish the two authors; this paragraph is the only record that they were separate.

Two sessions writing the same tree also nearly collided on ADR numbering: 0084 and 0085 were referenced by
name from 0082 before their files existed, which is what pushed the CLI ADR to 0086. **Check `git status`
for untracked `docs/adr/NNNN-*.md` before claiming a number**, not just `ls`.

## Next

**`Core\Path` — spec § 11.** Unchanged and still the cheapest slice inside `examples/collect.mwl`: `join`
(variadic, which exists), `basename({withoutExtension})`, `extension(): ?string`, `SEPARATOR`, and no new
dependency. `docs/agent/loop-goal.md` § *Standing decisions* has the two-legs rule for `SEPARATOR` — a case
asserting a built path must normalize it.

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
- **[ADR 0079](../adr/0079-testing-is-a-language-feature.md)'s first slice, after Stage 3** — `#[Test]`
  parsing plus the compile-time table (§ 1) and the generic `Core\Test` assertion roster (§ 4).
- **[ADR 0086](../adr/0086-core-cli-terminal-is-a-sink.md)'s M4S slice, after Stage 3** — the `#[Command]`
  table beside `#[Route]`'s, sharing [0061](../adr/0061-compile-time-autoload-and-program-discovery.md)
  § 3's enumeration. Its *Verification* section is the fixture list.
