---
looper: 1
title: "each time class builds its own values, and Core\\Time is the clock"
summary: "Reached when Instant, Date, TimeOfDay and DateTime each build themselves (Instant::fromEpoch and fromIso, Date::fromIso, TimeOfDay::fromIso, DateTime::at(Date, TimeOfDay, Zone), DateTime::fromIso(text, Zone) and DateTime::parse), Core\\Time keeps only now, monotonic and sleep, Date::atStartOfDay is gone, a fromIso literal is checked while compiling, a zone written as text at a Zone parameter is checked while compiling and built once rather than per use, the editor completes every zone name, every old name is a compile error that names the new one with a fix, and every program, test, page and proof in the tree uses the new names."
context: [crates/nvs-stdlib/src/time.rs, crates/nvs-types/src/intrinsics.rs, crates/nvs-stdlib/src/moved.rs, crates/nvs-types/src/moved.rs]
stages:
  - id: 1
    title: the rule says which class builds which value, and Core\Time is the clock
    status: done
    context: [docs/rules/core-api/one-paradigm-per-operation.md, docs/rules/core-api/verb-lexicon.md, docs/rules/core-api/no-mode-strings.md]
  - id: 2
    title: each class builds itself, and a fromIso literal is checked while compiling
    status: done
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/cldr.rs, crates/nvs-stdlib/src/lib.rs]
  - id: 3
    title: a zone written as text is one shared value built while compiling, and the editor completes every zone name
    status: done
    context: [crates/nvs-ir/src/ir.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-runtime/src/object.rs, crates/nvs-lsp/src/completion.rs]
  - id: 4
    title: an old constructor name is a compile error that names the new one, with a fix
    status: done
    context: [tests/conformance/lang/a-moved-core-name-says-where-it-went.nvst, docs/rules/core-api/a-moved-name-says-where-it-went.md]
  - id: 5
    title: every program, test and tool outside the feature proofs uses the new names
    status: done
    context: [tools/nv/proofs/probe.ts, tools/nv/cmd/pdf-validate.ts]
  - id: 6
    title: the feature proofs of the moved members live under their new classes
    status: done
    context: [docs/examples/core/Time, tests/hostile/core/Time, benches/members/core/Time]
  - id: 7
    title: the three fromIso members and the written zone have their proofs, and the reference and website say what each class builds
    status: done
    context: [docs/reference/core/Time.md, docs/reference/core/Time-DateTime.md, docs/agent/prose.md]
  - id: 8
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: constructors
    stage: 2
    name: "each time class builds its own values, a fromIso literal that is malformed or names no real date does not compile, and the same text read at run time throws a ParseError"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/time-each-class-builds-its-own-values.nvst, --case, tests/conformance/core/time-from-iso-reads-one-format-on-every-class.nvst, --case, tests/conformance/reject/a-malformed-iso-literal-is-a-compile-error.nvst]
    timeout: 1800
  - id: zones
    stage: 3
    name: "a zone written as text at a Zone parameter is checked while compiling and costs no allocation per use, and the editor offers every zone name there"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/time-a-written-zone-is-checked-and-shared.nvst, --case, tests/conformance/reject/an-unknown-written-zone-does-not-compile.nvst]
    timeout: 1800
  - id: zone-completion
    stage: 3
    name: "inside a string at a Zone parameter, the editor offers the zone regions and then the names under the one typed"
    run: [cargo, test, --test, completion_file_values, --, a_string_at_a_zone_parameter_offers_every_zone_name]
    timeout: 1800
    tests:
      - a_string_at_a_zone_parameter_offers_every_zone_name
  - id: moved
    stage: 4
    name: "every old Core\\Time constructor is a compile error whose help names the new member, and a renamed one carries a fix that writes it"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/lang/a-moved-core-name-says-where-it-went.nvst, --case, tests/conformance/lang/a-moved-core-member-says-where-it-went.nvst]
    timeout: 1800
---

# each time class builds its own values, and Core\Time is the clock

## Why

The person found the time API confusing. `Date`, `TimeOfDay`, `Duration` and `Zone` each build
themselves (`Date::at`, `TimeOfDay::at`, `Duration::seconds`, `Zone::of`). `Instant` and `DateTime`
have no static member at all: `Core\Time::now`, `fromEpoch` and `fromIso` build an `Instant`, and
`Core\Time::at` and `Core\Time::parse` build a `DateTime`. Nothing decided that split. The module grew
with four classes, and `Date` and `TimeOfDay` came later with their own `at`. It leaves one name with
two meanings: `Core\Time::at(2024, 3, 1, $zone)` is a `DateTime` and `Core\Time\Date::at(2024, 3, 1)`
is a `Date`.

`Core\Time::at`'s own shape is the second complaint. Year, month and day are positional, the zone sits
between them and the clock fields, and hour, minute, second and nanos ride in an options bag. That cut
is where `rule:core-api/options-bag` forces it, not where the meaning is.

The third is length. Building a `DateTime` from its parts is the right shape, but a date written by
hand needs a short form. PHP's `new DateTime('any string')` is short because it guesses, and Novis
declines the guess (`rule:core-api/no-mode-strings`, `errors/ambiguous-input-refused`). One fixed,
unambiguous text format, ISO 8601, is short and guesses nothing, and `Instant::fromIso` already reads it.

## Standing decisions

**The person's calls (2026-11-19).** Each value class builds itself. `Core\Time` keeps the clock alone.
`DateTime::at` takes its three parts, `DateTime::at(Date $date, TimeOfDay $time, Zone $zone)`, the
reverse of the readers `date()`, `timeOfDay()` and `zone()`. Every class gets `fromIso`, so a date
written by hand stays short. The whole set:

| Class | Static members |
|---|---|
| `Core\Time` | `now()`, `monotonic()`, `sleep(Duration)` |
| `Core\Time\Instant` | `fromEpoch(int, {nanos?})`, `fromIso(text)` |
| `Core\Time\Date` | `at(int, uint, uint)`, `fromIso(text)` |
| `Core\Time\TimeOfDay` | `at(uint, uint, {second?, nanos?})`, `fromIso(text)` |
| `Core\Time\DateTime` | `at(Date, TimeOfDay, Zone)`, `fromIso(text, Zone)`, `parse(text, pattern, Zone)` |

**`Date::atStartOfDay` is removed (my call; it was my own unconfirmed call in `date-parts`).**
`DateTime::at($date, TimeOfDay::at(0, 0), $zone)` gives the same value, because `at` resolves a skipped
local time forward as `atStartOfDay` does. Two ways to one value break
`rule:core-api/one-paradigm-per-operation`. `DateTime::fromIso('2024-03-01', $zone)` is the short form.

**`now` stays on `Core\Time` (my call).** It reads the outside world, like `monotonic` and `sleep`, and
the determinism rules fake the clock through it (`rule:testing/determinism-declared-on-the-test`).
`Core\Time` is then one idea: the clock.

**The `fromIso` grammar (my calls).** One format per class, the ISO 8601 extended form, nothing else:
- `Date::fromIso`: `YYYY-MM-DD`, with the year range `Date::at` takes. No week dates, no ordinal
  dates, no basic form such as `20240301`.
- `TimeOfDay::fromIso`: `HH:MM`, `HH:MM:SS` or `HH:MM:SS.f` with one to nine fraction digits. No
  `24:00` and no leap second `60`.
- `DateTime::fromIso(text, Zone)`: a date, or a date, a separator and a time. The separator is `T` or
  one space, as RFC 3339 allows. A date alone is the first instant of that day. A skipped or repeated
  local time resolves as `DateTime::at` resolves it. **An offset or a `Z` in the text throws:** the
  zone is the argument, and a text with an offset is `Instant::fromIso(...)->in($zone)`. The message
  says so.
- `Instant::fromIso` keeps its grammar, and accepts the same two separators.
- A bad text throws `ParseError` with the part that is wrong, as `Instant::fromIso` does now.

**A literal is checked while compiling (my call).** A string literal at the text argument of every
`fromIso`, `Instant`'s included, is a `rule:expressions/intrinsic-constant-arguments` intrinsic: a new
`Grammar::Iso` row per class in `crates/nvs-types/src/intrinsics.rs`. `'2024-02-30'` is a compile error
at the literal. The run-time path is the same parser (`rule:expressions/preparation-preserves-behaviour`).
`DateTime::parse` keeps its prepared-pattern row; only its owner changes.

**A zone is written as text where it is fixed (the person's call).** A string literal at a parameter
typed `Core\Time\Zone` is that zone: `DateTime::fromIso('2024-03-01 09:30', 'Europe/Berlin')`,
`$instant->in('UTC')`. It is the `30s` pattern: the short form for a value fixed in the source, and the
member (`Zone::of($text)`) for text that arrives at run time, which is the one place that throws. The
checker reads the literal as it reads a key literal at a `property<C>` parameter
(`rule:types/property-key`): an unknown name, or an offset `Zone::fixed` refuses, is a compile error at
the literal. Only a literal: a `string` variable at a `Zone` parameter stays a type error, so tainted
text never becomes a zone without `Zone::of`. `Zone::of('UTC')` written with a literal still compiles,
as `Duration::parse('30s')` does, and the docs show the short form.

**A written zone is built once, never per use (the person's call; how is mine).** "UTC is always UTC":
a written zone is one immortal instance in the compiled unit's data section, the mechanism a written
duration (`InstKind::ConstDuration`, `crates/nvs-ir/src/ir.rs:1094`) and a fixed `` html`…` `` already
use (`crates/nvs-runtime/src/object.rs:259` § *an immortal instance is on no list at all*). Its one slot,
the zone id, is an immortal string literal. Every request and every core shares it, and nothing writes
it: `Zone` has no member that writes a slot, which is that section's soundness condition. The same fold
covers `Zone::UTC` (a `Const::Built` call per evaluation today, `crates/nvs-stdlib/src/time.rs:1084`),
and `Zone::of` and `Zone::fixed` called with a literal. **What it spends:** a header, one slot and the
id's bytes once per written zone, charged to the compiled unit and freed with it. Per use: nothing,
where `Zone::of` costs 3 allocations and about 150 bytes per call (perf ledger). Each written zone is
its own instance, as each written duration is, so `==` stays identity
(`rule:expressions/object-identity-equality`).

**The name lookup per conversion is measured before it is changed (my call).** Every conversion
resolves the zone's id again (`resolve_zone`, `crates/nvs-stdlib/src/time.rs:3902`, through
`TimeZone::get`). Stage 3 counts what one `->in($zone)` spends on that lookup. Only if it shows does a
cache from id to `TimeZone` land, built while compiling for a written zone, so nothing on the request
path is made on first use. **Measured (stage 3): it does not show, so no cache lands.** One
`TimeZone::get` of a cached name is about 21 ns and allocates nothing (jiff 0.2.38, release, a
scratch bench of the lookup alone against `Zoned::new` plus one civil read, 12 ns). The `in` bench
makes two lookups per operation, about 42 ns, against a recorded 1,823 ns per operation: about 2%.

**The editor completes every zone name (the person's call).** Inside a string literal at a `Zone`
parameter, `Zone::of`'s included, completion offers the names from `jiff::tz::db().available()`, the
same database the checker and `Zone::of` resolve with. They are offered by segment, with `/` as the
separator, through the existing `file_values` (`crates/nvs-lsp/src/completion.rs:1042`): `Europe/` first,
then the cities under it. `UTC` is offered too. Hover on a written zone shows its current offset. The
database is the one on the machine that compiles, and Novis compiles where it runs, so a name the
editor offered and the server's database lacks is a compile error at start, never a failed request.

**A written zone name is looked up as `Zone::of` looks it up (my call, stage 5).** `written_zone`
asks the same `TimeZone::get`, which ignores case, and keeps the text as written, as `Zone::of` does.
Stage 3 matched the sorted name list case included, so `'europe/berlin'` did not compile while
`Zone::of` resolved it: a refusal the run time does not make, which
`rule:expressions/preparation-preserves-behaviour` forbids. Completion still offers the database's
own spelling.

**`TimeOfDay::at` keeps its bag (my call).** Its bag holds only the finer parts of the same value,
which is not the cut the person found arbitrary in `Core\Time::at`.

**A moved member is a compile error that names its new place (my call).** The move table
(`rule:core-api/a-moved-name-says-where-it-went`) covers types today. It gains member rows. A pure
rename (`fromEpoch`, `fromIso`, `parse`) carries the fix that writes the new name in full. `at` and
`atStartOfDay` change their arguments, so their rows give the help line alone, written as code:
`Core\Time\DateTime::at(Core\Time\Date::at(…), Core\Time\TimeOfDay::at(…), $zone)`. No alias keeps an
old name working (`rule:core-api/no-deprecation-window-before-a-release`).

**Rewriting callers (my call).** Stage 4 lands first, so stage 5 and 6 run `nvs check --fix` for the
three renames over each tree, and rewrite `at` and `atStartOfDay` calls by hand. A `Core\Time::at`
call whose arguments are all literals becomes `DateTime::fromIso('2024-03-01 09:30', $zone)`; one with
computed fields becomes `DateTime::at(Date::at(…), TimeOfDay::at(…), $zone)`. A `Zone::of('…')` or
`Zone::UTC` written as an argument becomes the string literal. A conformance case that
uses a constructor as an independent oracle keeps the `at` form, as `date-parts` kept `format`.

**What it costs.** `DateTime::at` from parts builds a `Date` and a `TimeOfDay` first: two more small
objects per call than `Core\Time::at`, whose ledger row is 4 allocations and 210 bytes per call. Stage 7
records the new rows and states the difference in `DateTime::at`'s card and the rule. A `fromIso`
literal is checked once while compiling, and reading it at run time is a fixed-format scan, cheaper
than a CLDR pattern. A written zone removes `Zone::of`'s per-call object, so a call written with
literals, `DateTime::fromIso('…', 'UTC')`, spends one `DateTime` and nothing else.

**Where it is written down.** A new fragment
`docs/rules/core-classes/a-time-class-builds-its-own-values.md`, with its JSON under
`data/rules/core-classes/`, holding the table above, the `fromIso` grammar, the written zone and the
Declined list: a free-form date text in PHP's manner, `Core\Time` as the home of constructors, an
instance `Date->at`, the positional seven-field `DateTime::at`, keeping `atStartOfDay`, a `string` zone
parameter in place of `Zone` (every member would check the name again and gain an error path, and
tainted text would reach it), and one shared instance per zone name across written zones (it would make
`==` true for two written zones and false for `Zone::of`'s, for no saving a request sees). The module doc of
`crates/nvs-stdlib/src/time.rs` is rewritten to cite it, and its *atStartOfDay* decision goes.

**No website redirects.** The old member pages are gone with the members, as for every Pre-Alpha removal.

**Lanes.** Stages 1 and 2 run in order: they share `time.rs`. Stage 3 (the checker, the IR, codegen
and the editor) and stage 4 (the move table) touch disjoint files and may run side by side after
stage 2. Stages 5 and 6 touch disjoint trees and may run side by side after stages 3 and 4. Stage 7
needs stage 6's moved proof directories.

**What stops the goal.** One attempt per item, two sessions per stage. An item still red after its
attempt goes into the handoff with its cause, and the stage moves on to its other items. A stage still
open after two sessions stops the loop with `BLOCKED` naming the item.

## Stages

### 1. the rule says which class builds which value, and Core\Time is the clock

**Does:** writes the rule and the module doc. No code changes.

- [x] `docs/rules/core-classes/a-time-class-builds-its-own-values.md:1`: the new rule with its JSON
  under `data/rules/core-classes/`, written from § *Standing decisions*: the table, `now` on the clock,
  the `fromIso` grammar, the compile-time check, the cost, and the Declined list.
  `docs/rules/core-api/no-mode-strings.md:15` and `docs/rules/core-api/one-paradigm-per-operation.md:1`
  cite it. Then `bun nv rules --render`, and `bun nv rules --check` is green.
- [x] `crates/nvs-stdlib/src/time.rs:1`: the module doc names the new home of each constructor, cites
  the rule, and drops the *atStartOfDay* decision at `:76`.

### 2. each class builds itself, and a fromIso literal is checked while compiling

**Does:** the keystone. The registry moves the constructors, adds the three `fromIso` members, changes
`DateTime::at`, and removes `atStartOfDay`. Every card here is end-user text (`docs/agent/prose.md`).

- [x] `crates/nvs-stdlib/src/time.rs:2307` `TIME`: keeps `now`, `monotonic` and `sleep`. `fromEpoch`
  and `fromIso` move to `INSTANT`'s `methods` (`:1202`), and `parse` to `DATETIME`'s (`:1659`), with
  their symbols and cards. `TIME_CARD` (`:2418`) says `Core\Time` reads the clock and waits.
- [x] `DATETIME`'s `at` takes `Date`, `TimeOfDay` and `Zone`. `nvs_core_time_at` (`:2409`, `:2616`) reads
  the three instances' slots and keeps its gap and range handling. Its card states what it spends.
- [x] `Date::fromIso`, `TimeOfDay::fromIso` and `DateTime::fromIso` with their symbols and cards. One
  parser per class serves the run-time call and the intrinsic. `Instant::fromIso` accepts the space
  separator too.
- [x] `atStartOfDay` (`:2786`) and its card (`:3004`) are deleted.
- [x] `crates/nvs-types/src/intrinsics.rs:354`: the `parse` row's owner is `Core\Time\DateTime`, and one
  `Grammar::Iso` row per `fromIso` checks a literal with the same parser. `crates/nvs-stdlib/src/registry.rs:3985`
  `PREPARED_MEMBERS` names `DATETIME_NAME`'s `parse`. The messages at `crates/nvs-stdlib/src/lib.rs:948`
  and `crates/nvs-stdlib/src/cldr.rs:953`, `:1249` name `Core\Time\DateTime::parse`.
- [x] The Rust tests that write the old names are updated: the unit tests in `time.rs`,
  `crates/nvs-stdlib/src/{uri,test,json,format,lib}.rs`, `crates/nvs-stdlib/tests/corpus/mod.rs`,
  `crates/nvs-types/tests/intrinsics.rs`, `crates/nvs-ir/tests/prepared_patterns.rs` and
  `crates/nvs-cli/tests/spreadsheet_reader.rs`.
- [x] `tests/conformance/core/time-each-class-builds-its-own-values.nvst:1`: every constructor in the
  table, each printed, and `DateTime::at` agreeing with `date()`, `timeOfDay()` and `zone()`.
- [x] `tests/conformance/core/time-from-iso-reads-one-format-on-every-class.nvst:1`: each accepted form,
  a date alone as the start of a day with a skipped midnight, a text with an offset at
  `DateTime::fromIso` throwing `ParseError`, and run-time texts that throw.
- [x] `tests/conformance/reject/a-malformed-iso-literal-is-a-compile-error.nvst:1`: `'2024-02-30'`,
  `'24:00'`, `'2024-3-1'`, `'20240301'` and an offset at `DateTime::fromIso`, each one diagnostic.
- [x] The `constructors` check runs once.

### 3. a zone written as text is one shared value built while compiling, and the editor completes every zone name

**Does:** a string literal at a `Zone` parameter is checked and folded to one immortal `Zone`, the
literal forms of `Zone::UTC`, `Zone::of` and `Zone::fixed` fold the same way, and the editor offers the
zone names. Read `crates/nvs-runtime/src/object.rs:259` § *an immortal instance is on no list at all*
before the first edit.

**Split:** one writing subagent takes the editor items (`crates/nvs-lsp/` alone) while the session does
the checker, IR and codegen. Their files are disjoint, and no one builds until both end.

- [x] The checker: a string literal at a parameter typed `Core\Time\Zone` checks against
  `resolve_zone` (`crates/nvs-stdlib/src/time.rs:3902`) and is typed `Zone`. An unknown name is one
  diagnostic at the literal, whose help names the nearest zone ids. Find the key-literal path to copy
  with `bun nv peek --locate key_parameter_class` (`crates/nvs-types/src/expr/args.rs:158`). A `string`
  variable at a `Zone` parameter stays the type error it is.
- [x] `crates/nvs-ir/src/ir.rs:1094`: `InstKind::ConstZone(String)` beside `ConstDuration`, in every match
  that lists `ConstDuration` (`:2587`, `:4059`, `:4361`, `crates/nvs-ir/src/print.rs:203`), lowered beside
  `crates/nvs-ir/src/lower/expr.rs:3306`. A written zone, `Zone::UTC`, and `Zone::of`/`Zone::fixed` with a
  literal all lower to it, with the id `resolve_zone` reads (`fixed`'s canonical `±HH:MM[:SS]`).
- [x] `crates/nvs-codegen/src/emit.rs:853`: `ConstZone` emits one immortal instance with
  `immortal_object_bytes`, its slot an immortal string. `crates/nvs-codegen/tests/core_members.rs:70`
  gains a test that a written zone emits no call and no allocation.
- [x] Measure the lookup: the counts `bun nv proofs --record-perf --group 'Core\Time\Instant'` gives
  for `in`, against the same member with the lookup stubbed in a scratch build under `.agent-tmp/`.
  Record the figure in the handoff. Only if the lookup is a visible share of `in`: an id-to-`TimeZone`
  table filled while compiling for written zones, and the figure goes into the rule.
- [x] `tests/conformance/core/time-a-written-zone-is-checked-and-shared.nvst:1`: a written zone at
  `DateTime::at`, `fromIso`, `parse` and `Instant->in`, `Zone::UTC`, `Zone::of('Europe/Berlin')`, and a
  fixed offset, each printed. A loop of many conversions with a written zone, whose memory reading shows
  no growth per iteration.
- [x] `tests/conformance/reject/an-unknown-written-zone-does-not-compile.nvst:1`: `'Europe/Berln'`,
  `'utc '`, `'+26:00'` and a `string` variable at a `Zone` parameter, each one diagnostic. (`+25:00`
  is a valid offset: `Zone::fixed` takes up to 25:59:59, so the case writes `+26:00`.)
- [x] `crates/nvs-lsp/src/completion.rs:1042`: inside a string at a `Zone` parameter, `file_values` gets
  every id from `jiff::tz::db().available()` with `/` as the separator, built once per server and kept.
  Hover on a written zone shows its id and current offset.
- [x] `crates/nvs-lsp/tests/completion_file_values.rs:1`: the test
  `a_string_at_a_zone_parameter_offers_every_zone_name`: `'Eu` offers `Europe/`, `'Europe/Be` offers
  `Europe/Berlin`, and `Zone::of('` offers the same set.
- [x] The `zones` and `zone-completion` checks run once.

### 4. an old constructor name is a compile error that names the new one, with a fix

**Does:** extends the move table to static members, so stage 5 and 6 can let `nvs check --fix` do
the renames.

- [x] `crates/nvs-stdlib/src/moved.rs:28` `MOVES`: rows for `Core\Time::fromEpoch`, `fromIso`, `parse`,
  `at` and `Core\Time\Date::atStartOfDay`. A row says whether it carries a fix. The test at `:45` holds a
  member row to the registry: the new member exists and the old one does not.
- [x] `crates/nvs-types/src/moved.rs:34` `moved_subject`: also matches the unknown-member diagnostic on a
  `Core` class (`E0405`), for a static call and for `use Core\Time;` then `Time::fromIso`. The fix writes
  the full new name, as `written_fix` (`:99`) does for a type.
- [x] `docs/rules/core-api/a-moved-name-says-where-it-went.md:1`: says a member moves too, and a member
  whose arguments changed gets the help line alone.
- [x] `tests/conformance/lang/a-moved-core-member-says-where-it-went.nvst:1`: each of the five old names,
  with its help line, and the fix applied for the three renames. The `moved` check runs once.

### 5. every program, test and tool outside the feature proofs uses the new names

**Does:** rewrites every caller outside `docs/examples/`, `tests/hostile/` and `benches/members/`.
Run `nvs check --fix` per tree for the renames, then rewrite each `at` call as § *Standing decisions*
says. The set: an `Explore` subagent lists every file that matches
`Time::(at|parse|fromIso|fromEpoch)\b|atStartOfDay` outside those three trees and `target/`.

**Split:** writing subagents with disjoint files, side by side, none of them building: one for
`tests/conformance/` (about 60 cases), one for `apps/`, `packages/`, `extensions/` and `examples/`,
and one for `tools/nv/proofs/probe.ts:112`, `tools/nv/cmd/pdf-validate.ts:54`, the doc comments that
name `Core\Time::parse` (`crates/nvs-stdlib/src/cldr.rs`, `crates/nvs-runtime/src/throwable.rs:353`,
`crates/nvs-types/src/intrinsics.rs`) and the three playbook files under `docs/agent/playbook/`.
The session builds once they all end.

- [x] The list exists, and each subagent has its part of it.
- [x] `tests/conformance/` uses the new names. Each case whose expected output changed is checked by
  reading its diff, not by regenerating it.
- [x] `apps/`, `packages/`, `extensions/` and `examples/` use the new names, and `cargo build` is green.
- [x] The tools, doc comments and playbook files use the new names.
- [x] The `Explore` search, run again, finds nothing outside the three proof trees.

### 6. the feature proofs of the moved members live under their new classes

**Does:** moves the proof directories with `git mv`, deletes `atStartOfDay`'s, and rewrites every other
proof file that calls an old name. Every comment and `about.md` touched is end-user text.

**Split:** three writing subagents side by side, one per tree, none of them building:
`docs/examples/`, `tests/hostile/` and `benches/members/`. Each moves its tree's directories first, then
runs `nvs check --fix`, then rewrites the `at` calls.

- [x] The moves, in each of the three trees: `core/Time/at` to `core/Time-DateTime/at`, `core/Time/parse`
  to `core/Time-DateTime/parse`, `core/Time/fromIso` to `core/Time-Instant/fromIso`, `core/Time/fromEpoch`
  to `core/Time-Instant/fromEpoch` (a bench is one file, `benches/members/core/Time/at.nvs` and its
  `.scale.nvs` siblings). `docs/examples/core/Time-Date/atStartOfDay/` and its hostile and bench files are
  deleted. Each moved `about.md` describes the new signature.
- [x] `docs/examples/` uses the new names (about 190 files), and each `.out` still matches.
- [x] `tests/hostile/` uses the new names (about 60 files).
- [x] `benches/members/` uses the new names (about 50 files).
- [x] `bun nv proofs --id 'Core\Time\DateTime::at'`, `'Core\Time\DateTime::parse'`,
  `'Core\Time\Instant::fromIso'` and `'Core\Time\Instant::fromEpoch'` each report the moved proofs.

### 7. the three fromIso members and the written zone have their proofs, and the reference and website say what each class builds

**Does:** the new feature proofs and the user pages. Every line here is end-user text, held to
`AGENTS.md` § *Text an end user reads*. Run `bun nv proofs --id` for each of the three first and last.

**Split:** three writing subagents side by side, one per new member, each writing its `about.md`, three
examples, one bench and one attack. The session writes the reference pages meanwhile.

- [x] `Core\Time\Date::fromIso`: `docs/examples/core/Time-Date/fromIso/about.md:1`, three examples, a
  bench, an attack. Its tests are stage 2's two cases.
- [x] `Core\Time\TimeOfDay::fromIso`: the same under `Time-TimeOfDay/fromIso/`.
- [x] `Core\Time\DateTime::fromIso`: the same under `Time-DateTime/fromIso/`. One example shows a date
  alone as the start of the day, and the attack writes offsets and far years.
- [x] The written zone: `docs/examples/core/Time-Zone/of/about.md` says a fixed zone is written as text
  and `Zone::of` is for text read at run time. One example there writes `'Europe/Berlin'` at
  `DateTime::fromIso`. `tests/hostile/core/Time-Zone/of/03-zone-names-that-only-look-real.nvs:1`: an attack
  whose written names are misspelled, padded or past the offset range. None of them compiles.
- [x] `docs/reference/core/Time.md:1`: `Core\Time` is the clock; its code block uses only `now`,
  `monotonic` and `sleep`, and a sentence points to each class's constructors.
  `docs/reference/core/Time-DateTime.md:1`, `Time-Instant.md:1`, `Time-Date.md:1` and
  `Time-TimeOfDay.md:1` show their own constructors, `fromIso` first.
- [x] `website/src/data/questions.json` uses the new names, and `bun nv render --website` writes
  `core.json` and `llms-full.txt` again.
- [x] `bun nv proofs --record-perf --group 'Core\Time\DateTime'`, and the same for `Date`, `TimeOfDay` and
  `Instant`. `DateTime::at`'s card and the rule state its allocations and bytes beside the old
  `Core\Time::at` row's 4 and 210.
- [x] `bun nv proofs --comments` passes on every new file, and `bun nv owners --closes time-constructors`
  names no gap.

### 8. the goal is verified once, and every failure is fixed

- [x] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [x] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Every stage is done. Stage 8 closed the goal without the full gate: `cargo build` and `cargo clippy
--all-targets -- -D warnings` are green, and the four acceptance checks pass by name (`constructors`,
`zones`, `zone-completion`, `moved`). `bun nv owners --closes time-constructors` names no gap. The one
decision only this file held, that no cache from a zone id to its `TimeZone` lands because the lookup
is about 2% of a conversion, is now a Declined bullet of
`rule:core-classes/a-time-class-builds-its-own-values`. The chapter and `ground-rules.md` are rendered
again, so the rule no longer reads *designed* there.

The nightly the sweep named (`44510687786`) ran at `adc7487822`, before the Miri bound fix
(`9478fd2f66`) and the rest of this goal, so this stage changed nothing for it.

**Expected red:** none known. This goal joins the chain's `ungated:` list, and the next full gate runs
`bun nv affected --run` over it.

### Next

Nothing. The driver retires this file on `DONE`.

### Backlog
