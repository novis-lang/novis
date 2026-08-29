# Handoff

## State

**Goal 1's stage-7 acceptance check is closed.** `python tools/check-migration.py --min 33` reports
34% covered over 393 rows, up from 25%/291. The five domains goal 1 owned are written out in
[docs/spec/02-php-migration.md](../spec/02-php-migration.md): dates and times (73 rows), regular
expressions (11), JSON (5), URLs and query strings (7), paths (6). No goal-1 family name is still
unclassified — of the 758 open, 46 are `stream_*` and the rest belong to goals 2-6 by the prefix
attribution in `loop-goal.toml`'s stage 7 comment header, which is that attribution's only home.

**Item 12's roster: `Core\Bytes` is done, 50 members across nine classes remain.** The twelve rows at
`crates/nvs-stdlib/src/bytes.rs:171` carry `CoreTy::Blob(Qual::…)`/`Text(Qual::…)` and are off
`UNCLASSIFIED` at `crates/nvs-stdlib/src/registry.rs:1526`. Ten were mechanical — the rule at
`registry.rs:74`, cross-checked against the already-classified `Core\Str` twin of the same name — and
`pack`/`unpack`'s format string is R11's fourth grammar, so `Sink`. One is a judgement and is recorded
where it belongs: **`Core\Bytes::at` is `Contagious` although it answers a `uint`**, because that
`uint` is a byte of the subject and `fill` plus `join` reassemble a buffer from those numbers;
`bytes.rs`'s module doc bullet on `at` owns the reasoning. Remaining: `Path` 9, `Time` 6, `Uri` 9,
`Test` 8, `Validate` 6, `Hash` 4, `Uuid` 2, `Router` 2, `Csv` 2.

**Nothing consumes `Qual` yet** — it is declarative in `nvs-stdlib` and no `nvs-types` code reads
`classification()`, so a classification pass cannot change what a program does today. The playbook
bullet saying no `Core` member accepts a qualified argument is still true for that reason.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale, a `bytes` array
key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last two have playbook bullets under
*Writing a test case*.

**Orientation gaps.** The pack named neither `docs/spec/02-php-migration.md` nor
`tools/check-migration.py`, though stage 7's check is the goal's own measure and the failing check
routed this whole session at them: add both to `[context] docs`, and add the stage-7 comment header's
per-goal floor table, which had to be read out of `loop-goal.toml` by hand. Still owed from before: a
selector that prints the *failing* check's own `cases` block, `[context] modules` naming an
`nvs-stdlib` class module, a `[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`, and ADR
0088 § 1, 0063 R11, `0085 §§ 1-4`, `0071 §§ 2, 7`.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/registry.rs` — every slice deletes its class's lines from
`UNCLASSIFIED` at `registry.rs:1526` and reads the rule at `registry.rs:74` — plus one class module
each. Item 12 continues in roster order. Each class's `Core\Str` twin, where one exists, is the
cross-check; `Sink` is only ever R11's four grammars.

- [ ] **`Core\Path`'s nine rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/path.rs:72`. No member
      touches the disk and none is a grammar, so every row is the two ordinary bullets: `isAbsolute`
      answers a `bool` and is `Neutral`; the other eight answer text built from the path and are
      `Contagious`. `normalize` is explicitly **not** a launderer ([01 § 8](../spec/01-core-library.md)).
- [ ] **`Core\Time`'s six rows** (ADR 0088 § 2) — `Core\Time::fromIso` at
      `crates/nvs-stdlib/src/time.rs:1130`, `Core\Time::parse` at `time.rs:1137`,
      `Core\Time\DateTime::format` at `time.rs:914`, `Core\Time\Date::format` at `time.rs:1259`,
      `Core\Time\TimeOfDay::format` at `time.rs:1392`, `Core\Time\Duration::parse` at `time.rs:190`,
      `Core\Time\Zone::of` at `time.rs:622`. Every `format`/`parse` **pattern** is a CLDR pattern and
      so `Sink`; the `$text` being parsed stays `Contagious`. `Duration::parse` is the one member in
      the class whose doc already claims it **launders** a tainted config value.
- [ ] **`Core\Uri`'s nine rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/uri.rs:343`. The component
      accessors answer parts of the argument, so `Contagious`; watch `encodeComponent` /
      `encodeFormValue`, which look like launderers and are not — a percent-encoding launders for no
      sink this tree declares, and `Launder` is claimed by a doc comment naming the sink or not at all.

## Backlog

- 02-php-migration.md's prose cells carry no ADR 0089 § 6 rule id, though its own *How to read a row*
  says they must; no pre-existing row does either, so either the requirement or ~200 rows are wrong.
- Goals 2-6 own the 758 names still open; `check-migration.py --report` and stage 7's comment header
  are the worklist (docs/agent/loop-goal.toml).
- `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale — the route table is built.
- A `bytes` array key ICEs in `nvs-ir` (docs/agent/playbook.md, *Writing a test case*).
- `catch (Core\Error $e)` panics rather than diagnosing (docs/agent/playbook.md, same section).
- M4's residue is the 1000-case corpus count (docs/implementation-plan.md, *Open now*).
