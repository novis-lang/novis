---
milestone: M9
position: last
---
# Loop goal 197 — `Novis\Intl` is built into every binary: ICU4X with every CLDR locale, batch-shaped, over `Core\Time` values

The second component ADR 0247 builds into every `nvs` binary. It is ICU4X with its CLDR data compiled
in, one Rust crate under `extensions/intl/`, built, packed and embedded by the same build script goal
`ext-image` wrote, and loaded beside the image component with no entry and no pin.

```nvs
use Novis\Intl\{Collator, NumberFormat, Locale};        // names are the record's; these are a sketch

$sorted = Collator::sort($names, "sv");                  // 10,000 strings: one call into the component
$prices = NumberFormat::currency([19.9, 1250.0], "EUR", "de-AT");
$lang   = Locale::negotiate($request->header("Accept-Language"), ["en", "de", "fr"], "en");
```

Its scope is ADR 0247 § 5's list and nothing more: collation (sort keys and whole-array sort), numbers
(decimal, percent, currency, compact), dates and times over `Core\Time` values, plural rules, relative
time, list formatting, word and sentence segmentation, and locale negotiation from an
`Accept-Language` value. Message formatting is not in it. Its API is designed by this goal's own
record, batch-shaped as ADR 0051 § 3 requires: an entry point takes a list and returns a list, so a
page of numbers or a table of names costs one crossing.

## Why here

It is the last of the M9 goals because it stands on all of them: the world, `nvs-ext`, the compiler's
registration layer and source section, the tooling, and the build, embedding, crossing counter and
`novis/` proofs paths goal `ext-image` built for the first component. The second component reuses
every one of those, so what is new here is the API record, the ICU4X crate, and the two measurements
ADR 0247 owes: the binary's growth, and a bundle that calls both components.

It is also what reverses ADR 0051's trade: "locale-correct formatting requires installing something"
stops being true when this goal's checks are green
(`rule:packaging/the-first-party-components-are-built-in`).

It carries `position: last` because every M9 goal sits behind the pinned closing goals.

## Stage 0 — the catch-up

The sentences on disk this goal makes untrue, each rewritten whole by the session that lands the
behaviour and not before:

- `docs/rules/packaging/the-first-party-components-are-built-in.md`'s **Not on disk** paragraph,
  which goal `ext-image` left naming the intl half. It is deleted when Stage 7's checks are green,
  and the fragment's "What it spends" sentence gains the measured size.
- `crates/nvs-stdlib/src/cldr.rs`'s module doc says "a full CLDR implementation is `icu`, which is a
  locale-data dependency an order of magnitude larger than everything else in `Core`" and that
  localization "is an application concern rather than a Tier 0 one". Both stay true of `Core`; the doc
  gains one sentence naming `Novis\Intl` as where locale-aware formatting lives.

The search that closes the stage:
`grep -rn -i "intl\|locale-correct\|installing something" docs/rules docs/reference crates/nvs-stdlib/src/cldr.rs`,
read line by line. Every hit is true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated
and are regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. After goal `goal-closeout` that is nothing: the
suites, the `.nvst` trees and `nv verify` are the safety net. Every check goals `ext-image` and
`ext-image-analysis` turned green stays green, and `Core\Cldr::pluralCategory` answers exactly what
it answers today.

## Stage 2 — the record and `wit/intl.wit`

**Does:** Writes the intl API record, batch-shaped and covering exactly ADR 0247 § 5's list, and the
WIT interface the component exports.

One file set: `docs/decisions/` (the record), `docs/rules/core-classes/` (the fragments it creates),
`wit/intl.wit` (new), `crates/nvs-stdlib/tests/ext_world.rs` (goal `ext-design`'s test of the
world).

- **The record**, this goal's one slot, written first, from § *Standing decisions*. It names every
  class and member under `Novis\Intl`, each options shape (`rule:core-api/one-paradigm-per-operation`:
  one class per job, no procedural twin), each export in `wit/intl.wit`, and what a malformed and an
  unsupported locale do. It creates the rule fragments the API needs under
  `docs/rules/core-classes/` (at least one stating the batch shape and the roster), and states the
  tradeoffs. Its last item adds a `bun nv proofs --verify --group` check for each class it names to
  this goal's Stage 8 in `data/goals/ext-intl.json`: a check is added, never removed.
- **`wit/intl.wit`**, against `nvs:ext@1.0.0` and in ADR 0246 § 1's table only. Every formatting
  export takes a `list` of values and one options record and returns a `list`. A date crosses as the
  record `types.wit` holds for its `Core\Time` class (goal `ext-design` walked them).
- **The test** in `crates/nvs-stdlib/tests/ext_world.rs` parses `wit/intl.wit` against the world and
  fails, naming the export, on one that takes a single value where a list would do. Locale
  negotiation, whose input is one header, is the named exception in the test.
- **Pinned by** the Stage 2 check.

## Stage 3 — collation, and the component is built in

**Does:** Builds `extensions/intl` into every binary beside the image component, and adds sort keys
and whole-array sort, where sorting 10,000 strings is one crossing.

Two file sets, in this order. The crate and its embedding: `extensions/intl/Cargo.toml`,
`extensions/intl/src/lib.rs` (new), `crates/nvs-ext/build.rs`, `crates/nvs-ext/src/builtin.rs` (goal
`ext-image`'s). The job: `extensions/intl/src/collation.rs` (new), `extensions/intl/nvs/` (new),
`crates/nvs-ext/tests/intl.rs` (new).

- **The crate** (`rule:packaging/the-first-party-components-are-built-in`) under `extensions/`,
  outside the workspace, built for `wasm32-wasip2` by the same build script and embedded in the same
  built-in set, its digest in `env_hash` the way the image component's is. Compiled on first use.
- **Collation** (`rule:packaging/the-boundary-is-the-cost`, ADR 0051 § 3): `icu_collator`. Whole-array
  sort of a list of strings and sort keys for a list of strings, each one call, with the strength,
  case order and numeric options the record names.
- **One crossing, counted.** The test sorts 10,000 strings and reads nvs-ext's per-instance call
  counter: one host-to-guest call.
- **The order is the locale's.** In Swedish `ä` sorts after `z`; in German it sorts with `a`.
- **Pinned by** the Stage 3 checks, the first of which is ADR 0247 *Verification*'s "`Novis\Intl`
  answers with no `nvs.toml`".

## Stage 4 — numbers and plural rules

**Does:** Adds decimal, percent, currency and compact number formatting and the plural and ordinal
categories, each over a list in one call.

One file set: `extensions/intl/src/number.rs`, `extensions/intl/src/plural.rs` (new),
`extensions/intl/nvs/`, `crates/nvs-ext/tests/intl.rs`, and `crates/nvs-stdlib/src/cldr.rs:1625`
(`Core\Cldr`'s class, read only).

- **Numbers**: decimal and percent (`icu_decimal`), currency and compact (where ICU4X's pinned
  version has them, see § *Standing decisions*), with the grouping, the fraction digits and the
  currency display the record names. A batch of numbers is one call.
- **Plural rules** (`icu_plurals`): the cardinal and ordinal category of each number in a list.
- **Agreement with `Core\Cldr`.** `Core\Cldr::pluralCategory` and `ordinalCategory` stay in Tier 0
  with their closed roster (`rule:programs/framework-core-half`). A test asks both for every
  language on that roster over a sweep of numbers and fails, naming the language and the number,
  where they differ.
- **Pinned by** the Stage 4 checks.

## Stage 5 — dates and times, relative time, lists

**Does:** Formats `Core\Time` values in a locale and a zone, formats relative times, and joins lists
with the locale's words, each over a list in one call.

One file set: `extensions/intl/src/date.rs`, `extensions/intl/src/relative.rs`,
`extensions/intl/src/list.rs` (new), `extensions/intl/nvs/`, `crates/nvs-ext/tests/intl.rs`.

- **Dates and times** (`icu_datetime`) over `Core\Time\Instant` with a `Core\Time\Zone`, and over
  `Core\Time\DateTime`, `Date` and `TimeOfDay`, in the lengths the record names. The host converts an
  instant in its zone to local fields and an offset with `Core\Time` before the call, so the guest
  carries no time-zone database; the guest formats the fields and names the zone from its id.
- **Relative time** (`icu_experimental`'s relative-time formatter, see § *Standing decisions*) from a
  `Core\Time\Duration` or a count and a unit, past and future.
- **Lists** (`icu_list`): "and", "or" and unit lists in the locale's words.
- **Pinned by** the Stage 5 checks.

## Stage 6 — segmentation and locale negotiation

**Does:** Splits strings into words and sentences by the locale's rules, and picks the best available
locale from an `Accept-Language` value.

One file set: `extensions/intl/src/segment.rs`, `extensions/intl/src/negotiate.rs` (new),
`extensions/intl/nvs/`, `crates/nvs-ext/tests/intl.rs`.

- **Segmentation** (`icu_segmenter`): word and sentence segments of each string in a list. Grapheme
  segmentation stays in `Core\Str` (ADR 0051 § 3) and is not repeated here.
- **Negotiation** (`icu_locale`): an `Accept-Language` value, the locales the application offers,
  and a default. The header is parsed by its quality values, each range is matched with CLDR's
  fallback (`de-AT` reaches `de`), and the best offered locale is returned. An empty or malformed
  header returns the default; it never throws, because the header is request data the program does
  not control.
- **Pinned by** the Stage 6 checks.

## Stage 7 — the growth, and a bundle

**Does:** Measures what the two components add to the binary and records it in the component's module
doc, and proves a bundle built with `nvs build --compile` calls both.

One file set: `extensions/intl/src/lib.rs` (its module doc), `extensions/image/src/lib.rs` (its
module doc), `crates/nvs-ext/tests/builtin.rs`, `crates/nvs-cli/tests/bundle.rs:43` (`bundle_of`).

- **The growth** (ADR 0247 *Consequences*). Each component's module doc states the size of its
  embedded `.nvsx`, in MiB to one decimal, measured from a release build. A test reads both docs
  and fails when a figure is more than 5% from the embedded bytes, so the figure is one the code
  enforces. `rule:packaging/the-first-party-components-are-built-in`'s "What it spends" sentence names
  the total.
- **A bundle calls both.** A test in `crates/nvs-cli/tests/bundle.rs` builds an entry with
  `nvs build --compile` (`bundle_of` at line 43), runs it from a folder with no `nvs.toml`, and reads
  one answer from `Novis\Image` and one from `Novis\Intl`.
- **If the growth looks too large**, ADR 0247's *Revisiting* says the answer is a slimmer locale
  set, never an optional component. That is the user's call: the session records the figure and puts
  the question in the handoff's `## Backlog`.

## Stage 8 — the feature proofs

**Does:** Writes the feature proofs for every `Novis\Intl` member the record named.

One file set: `docs/examples/novis/`, `tests/hostile/novis/`, `benches/members/novis/`,
`docs/reference/novis/Intl.md` (new), `data/proofs/policy.json`.

- **What each member owes** (`rule:testing/feature-proofs`): `about.md`, tests from Novis and Rust,
  three examples, one bench, one attack and its help in the binary. Goal `ext-image` taught the
  roster to see `Novis\`, so `bun nv proofs --id '<member>'` prints what is owed.
- **The manifest class** follows goal `ext-image`'s rule for `Novis\Image\Codec`: tests, attack and
  help in full; examples and bench in `data/proofs/policy.json`'s `skip`, one reason each.
- **The attacks**: a list of a million strings to sort under a small memory cap, a locale tag of
  10,000 characters, an `Accept-Language` header with a thousand ranges, a number past `f64`'s
  range, a string of combining marks to segment.
- **The reference page** `docs/reference/novis/Intl.md`, in the shape of `Image.md`.
- **Pinned by** the Stage 8 checks, and by the per-class checks Stage 2's record added.

## Standing decisions

- **The user's calls, as instructions.** Intl covers the web-app set of ADR 0247 § 5; MessageFormat
  is out. Values cross as typed WIT values by ADR 0246 § 1's table, and the `value` handle only for
  `mixed`. A guest links WASI with an empty context and may be granted files and outbound HTTP and
  nothing else; the intl component is granted nothing. Grants are the intersection of the entry's,
  the manifest's and the caller's. A guest call runs on its request's core as a wasmtime async call
  polled by the coroutine, yielding at every epoch tick, with no compute pool. The Novis source half
  travels inside the `.nvsx`. A trap throws `ExtensionError`; a CPU or memory limit is a
  resource-limit `FATAL`. `Novis\Image` and `Novis\Intl` are built into every binary and always on:
  nothing turns them off. The component crates are compiled for `wasm32-wasip2`. No signatures in M9.
- **The record writer's calls, not confirmed by the user, also standing.** `Novis\` is reserved; the
  error variant `invalid|parse|runtime` throws `LogicError|ParseError|RuntimeError`; one instance per
  extension per request, a second task waits; `nvs check` and the LSP read manifests and never
  instantiate; a bundle embeds the `.nvsx` files its configuration lists, and the built-in components
  are in every bundle because they are in the binary.
- **One record slot**: one new record and no other number, checked against `docs/decisions/` right
  before it is written, because another agent may take a number first. It is the intl API's first
  slice, batch-shaped, covering exactly ADR 0247 § 5's list, with `Core\Time` values for dates.
- **The crates are ICU4X's**, at one pinned version: the `icu` crates with `compiled_data` —
  `icu_collator`, `icu_decimal`, `icu_plurals`, `icu_datetime`, `icu_list`, `icu_segmenter`,
  `icu_locale` — and `icu_experimental` for what ICU4X has not stabilised. As the goal was written,
  percent, currency, compact and relative-time formatting were in `icu_experimental`; that is not
  checked against the version the session pins. `icu_experimental` is pinned with `=` and its use is
  confined to the component, so an upstream change never reaches a program
  (`rule:packaging/a-dependency-break-is-absorbed-never-forwarded`). ICU4X is `Unicode-3.0`, already on
  `deny.toml`'s allowlist.
- **Every CLDR locale** means the data `compiled_data` bakes in. Whether that is CLDR's whole set or
  ICU4X's default coverage level is not checked; the record states which, and the measured size.
- **A locale is always an argument** (`rule:core-api/no-ambient-state`,
  `rule:security/no-cross-request-state`): no default locale, no setting that holds one, nothing
  process-wide. An `[intl]` block is added to `crates/nvs-config/src/default.toml` only if the record
  argues a key that is not a locale, and none is expected.
- **A malformed locale tag** is a `LogicError` naming it. A well-formed tag ICU4X has no data for
  falls back along CLDR's chain, and the record says how a program learns which locale answered.
- **The host converts time, the guest formats it.** `Core\Time` owns the time-zone database; the guest
  receives local fields, an offset and a zone id. The goal writer's call, unconfirmed: it keeps a
  second tz database out of the binary.
- **`Core\Cldr` stays.** Its two members are Tier 0 and answer on their closed roster; `Novis\Intl`'s
  plural rules agree with them there, and the test in Stage 4 holds that. Neither is deprecated.
- **The tradeoffs**, stated here and in the record because AGENTS.md asks. Performance: one crossing
  per batch, so a sort of 10,000 names or a page of prices is one call; a single value costs the same
  one call. Memory: nothing until first use; then the compiled module, shared by every core, and each
  calling request's instance under its own cap. The CLDR data is in the binary, measured by Stage 7.
  Usability: locale-correct sorting, numbers, dates and negotiation on every install with no
  configuration. Simplicity: one more component through a build step that already exists, and an API
  of batch members, with a single value written as a list of one.
- **Neutral names only** in every test, example and record — `Shop`, `Blog`, `example.com`.
- **Every comment in a new `.nvs` and every `about.md` this goal writes follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over
  them before the wrap.
- **A debug cargo command never takes `-p`.** Narrow what runs with `bun nv verify -p nvs-ext` or a
  `--test` filter.
