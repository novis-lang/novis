---
looper: 1
title: Text is tainted unless the checker proves it clean
summary: "Reached when one exhaustive classifier decides every taint walk and a type it does not recognise counts as tainted, a Core member's returned text is tainted unless its row says where the text comes from, every expression form and every codec has a taint row, no launderer returns a value its own sink reads as an instruction, the request path's IR is byte-identical and the checker's time is within 3% of its baseline, and the features that moved have their proofs and their website entries."
context: [docs/rules/security/taint-propagation.md, docs/rules/security/tainted-sources.md, docs/rules/security/unclassified-parameter-refuses-tainted.md, crates/nvs-types/src/expr/quals.rs]
stages:
  - id: 1
    title: "the request path's IR and the checker's time have a baseline, and one command compares a build against it"
    status: open
    context: [tools/nv/cmd/start-cost.ts, docs/rules/testing/perf-two-mechanisms.md]
  - id: 2
    title: "one exhaustive classifier decides every taint walk, and a type it does not recognise counts as tainted"
    status: open
    context: [crates/nvs-types/src/expr/quals.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/expr/args.rs]
  - id: 3
    title: "a Core member's returned text is tainted unless its row says where the text comes from"
    status: open
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/tests/taint_signatures.rs, docs/rules/security/tainted-sources.md]
  - id: 4
    title: "every expression form and every codec has a taint row, and a form or a type without one does not compile"
    status: open
    context: [crates/nvs-types/src/derive.rs]
  - id: 5
    title: "no launderer returns a value its own sink reads as an instruction"
    status: open
    context: [crates/nvs-stdlib/src/uri.rs, docs/rules/security/launderers-are-sink-named.md]
  - id: 6
    title: "the features that moved have their proofs, their pages and their change entries"
    status: open
    context: [website/src/data/changes.json]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: against-base
    stage: 1
    name: "bun nv against records a base build and compares IR and check time, and two runs of one build compare equal"
    run: [bun, test, tools/nv/test/against.test.ts]
    timeout: 600
  - id: text-class
    stage: 2
    name: "every Ty variant has an arm in the text classifier, and an unknown Core class, a generic wrapper and a nested shape each count as tainted"
    run: [cargo, test, --test, text_class]
    timeout: 1800
  - id: taint-signatures
    stage: 3
    name: "every Core row that returns text says where the text comes from, and a row with no mark answers tainted"
    run: [cargo, test, --test, taint_signatures]
    timeout: 1800
  - id: taint-every-form
    stage: 4
    name: "every expression kind that can give text has a row, and a tainted operand gives a tainted result in each"
    run: [cargo, test, --test, taint_every_form]
    timeout: 1800
  - id: launderer-matrix
    stage: 5
    name: "every launderer's answer for dot segments, separators and control bytes is safe at its own sink"
    run: [cargo, test, --test, launderer_matrix]
    timeout: 1800
  - id: proofs
    stage: 6
    name: "every feature the goal's work reached has its proofs"
    run: [bun, nv, proofs, --gate, --goal, taint-fail-closed]
    timeout: 1800
  - id: close
    stage: 7
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, taint-fail-closed]
    timeout: 900
---

# Text is tainted unless the checker proves it clean

## Why

The sink side of `tainted` already fails closed. A `Core` parameter that nobody classified refuses a
tainted argument (`rule:security/unclassified-parameter-refuses-tainted`), and a user's plain `string`
parameter refuses one too. The source side and the propagation side still fail open. The bit is
positive: `Ty::TaintedString` is set by the walks that know a type, and every walk ends in an arm that
answers "clean" for a type it does not know. `crates/nvs-types/src/expr/quals.rs` has about twenty
such arms (`carries_tainted` at `:260`, `tainted_result` at `:897`, `keys_from_outside` at `:361`,
and more), and `crates/nvs-types/src/derive.rs` four more. A `Core` row that returns `CoreTy::Str`
returns clean text, whatever the member reads: 111 rows return `CoreTy::Str`, 35 `CoreTy::Bytes` and
22 a nullable of either, against 53 that return a tainted form.

The 0.0.5 review's taint hole was this shape. `classes_reached`' predecessor matched `Ty::Class` and
`Ty::Array` and answered `_ => None`, so `jsonAs<{ user: User }>` compiled where `jsonAs<User>` is
`E0810`. Commit `7c2cd558df` fixed the shape, and the new walk still ends in `_ => {}`
(`crates/nvs-types/src/derive.rs:2385`).

A probe on the release binary of 2026-11-22 found one more hole, of another kind.
`Core\Uri::encodeComponent` launders, and its answer for `..` is `..`: `.` is unreserved
(`crates/nvs-stdlib/src/uri.rs:1213`). Its doc says "No byte of the result can change the structure
of the URI", and a `..` path segment does. The same answer reaches a path sink as clean text:
`Core\IO::exists(Core\Path::thisDir('sub') . '/' . Core\Uri::encodeComponent($tainted) . '/x')` with
`..` read a file outside `sub/`. The `fs` grants still bound it.

The same review ruled out four paths, each by reading the code, and they need nothing here:

- an `inout` argument must have the exact type of its parameter (`crates/nvs-types/src/expr/args.rs:1769`);
- a generic class's type arguments are invariant (`crates/nvs-types/src/expr/assign.rs:611`);
- a property redeclared at a narrower type is refused, and a `covariant` one is checked for dropping
  `tainted` (`crates/nvs-types/src/conformance.rs:387`);
- the editor's check cache keys on the source of every declaration in the graph, types included
  (`crates/nvs-lsp/src/surface.rs:1`), and `nvs check` keeps no per-file cache.

`Core\Uri::parse` of a tainted string does not compile, because a contagious parameter whose return
carries no text refuses a tainted argument. So a `Core` object built from tainted text is already
closed, except where the object comes from a source member with no argument, such as
`Core\Request::part`. Stage 3 covers that case through the getters' rows.

## Standing decisions

**Clean is the proof, and tainted is the default.** Text is clean when it is a literal or a constant,
a configuration value, a launderer's answer, the result of a checked conversion, or the result of an
operation whose every text operand is clean. Everything else is tainted. The language does not change:
`tainted string` is still the written form, and a plain `string` still means clean. Only the
checker's and the registry's defaults move. Reason: a forgotten case then gives a compile error, never
a hole.

**One exhaustive classifier, not twenty exhaustive matches.** `quals.rs` gains one function,
`text_class(ty) -> TextClass`, with variants `Clean`, `Tainted`, `NoText` and `Holds(children)`. It
matches every `Ty` variant by name, with no `_` arm, so a variant added later does not compile until
somebody decides it. Every walk of § *Why* reads its answer through `text_class` and keeps only its own
direction: a walk that *sets* the bit treats an unknown answer as tainted, and a walk that *admits*
treats it as refused. An unknown `Core` class is `Tainted` unless the registry lists it as `NoText`.
A user class is `Holds` its declared properties, as today.

**A `Core` row that returns text says where the text comes from.** `registry.rs` gains a return mark
with three values:

- `Derived`: the text is made only from the arguments and the program's own state. It is tainted
  when a contagious argument is, as today.
- `Source`: the text comes from outside. It is always tainted.
- `Clean(reason)`: the text is never from outside, with a sentence that says why. Examples are
  `Core\IO` reads, a status code, `Core\Storage::list` keys and `Core\Env::secret`, each of which
  `rule:security/tainted-sources` already decided.

A row with no mark answers tainted, so a member added later is safe until its author says otherwise.
`taint_signatures.rs` lists each `Clean` row with its reason, and fails on a reason that is empty.
Most of the 168 rows are `Derived`. The classification is one pass, and it never recurs.

**The bit stays a compile-time fact.** Nothing in this goal adds a tag, a branch or a byte on the
request path. The one runtime change is in stage 5: a launderer refuses a dot segment.

**What performance this goal may spend.** Nothing at run time, and at most 3% of the checker's time.

- **Run time:** the IR that `nvs run --dump-ir` prints for every program of the corpus is
  byte-identical to the base build's. The classifier only decides which programs compile, and a
  program that compiles must lower to the same code. An IR difference is a bug in this goal, never
  an accepted cost.
- **Check time:** the median ratio of `nvs check` over the corpus, new build against the base
  build, in alternating pairs, is at most 1.03. The walks keep their shape and gain one match per
  step, so the expected cost is noise.
- **If the check time is over budget,** `text_class` is memoised per `TypeId` in a side table of the
  interner. It costs one byte per interned type, at compile time only. Raising the budget is not a
  fix.
- **The editor's guard stays green.** `crates/nvs-lsp/tests/latency.rs` runs in the gate stage, and
  its bound is never raised.

**`bun nv against` is the one way to compare.** It runs after every code stage, so it is a command:
`bun nv against --record` copies `target/release/nvs` and its commit to `.agent-tmp/against/base/`.
`bun nv against` compares the current release binary with the base. It prints every program whose IR
differs, and the median check-time ratio with its pairs. The corpus is `benches/members/` and
`docs/examples/` for the IR, and the 40 largest programs of both for the timing. A base that differs
from itself on a second dump fails the record, so a dump that is not deterministic is found before it
can hide a change. The gate stage deletes `.agent-tmp/against/`.

**A launderer's answer is safe at its own sink.** `Core\Uri::encodeComponent` throws a `LogicError`,
as the runtime's other argument errors do, on an argument that is exactly `.` or `..`, and its doc says so. A query value goes through
`Core\Uri::withQueryParameter` or `buildQuery`, which the error names. Reason: a path segment is the
first use its doc names, and a dot segment changes a URI's structure there. Encoding the dots as `%2E`
does not help, because a URL parser reads `%2e%2e` as `..`. It costs one length test per call. The
same question is asked of every launderer in stage 5's matrix.

**A launderer used at another sink is not this goal's to close.** The terminal escape's answer for
`..` is `..`, and a path sink takes it. The root fix is an answer type for each sink, which
`rule:security/launderer-answers-a-carrier` declined. Stage 5 writes a gap record for each cell of the
matrix that is only a hole at another sink. It names that carrier design as the fix, for the user to
decide.

**The HTML template's blind spot stays out.** A hole in `href=` or `<script>` is escaped as element
text, and `rule:core-classes/html-template` records that and its Declined list. Closing it reverses a
recorded decision and needs an HTML position model, so it is a goal of its own if the user wants one.

**What stops it.** One attempt per item, and two sessions per stage, counted in `### State`. At two
sessions a stage's open items move to `### Backlog` as declined, each with its reason, and a security
item among them gets a gap record. No stage gains an item. A `bun nv against` that is over budget
after the memo is a stop: the session records the figures under `### State` and writes `BLOCKED`.

**Lanes.** Stage 1 comes first, because its base must be recorded before any change. Stages 2 and 4
share `quals.rs` and `derive.rs`, and run in order. Stage 3 touches `crates/nvs-stdlib` and may run
beside stage 2 as a side goal once stage 1 is done. Stage 5 touches `uri.rs` and its own test, and may
run beside either. This goal starts after `safety-holes`. Gate: `docs/goals/safety-holes.md` is
deleted.

## Stages

### 1. the request path's IR and the checker's time have a baseline, and one command compares a build against it

- [ ] `tools/nv/cmd/against.ts:1`: `--record` and the compare, as the standing decision gives them.
  The help text follows `docs/agent/conventions.md` § *An `nvs` command*. The command finds programs
  with `Glob`-style walks over the two folders, and uses one `nvs check` and one `nvs run --dump-ir`
  per program.
- [ ] `tools/nv/test/against.test.ts:1`: a record of the current binary, then a compare of that same
  binary against it, prints no IR difference and a ratio between 0.9 and 1.1. It runs once, by name.
- [ ] `bun nv relink --build`, then `bun nv against --record` on the commit before stage 2. The commit
  and the corpus size go under `### State`.
- [ ] `docs/agent/commands.md`: one line for `bun nv against` in its table.

### 2. one exhaustive classifier decides every taint walk, and a type it does not recognise counts as tainted

**Split:** an `Explore` subagent lists every variant of `Ty` (`crates/nvs-types/src/ty.rs`), one line
each, before the classifier is written.

- [ ] `crates/nvs-types/src/expr/quals.rs:1`: `text_class` and `TextClass`, as the standing decision
  gives them. `rule:security/taint-propagation`
- [ ] `quals.rs`: every catch-all arm of § *Why* reads `text_class`. Each keeps its direction: those
  at `:260`, `:305`, `:315`, `:327`, `:361`, `:452`, `:503`, `:525`, `:554`, `:654`, `:680`, `:897`,
  `:931`, `:994`, `:1418`, `:1741`, `:1906`. `:1873` already answers tainted and stays.
- [ ] `crates/nvs-types/src/derive.rs:2385` (`classes_reached`), `:2408` (`unqualified_text`),
  `:2529` (`db_reachable`) and `:2597` (`json_reachable`) read `text_class`. `classes_reached` steps
  into a class's type arguments. *check first*: `db_reachable` lists no `SecretString`,
  `SecretBytes` or `SecretTaintedString`, and `json_reachable` no `TaintedBytes` or `SecretBytes`.
  Write one program for each that should not compile.
- [ ] `crates/nvs-stdlib/src/registry.rs:4461` and `:4313`: `carries_tainted_bytes` and
  `holds_cells_of_its_source` keep their lists. An unlisted `Core` class answers `Tainted` unless the
  registry lists it as `NoText`.
- [ ] `crates/nvs-types/tests/text_class.rs:1`: an unknown `Core` class, a generic wrapper, a shape
  in an array in a nullable, and `jsonAs<{ a: { b: User } }>` each count as tainted. It runs once,
  by name.
- [ ] The rule fragments `taint-propagation.md` and `tainted-qualifier.md` say "clean is the proof"
  in one paragraph each, and the `quals.rs` module doc says it once.
- [ ] `cargo build`, `cargo clippy --all-targets -- -D warnings`, then `bun nv relink --build` and
  `bun nv against`. No IR difference, and the ratio goes under `### State`.

### 3. a Core member's returned text is tainted unless its row says where the text comes from

**Split:** an `Explore` subagent lists every row that returns `Str`, `Bytes`, a nullable of either,
an array of either or a shape with either, one line each with its file and the member. Then writing
subagents mark the rows file by file, in disjoint batches of the 49 files. None of them builds.

- [ ] `crates/nvs-stdlib/src/registry.rs:242`: the return mark on `CoreMethod`, with `Derived`,
  `Source` and `Clean(&'static str)`. A row with no mark answers tainted.
  `rule:security/tainted-sources`
- [ ] Every row of the Explore list is marked. A row whose text comes from outside and was plain is a
  bug fixed here: its `about.md` change and its change entry go into stage 6's list.
- [ ] `crates/nvs-types/src/signatures.rs:182`: `contagious_return` reads the mark.
- [ ] `crates/nvs-stdlib/tests/taint_signatures.rs:127`: fails on a text return with no mark, lists
  every `Clean` row, and fails on an empty reason. The allow list at `:21` keeps its rows.
- [ ] `rule:security/tainted-sources` gains one paragraph: the mark, and that no mark means tainted.
  `rule:security/unclassified-parameter-refuses-tainted` names the return side as its pair.
- [ ] The conformance cases that go red are those whose program used a row now marked `Source`. Each
  is fixed in its program, never in its expected output, unless the expected output is the bug.
- [ ] `cargo build`, clippy, `bun nv relink --build` and `bun nv against`. No IR difference.

### 4. every expression form and every codec has a taint row, and a form or a type without one does not compile

**Split:** an `Explore` subagent lists every expression kind of `nvs-syntax` that can give a `string`
or `bytes`, one line each.

- [ ] `crates/nvs-types/tests/taint_every_form.rs:1`: one row per expression kind, read through an
  exhaustive match in the test so a new kind fails to compile the test. Each row is a program in which
  a tainted operand reaches the result, and the result must be tainted. The forms include
  destructuring, spread, `yield`, a closure capture, `??=`, `?->`, a string-holes literal, `match` and
  a heredoc. It runs once, by name.
- [ ] Each row that compiles clean is a hole, fixed at the expression's check, with its anchor under
  `### State`.
- [ ] *check first*: a decode issue's `path` uses the declared field's key (`crates/nvs-stdlib/src/json.rs:5321`).
  One program decodes a document into `{ counts: array<int> }` with a bad value under a key the
  document chose, and prints the issue's path. If the key text is in it, `Core\Issue::$path` becomes
  `tainted string` (`crates/nvs-types/src/error_lib.rs:222`), and stage 6 lists it.
- [ ] *check first*: `crates/nvs-stdlib/src/db/row.rs:220` and the `format!` calls of `json.rs` not
  yet read. A message that includes input text makes `Throwable::$message` `tainted string`
  (`error_lib.rs:238`). One program for each finding.
- [ ] `cargo build`, clippy, `bun nv relink --build` and `bun nv against`. No IR difference.

### 5. no launderer returns a value its own sink reads as an instruction

- [ ] `crates/nvs-stdlib/tests/launderer_matrix.rs:1`: every launderer on the roster
  (`rule:security/launderer-answers-a-carrier`'s registry guard lists it), given `.`, `..`, `/`,
  `\`, a NUL byte, CR LF and an empty string. Each answer must be safe at the launderer's own sink. It
  runs once, by name.
- [ ] `crates/nvs-stdlib/src/uri.rs:3215`: `encodeComponent` throws a `LogicError` on `.` and `..`.
  Its doc at `:723` says so, and the error names `withQueryParameter`. `rule:security/launderers-are-sink-named`
- [ ] Each matrix cell that is unsafe at its own sink is fixed in the launderer, as `encodeComponent`
  is.
- [ ] Each cell that is unsafe only at another sink, such as the terminal escape's `..` at a path
  sink, gets one gap record under `data/gaps/` that names the carrier design as the fix.
- [ ] `tests/hostile/`: one attack, a request whose route capture is `..`, encoded with
  `encodeComponent` into an outbound URL's path. The call throws.
- [ ] `cargo build`, clippy, `bun nv relink --build` and `bun nv against`. No IR difference.

### 6. the features that moved have their proofs, their pages and their change entries

- [ ] `website/src/data/changes.json:1`: `changed` for `Core\Uri::encodeComponent`, for each row
  stage 3 moved from plain to tainted, and for `Core\Issue` and `Throwable::$message` if stage 4
  changed them.
- [ ] The `about.md` and help of each: reread against what was built and stamped with
  `bun nv site --stamp`. The page on `tainted` says in one sentence that text from a `Core` member is
  tainted unless the member says otherwise.
- [ ] `bun nv proofs --owed --goal taint-fail-closed`, and every proof it names is paid.
- [ ] `bun nv site --check stale`, `changes` and `review` are green.

### 7. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of [prose.md](../agent/prose.md) § *A newcomer reads it* over the texts
  stage 6 wrote.
- [ ] `crates/nvs-lsp/tests/latency.rs` runs once, by name, and is green at its bound.
- [ ] `bun nv against` once more on the final tree. Its figures go under `### State`, and
  `.agent-tmp/against/` is deleted.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Nothing is built. The figures of § *Why* were read on 2026-11-22 from `main` at `06cb4bfb05`, and the
`encodeComponent` probe ran on the release binary of that day.

Sessions per stage: none yet.

### Next

Stage 1 from its first item, at `tools/nv/cmd/against.ts:1`. Read `tools/nv/cmd/start-cost.ts` for how
a `bun nv` command finds and runs the release binary, then write the record and the compare. No test
is expected to go red.

### Backlog
