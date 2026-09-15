---
milestone: M8
---
# Loop goal 59 — every class M8 names is as deep as its spec section

Every class M8 promises outside the database now does what its spec row and its record say. That covers
`Core\Decimal`'s full roster, `Core\Reflect` acting through the same visibility check ordinary code faces,
`Core\Ast` answering typed nodes, `Core\Metrics`, `Core\Process::spawn`, a `Core\Cli\Text` that renders per
stream, and a CSV reader that streams a file. The record producers all write one shape, and the two
benches M8's records ask for exist as guards. Nothing M8 promised outside the database is left in a
`# Known gaps` block tagged `M8`, and no promise an earlier milestone made to these classes is left open.

## Why here

M8 is closed in two halves that share no file set: goal `m8-db-queue` takes the database, this goal takes
everything else. Their order is the chain's, not a dependency. This goal needs nothing that goal builds,
and its floor is that goal's list. It sits in front of goal `unowned-closures` for a reason: that goal's
decision sheet was written over whatever was still `unowned`, and this goal builds four of those items:
`Core\Metrics` and `Core\Process::spawn`, and — to the answers the user gave on that sheet — `Core\Json`'s
undecodable field types and `Core\Ast::parseFile`. `array<T>`'s variance, once on the same list, is
already built (stage 2). It sits in
front of goal `gap-zero` for that goal's standing reason: a register is emptied after everything that adds
to it has run.

What it needs, already built, is:

- `nvs_runtime::decimal`'s arithmetic;
- the dispatch roster in `crates/nvs-stdlib/src/instance.rs:115`;
- `Core\IO\File` and `Core\IO\Lines` (`crates/nvs-stdlib/src/io.rs:1270`, `:1627`);
- `nvs_render`'s model and its plain and JSON renderings;
- the engine floor's record (`crates/nvs-runtime/src/floor.rs:94`);
- the per-core metrics registry (`crates/nvs-server/src/metrics.rs`);
- `Core\Process::run`'s door and off-core wait (`crates/nvs-stdlib/src/process.rs:36-50`).

## Stage 0 — the catch-up

Sentences on disk that are already wrong, or that name an owner who will not close them. Each correction
here is a doc edit and no code. Re-grep before editing: these are anchors, and files move.

1. **Owners are re-pointed at this goal**, so the register says who closes what:
   - `crates/nvs-stdlib/src/heap.rs:47` and `crates/nvs-stdlib/src/cldr.rs:165`, `:183` (`gap-zero`);
   - `crates/nvs-stdlib/src/ast.rs:64` (`unowned`) and `crates/nvs-stdlib/src/json.rs:143` (`unowned`,
     the same knot as `crates/nvs-types/src/derive.rs:44`'s `M8` gap 1);
   - `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:16` and
     `crates/nvs-stdlib/tests/migration-members-outstanding.txt:22` (`unowned` → `m8-stdlib-depth`);
   - `docs/agent/carried-gaps.md:62`'s owner cell.
2. **`crates/nvs-stdlib/src/cldr.rs:166-176` gap 3 is not a gap.** It says the twenty languages it used to
   name are carried, and `every_language_named_absent_in_the_gap_note_now_has_a_rule`
   (`crates/nvs-stdlib/src/cldr.rs:2812`) holds it to the table. The sentence moves into the module doc's
   prose about the roster's boundary, and the item leaves the gap block.
3. **`crates/nvs-stdlib/src/out.rs:34-42` gap 1 is half stale.** `Core\Cli\Text` does have members now:
   `plain` and `styled`, at `crates/nvs-stdlib/src/cli.rs:1945` and `:1954`. What `through` still cannot do
   is *read* the text it was handed, because the class has no instance member (`instance: &[]`, `:1963`).
   Rewrite the gap to say that, pointing at stage 8, which closes it.

## Stage 1 — the floor

Goal `m8-db-queue`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — none: `array<T>` is already covariant

`crates/nvs-types/src/expr/assign.rs:159-175` already makes `array<T>` covariant in its element, with the
soundness argument in its comment (a copy-on-write value cannot alias). The code is ahead of
`rule:types/arrays`' *Invariant.* bullet, so under the user's rule the fragment is amended to the code —
that is goal `plan-truth`'s, together with striking `crates/nvs-stdlib/src/lib.rs` gap 4. This goal opens
no record for it. A session here only confirms `Core\Arr::flip($stringArray)` compiles; if it does not,
that is a gap in the code, recorded against this goal and built here.

## Stage 3 — `Core\Decimal`'s roster

One file set: `crates/nvs-stdlib/src/decimal.rs` and `crates/nvs-stdlib/src/math.rs`.
`rule:types/decimal`, ADR 0054 §§ 3–4.

1. **`allocate($amount, $ratios)`**: the penny split whose parts add back to the sum exactly.
   `crates/nvs-stdlib/src/decimal.rs:38` gap 1, which M8's plan paragraph names outright.
2. **`floor`, `ceil`, `truncate` and `round`, answering `decimal`.** `round` takes `Core\RoundMode`
   (already a parameter type at `crates/nvs-stdlib/src/decimal.rs:73`). This is the same work as
   `crates/nvs-stdlib/src/math.rs:32-42`'s gap. That gap becomes a decision: `Core\Math`'s four stay
   `float`, and the exact rounding lives on `Core\Decimal`.
3. **`pow`**: the member `**` with a `decimal` base is refused in favour of (`docs/decisions/0054.md:111`).
4. The gap block at `crates/nvs-stdlib/src/decimal.rs:36-45` goes, and the module doc is rewritten whole.

## Stage 4 — the two guards

One file: `benches/abi-probe/tests/perf_guards.rs`. The shape is already there: a release-only test with
a loose threshold that prints its own figure (`:1-15`).

1. **ADR 0054's decimal loop.** A typed `decimal` arithmetic loop, as a figure with a guard, beside the
   `int` one at `benches/abi-probe/tests/perf_guards.rs:676` (`docs/decisions/0054.md:246-250`). The
   figure's program goes where `python tools/dossier.py --id` puts the feature's bench
   (`benches/members/README.md` § *Where a bench goes*).
2. **ADR 0056's tier comparison.** The linear tier's throughput against the backtracking tier's on one
   shared corpus, so "the default tier is not a performance concession" is measured
   (`docs/decisions/0056.md:146-148`).

## Stage 5 — `decimal`, `Instant` and an inline shape on the JSON wire

One file set: `crates/nvs-stdlib/src/json.rs` and `crates/nvs-types/src/derive.rs`.
`rule:core-classes/derive-field-list`.

1. **`crates/nvs-stdlib/src/json.rs:128-143` gap 1 and `crates/nvs-types/src/derive.rs:44-71` gap 1 are
   one knot.** The three types erase to `CodecTy::Opaque`, so `decodeAs<T>` refuses before reading.
   Each gets its wire form (§ *Standing decisions*), a `CodecTy` arm, and a `decode_field` case. An
   `array<T>` of one of them follows for free.
2. **ADR 0054's M8 bullet**: `Core\Json::decode` into a `decimal` shape field round-trips a
   25-significant-digit number exactly (`docs/decisions/0054.md:251-252`). No case decodes into a
   `decimal` today; only the encode side is pinned
   (`tests/conformance/core/json-encode-decode-and-validate.nvst:20`).

## Stage 6 — `Core\Reflect` acts through the check ordinary code faces

One file set: `crates/nvs-stdlib/src/reflect.rs`, plus the write path it reaches through,
`nvs_runtime::write_erased_property`. `rule:core-classes/reflect`,
`rule:security/reflection-enforces-visibility`.

1. **The remaining `*Info` classes.** ADR 0019 § 1 names `MethodInfo`, `PropertyInfo`, `ParameterInfo`,
   `ConstantInfo`, `AttributeInfo` and `EnumInfo`. Only `Core\Reflect` and `ClassInfo` are registered
   (`crates/nvs-stdlib/src/reflect.rs:112`, `:293`) — gap 1 at `:73`. A description names its methods,
   so `get_class_methods` and `method_exists` get their answer.
2. **The walk knows its call site** — gap 2 at `crates/nvs-stdlib/src/reflect.rs:78`. From inside the
   described class a reflective read sees what an ordinary read there would; from outside, it does not.
3. **A reflective method call and a reflective constructor**, each through the visibility check. Also, a
   reflective write runs the property's `set` hook instead of writing past it — gap 3 at
   `crates/nvs-stdlib/src/reflect.rs:83-91`. **This is M8's named acceptance: a reflective call to a
   `private` method from outside its class fails like the ordinary call.**

## Stage 7 — `Core\Ast`'s typed roster, `parseFile`, and the fuzz target

One file set: `crates/nvs-stdlib/src/ast.rs`, `crates/nvs-syntax/src/walk.rs`, `fuzz/`.
`rule:core-classes/ast-is-inert`.

1. **One class per production** — `Core\Ast\ClassDecl`, `Core\Ast\MethodDecl`, and so on, beneath the
   `Core\Ast\Node` every node already is — gap 1 at `crates/nvs-stdlib/src/ast.rs:46`. Which productions
   are nodes is `nvs_syntax::walk`'s module doc, and the roster is derived from that table rather than
   written a second time.
2. **`parseFile`, to its decision** — gap 3 at `crates/nvs-stdlib/src/ast.rs:60` carries the user's
   `Decided:` sentence: build it behind `Core\IO`'s `fs.read` door, or strike spec § 3's row (ADR 0019
   § 3 names it beside `parse`) and say in the module doc how a program composes the two.
3. **M8's fuzz bullet**: "`Core\Ast::parse()` fuzzed with the same corpus as M1's lexer/parser target".
   Today `fuzz/fuzz_targets/parse.rs` fuzzes `nvs_syntax::parse_file` directly, no committed seeds exist
   for it (`fuzz/seeds/` holds `prefix` alone), and nothing replays a corpus on stable. So: an `ast`
   target in `fuzz/Cargo.toml` over `Core\Ast::parse`, committed seeds under `fuzz/seeds/parse/` that both
   targets read, and a stable test in `nvs-stdlib` that replays those seeds through both doors and asserts
   the same verdict.

## Stage 8 — a `Core\Cli\Text` of runs

One file set: `crates/nvs-stdlib/src/cli.rs` and `crates/nvs-stdlib/src/out.rs`.
`rule:tooling/styling-is-a-value-not-a-grammar`, `rule:tooling/the-terminal-profile-resolves-once`.

1. **A `Text` holds runs, not rendered bytes**, and the sink renders them for the stream it is writing to
   — gap 2 at `crates/nvs-stdlib/src/cli.rs:135-145`. One `Text` is styled on a terminal standard output
   and plain on a redirected standard error in the same run.
2. **A `Text` can be read**: enough instance surface that `Core\Out::capture`'s `{through:}` can rewrite
   what it captured, which closes `crates/nvs-stdlib/src/out.rs:36` gap 1. The spelling comes from spec
   § 13's `Cli\Text` body under `rule:core-api/verb-lexicon`.

## Stage 9 — every record producer writes one shape

One file set: `crates/nvs-runtime/src/floor.rs`, `crates/nvs-render/src/`, `crates/nvs-stdlib/src/debug.rs`.
`rule:errors/record-producers`, `rule:errors/renderings`.

1. **An uncaught `Throwable`'s frames are Sequence-of-Object nodes**, as `rule:errors/record-producers`'s
   table says. Today they are one `backtrace` string (`crates/nvs-runtime/src/floor.rs:86-107`). A
   `secret`-typed property on an object in a frame is redacted there, which is ADR 0092's M4 bullet
   (`docs/decisions/0092.md:419-420`). `crates/nvs-stdlib/src/debug.rs:77-83` gap 3 is struck, because
   the producer is `nvs_runtime::floor::uncaught`, which already exists.
2. **The HTML rendering**, the third of `rule:errors/renderings`' three, as `nvs_render::html`, carried
   by `Core\Html\Markup`. ADR 0092's M4 bullets ask that plain, JSON and HTML agree on an elision and
   that a `secret` renders as the placeholder in all three (`docs/decisions/0092.md:410-418`). Wiring
   the rendering into a request under `[debug] inline` is ADR 0092's M7 bullet (`:423-425`). It is this
   goal's work unless goal `m7-server-surface` has already built it by the time this stage opens.
3. **`crates/nvs-render/src/lib.rs:12-19`** is rewritten whole. What is left unbuilt moves into a real
   `# Known gaps` block with owners (§ *Standing decisions* says which).
4. The floor/`Core\Log` schema case already exists
   (`tests/conformance/error/the-floor-and-core-log-write-record-the-same-error-the-same-way.nvst`) and
   must stay green through item 1's change of shape.

## Stage 10 — the dispatch roster: CSV streams a file, and `Core` instances order themselves

One file set: `crates/nvs-stdlib/src/instance.rs:115`'s `DISPATCH_ROSTER`, plus `csv.rs`, `io.rs`,
`heap.rs`, `time.rs` and `uri.rs` in `crates/nvs-stdlib/src/`. `rule:classes/comparable`.

1. **A streaming CSV reader over `Core\IO\File`** — gap 1 at `crates/nvs-stdlib/src/csv.rs:128`. The
   module's parse loop is already a fed-buffer loop over `csv-core`, and `Core\IO\Lines`
   (`crates/nvs-stdlib/src/io.rs:1627`) is the precedent for an iterable handle on the roster. The spec
   § 12 row is added beside `Csv::parse` (`docs/spec/01-core-library.md:966`).
2. **Every `Core` class that declares `compareTo` gets a dispatch row**, so a `Core\Heap` of them needs no
   comparator. That is `crates/nvs-stdlib/src/heap.rs:40-47`'s gap. The classes are the rows at
   `crates/nvs-stdlib/src/time.rs:340`, `:1198`, `:2501`, `:2797` and `crates/nvs-stdlib/src/uri.rs:622`.
   The rows are derived from the registry (`crates/nvs-stdlib/src/registry.rs:3036-3052` already finds
   every such member), not listed by hand.

## Stage 11 — CLDR's refused letters and its ordinals

One file: `crates/nvs-stdlib/src/cldr.rs`.

1. **Every letter gap 2 names formats** — `Y`, `e`, `U`, `r`, `B`, `b`, `A`, `g`, `z`, `Z`, `O` and `v`
   (`crates/nvs-stdlib/src/cldr.rs:156-165`). Each answers what ICU answers for the Gregorian calendar,
   the only calendar Novis has. `Y` and `e` read a transcribed per-territory week table.
2. **`ORDINALS` is every language CLDR gives an ordinal rule to**, so a missing row can no longer answer
   `Other` silently — gap 4 at `crates/nvs-stdlib/src/cldr.rs:177-183`.

## Stage 12 — `Core\Metrics`

One file set: a new `crates/nvs-stdlib/src/metrics.rs`, `crates/nvs-server/src/metrics.rs`, and wherever
the per-core registry moves to so both can reach it. `rule:observability/metrics-three-members`,
`rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`.

1. **`increment`, `observe` and `gauge`** as `rule:observability/metrics-three-members` writes them. They
   accumulate into the same per-core registry the exporter scrapes, and they are Tier 0 in every build.
   `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:16` is struck in the same slice.
2. **ADR 0076's M8 bullets** (`docs/decisions/0076.md:367-378`), except the `traceparent` and `Core\Log`
   ones, which goal `http-client` built:
   - a name used as a gauge and then incremented throws naming both call sites;
   - a literal name outside `[a-z][a-z0-9_]*` is a compile error;
   - a `tainted` label value is a compile-time diagnostic;
   - a build with the exporter feature off still accumulates.

## Stage 13 — `Core\Process::spawn`

One file set: `crates/nvs-stdlib/src/process.rs`, and `benches/abi-probe/tests/perf_guards.rs` for the
guard. `rule:core-classes/process-spawn`, `rule:core-classes/process-is-argv-only`.

1. **`spawn` answers a handle**: read stdout, read stderr, write stdin, wait, kill. Every read and write
   suspends the calling coroutine, as `run`'s wait does (`crates/nvs-stdlib/src/process.rs:36-50`).
   `crates/nvs-stdlib/tests/migration-members-outstanding.txt:22` is struck in the same slice.
2. **M8's `Core\Process` bullets that are still open.** A `tainted` `$path` or `$argv` element is refused
   where it is written; only the shell-string refusal is pinned today
   (`tests/conformance/reject/there-is-no-shell-string-form-of-process-run.nvst`). And there is a
   concurrent-spawn scheduler guard beside the coroutine guards: `perf_guards.rs:436` measures one spawn,
   never several at once.
3. A `spawn` produces exactly one span (`docs/decisions/0076.md:372-373`).

## Stage 14 — the rest of M8's non-database verification lists

Each is one test or one case, over files the stage names. A session re-checks each against the tree
first and strikes one that has landed, citing the evidence.

1. **ADR 0057 § 4, the differential.** The prepared artifact and the runtime-built one are byte-identical
   across a corpus of literals for all four intrinsics. The format half exists
   (`crates/nvs-stdlib/src/format.rs:835`); the pattern, URI and date-format halves do not.
2. **ADR 0057 § 5, coverage.** A statement holding a folded intrinsic call is still reported covered.
   None of `crates/nvs-codegen/tests/probes.rs`'s coverage tests (`:140`, `:206`) holds one.
3. **ADR 0057's cache bullet.** A prepared entry from another compiler build is a miss.
   `crates/nvs-cli/src/cache.rs:2505` proves this for the whole payload. If prepared entries ride inside
   that payload, the bullet is struck with that evidence and needs no test.
4. **ADR 0056's per-pattern tier record.** Every literal pattern in the regex suite has its tier written
   to a committed file, so an engine change that moves a pattern shows up in a diff
   (`docs/decisions/0056.md:144-145`). `crates/nvs-types/tests/intrinsics.rs:206` settles one pattern's
   tier and records nothing.
5. **ADR 0059's coherence bullet.** A value written through the local tier on one core is absent on
   another (`docs/decisions/0059.md:163-164`). The existing case,
   `tests/conformance/core/cache-a-local-entry-may-be-absent-at-any-time.nvst`, runs on one core.
6. **ADR 0060's last bullet.** No protocol class exposes a raw-value accessor
   (`docs/decisions/0060.md:165-166`).

## Stage 15 — the rulebook

Flip `core-classes/process-spawn`, `observability/metrics-three-members` and
`observability/the-exporter-is-a-feature-and-core-metrics-is-not` to `shipped`, fill `guardedBy` from this
goal's cases and tests, and run `python tools/rules.py --render`. `errors/record-producers` stays
`designed`, because two of its five producers are not this goal's.

## Standing decisions

- **The user's rules, settled.** Milestones M0–M8 are complete when this chain's M8 goals are green, and
  every promise a past milestone's plan made to these classes is built. An item is deferred to M9 or later
  only when it **cannot be built** without that milestone's work — never because it is large. Size is a
  reason to split a slice, not to move it.
- **Decide and record; never `BLOCKED` for a design call.** Every call is settled under ADR 0004's
  ordering: security, then PHP-compatible correctness, then request-path latency, then simplicity, then
  memory. It is recorded in the module doc of the code it shapes, and in the spec row when it names a
  member. A spelling is fixed under `rule:core-api/verb-lexicon` and needs no record.
- **ADR slots: none.** `array<T>`'s covariance is already in the code (stage 2), and every other call
  here is settled by a rule, a `Decided:` sentence from the user's decision sheet, or the verb lexicon.
- **Every check runs locally.** CI is not running (a GitHub billing block), so no check leans on a
  workflow. The fuzz target is written and seeded, but the acceptance runs the stable replay test and
  `cargo metadata`'s view of the target, never `cargo +nightly fuzz`, because libFuzzer needs nightly and
  does not run on Windows.
- **`Core\Decimal`:**
  - `allocate(decimal $amount, array<…> $ratios): array<decimal>` answers parts at the amount's scale.
    Each part is the ratio's share rounded down, and the remainder goes one smallest unit at a time to
    the earliest parts, so the answer is deterministic and sums exactly. An empty, all-zero or negative
    ratio list throws.
  - `pow` takes a `uint` exponent and is exact or throws at the mantissa or scale bound. A negative power
    is written as `divRound(1, pow(…))`, which says the rounding out loud.
  - The four rounding members take a target scale defaulting to 0. `round` takes `Core\RoundMode` with no
    default, because naming the mode is the point.
- **The JSON wire is the user's, from the decision sheet.** The `Decided:` sentence in
  `crates/nvs-stdlib/src/json.rs` gap 1 fixes how a `decimal` and an `Instant` are written, and it is
  built exactly: a decoder accepts what that encoder writes, and a `decimal` is never read through
  `f64`. An inline shape is an object. Where `rule:core-classes/derive-field-list` says otherwise, the
  rule is amended to the decision in the same slice.
- **`Core\Reflect`'s call site:**
  - The compiler supplies the enclosing class to the acting members as a hidden argument no program can
    write, so it cannot be forged. The safe fallback, if it cannot be threaded, is to treat every
    reflective act as coming from outside the class. That fails closed, and M8's acceptance holds under it.
  - A reflective write runs the `set` hook. Writing past a hook is a correctness hole, and priority 2
    outranks the simplicity of the erased store.
- **`Core\Ast`:** the roster is generated from `nvs_syntax::walk`'s production table, the one home.
  `parseFile` is whatever the `Decided:` sentence at `crates/nvs-stdlib/src/ast.rs` gap 3 says — built
  behind `Core\IO`'s `fs.read` door, or its spec § 3 row struck so a program composes `Core\IO::read` with
  `parse`. `rule:security/reflection-needs-no-capability` keeps covering `parse` either way.
- **`Core\Cli\Text`:** the runs are rendered at the sink for the stream in force. Colour depth stays a
  process answer, and a stream that is not a terminal gets no styling
  (`rule:tooling/the-terminal-profile-resolves-once`). Since a `.nvst` case pipes both streams, the
  per-stream proof is a Rust test with a fake terminal answer.
- **The record producers:** the rule wins over the code. `crates/nvs-runtime/src/floor.rs:88-92` chose one
  string for "a stack summary", and `rule:errors/record-producers` asks for Sequence-of-Object frames. The
  JSON rendering writes an array of `{function, file, line}` objects, and the plaintext rendering still
  prints the `#0` form a person greps for. Two producers are not this goal's:
  - the compiler diagnostic's is M10, by ADR 0092's own list (`docs/decisions/0092.md:439-440`);
  - the `#[Test]` result's is written into `nvs-render`'s new gap block tagged with the milestone
    ADR 0079 § 24 gives § 22. If that milestone is M8 or earlier, it is built here instead, in stage 9.
- **CSV streaming** is spelled `Core\Csv::rows(Core\IO\File $file, {…parse's options}): Core\Csv\Rows`
  unless the verb lexicon says otherwise. It is iterable through the dispatch roster exactly as
  `Core\IO\Lines` is, and `{header: true}` keys every record by the first row.
- **CLDR:** each letter matches ICU under the Gregorian calendar. `U` and `r` are the year there. `z` and
  `v` fall back to the localized GMT format where the transcribed data has no name for the zone, which is
  ICU's own fallback. `B` and `b` read day periods transcribed for the languages the cardinal roster
  carries, and other languages take ICU's root fallback. No letter stays refused.
- **`Core\Metrics`:** `nvs-stdlib` cannot depend on `nvs-server`, so the per-core registry type moves to
  where a `Ctx` can reach it, and the exporter stays feature-gated in `nvs-server`. The move is recorded
  in both module docs. No value a program can read comes back from the class
  (`rule:observability/a-registry-is-per-core-and-nothing-reads-it`).
- **`Core\Process::spawn`:**
  - The handle class's spelling is fixed under the verb lexicon.
  - A child still running when its task ends is killed, so nothing outlives the task that opened it and
    memory stays O(in-flight).
  - If `run` has no options bag yet, `rule:core-classes/process-options`' `ProcessOptions` lands on both
    members together.
- **What it spends:**
  - `allocate` spends one array per call.
  - A `Text` of runs spends one small array per `Text`, instead of the bytes it held before.
  - `Csv::rows` holds one record and `csv-core`'s buffer per open reader, never the document.
  - A spawned child spends two or three pipes and a handle, charged to its task and released with it.
  - The metrics registry is per core, capped by `max_series`, and already paid for by the exporter.
  - The `*Info` and `Core\Ast\*` classes spend one descriptor each, O(classes in the program), and a
    parsed tree still spends one object per node, as it does today.
  - Nothing added is per process, and nothing grows with requests served.
- **Not this goal:**
  - the database half (goal `m8-db-queue`), which includes `crates/nvs-types/src/derive.rs:72-83` gap 2
    and `crates/nvs-cli/src/worker.rs:101`;
  - `request.rs`, `response.rs` and `test.rs` (goal `m7-server-surface`);
  - every other `unowned` gap (goal `unowned-closures`, after its decision sheet), which includes
    `crates/nvs-stdlib/src/ast.rs:52` gap 2 (positions, which M8 never promised) and
    `crates/nvs-stdlib/src/json.rs` gaps 2–5;
  - the M6-tagged `crates/nvs-stdlib/src/regex.rs:77` and `:91` gaps (goal `unowned-closures`);
  - ADR 0055's extension fixtures (M9);
  - ADR 0058's client bullets, which goal `http-client` carried.

  A session that finds one of these on its path writes it to the handoff's `## Backlog`.
