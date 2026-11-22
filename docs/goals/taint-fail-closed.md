---
looper: 1
title: Text is tainted unless the checker proves it clean
summary: "Reached when one exhaustive classifier decides every taint walk and a type it does not recognise counts as tainted, a Core member's returned text is tainted unless its row says where the text comes from, every expression form and every codec has a taint row, no launderer returns a value its own sink reads as an instruction, the request path's IR is byte-identical and the checker's time is within 3% of its baseline, and the features that moved have their proofs and their website entries."
context: [docs/rules/security/taint-propagation.md, docs/rules/security/tainted-sources.md, docs/rules/security/unclassified-parameter-refuses-tainted.md, crates/nvs-types/src/expr/quals.rs]
stages:
  - id: 1
    title: "the request path's IR and the checker's time have a baseline, and one command compares a build against it"
    status: done
    context: [tools/nv/cmd/start-cost.ts, docs/rules/testing/perf-two-mechanisms.md]
  - id: 2
    title: "one exhaustive classifier decides every taint walk, and a type it does not recognise counts as tainted"
    status: done
    context: [crates/nvs-types/src/expr/quals.rs, crates/nvs-types/src/derive.rs, crates/nvs-types/src/expr/args.rs]
  - id: 3
    title: "a Core member's returned text is tainted unless its row says where the text comes from"
    status: done
    context: [crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/tests/taint_signatures.rs, docs/rules/security/tainted-sources.md]
  - id: 4
    title: "every expression form and every codec has a taint row, and a form or a type without one does not compile"
    status: done
    context: [crates/nvs-types/src/derive.rs]
  - id: 5
    title: "no launderer returns a value its own sink reads as an instruction"
    status: done
    context: [crates/nvs-stdlib/src/uri.rs, docs/rules/security/launderers-are-sink-named.md]
  - id: 6
    title: "the features that moved have their proofs, their pages and their change entries"
    status: done
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

**The flip changes no answer for a type the walks already know.** `text_class` gives every `Ty`
variant the answer today's walks give it, and only a type no walk names changes, to tainted. A row
marked `Derived` answers what it answers today. So a program that compiled before still compiles,
unless it used a source that was not marked. Reason: the goal closes holes, and it must not move the
line for text that was already decided.

**Over-tainting is measured, not guessed.** Tainted text is only refused at a sink where text becomes
an instruction (`rule:security/sink-predicate`). A bound query parameter, a response body, an HTML
hole, a log field and a file's contents all take it. So too much taint shows up in exactly one way: a
program that compiled before no longer compiles. `bun nv against` lists each such program with its
diagnostic. Each one is a hole the flip found, and its line under `### State` names the source. Or it
is a wrong mark or a wrong arm, and it is fixed there. It is never fixed with
`Core\Taint::assertTrusted` in the program. A program that newly needs `assertTrusted` means a
launderer or a mark is missing, and that is the fix.

**`assertTrusted` has a budget.** Stage 1 counts the calls in `docs/examples/`, `tests/hostile/`,
`benches/members/` and `website/snippets/`, which was 48 on 2026-11-22. The count never grows during
this goal. Readers copy these programs, so stage 6 reads each call. A call whose value a launderer, a
checked conversion or a `Clean` row could untaint gets that instead. A call that stays names a case
that has no launderer and cannot have one (`rule:security/assert-trusted`).

**A refusal at a sink names the untaint for that sink.** The `E0401` that a tainted argument gets at
a `Sink` parameter has a help line that names the sink's launderer, such as `Core\IO::within` at a
path. Where the sink has none, the help line names `assertTrusted`. The editor's hover already shows
where the taint entered (`rule:ide/a-hover-shows-where-a-tainted-or-secret-value-came-from`). So a
user is pointed at the exact place to untaint, which is the sink, never the source.

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
differs, every program whose diagnostics differ with both sets, the `assertTrusted` count, and the
median check-time ratio with its pairs. The corpus is `benches/members/` and
`docs/examples/` for the IR, and the 40 largest programs of both for the timing. A base that differs
from itself on a second dump fails the record, so a dump that is not deterministic is found before it
can hide a change. The gate stage deletes `.agent-tmp/against/`.

**A launderer's answer is safe at its own sink.** `Core\Uri::encodeComponent` throws a `LogicError`,
as the runtime's other argument errors do, on an argument that is exactly `.` or `..`, and its doc says so. A query value goes through
`Core\Uri::withQueryParameter` or `buildQuery`, which the error names. Reason: a path segment is the
first use its doc names, and a dot segment changes a URI's structure there. Encoding the dots as `%2E`
does not help, because a URL parser reads `%2e%2e` as `..`. It costs one length test per call. The
same question is asked of every launderer in stage 5's matrix.

**The help at a refused sink is read from where the sink is, not from a list beside the rows.** A
`Core` row's signature carries its class (`MethodSig::core_class`), and the help is decided only
once an argument is refused. A path parameter names `Core\IO::within`, whatever its class. SQL text
in `Core\Db` names a bound parameter and `Core\Db::quoteIdentifier`, because `assertTrusted` there
would push the wrong fix. Any other sink names the `Launder` rows of its own class, with a quick fix
where one static launderer takes the value alone, and a class with none names `assertTrusted`.
Reason: `rule:security/launderers-are-sink-named` puts a sink's launderer on the sink's class, so
the roster already says it. A per-parameter sink kind would rewrite every `Qual::Sink` row for the
same answer. It costs nothing until a diagnostic is built.

**A launderer used at another sink is a documented limit (the user's answer to `d14`, option 1).**
The terminal escape's answer for `..` is `..`, and a path sink takes it. So do `Core\Regex::quote`
and `Core\Uri::encodeFormValue`. The carrier per sink stays declined: the four cases are in
`rule:security/launderer-answers-a-carrier`'s Declined list, their gap records are deleted, and a
path sink keeps `Core\IO::within` as its launderer. `CROSS_SINK` in `launderer_matrix.rs` lists each
with what its answer keeps.

**`encodeComponent` of an empty string stays an empty component.** One empty segment changes no
structure. The hole needs it first in a path and a second segment the attacker chose after it,
which gives `//host`. Throwing on it would break every empty query value and fragment a program
encodes by hand, at run time, which is common. So the matrix counts the empty cell as safe, and
`rule:security/launderer-answers-a-carrier`'s Declined list records it.

**The HTML template's blind spot stays out.** A hole in `href=` or `<script>` is escaped as element
text, and `rule:core-classes/html-template` records that and its Declined list. Closing it reverses a
recorded decision and needs an HTML position model. The user declined a goal for it, so no session
proposes one.

**Only a walk that asks whether text may come from outside reads `text_class`.** Those are
`carries_tainted`, `carries_qualifier`, `keys_from_outside`, `walks_outside_text`,
`field_from_outside`, `classes_reached` and `unqualified_text`. The others keep their own last arm,
because they ask another question of a type. `carries_unchecked`, `unchecked_outside_arrays` and
`all_unchecked` ask whether it is `mixed`. `holds_array` asks whether it may be an array, and
`has_fields` whether it has fields. `untainted_shape_text` and `untainted_field_from_outside` ask
about the written target. `db_reachable` and `json_reachable` are type maps whose last arm already
refuses. The transforms `tainted_result`, `tainted_walks`, `untainted` and `untainted_entries` leave a
type with no `tainted` form unchanged. A contagious member that returns one then refuses a `tainted`
argument, which is the safe direction. `:1741` matches a member's name, not a type. Reason: forcing a
text answer onto a question about array-ness or fields would only move answers the flip must keep.

**`carries_tainted` does not step into a shape.** Stepping in turned every image and PDF result into
`tainted bytes`. The plan shape's `Core\Bytes::Source` field has a `Core\Request\Part` arm, so the
result was tainted even when the program read a file. That is 165 programs. Whether a shape argument's
text reaches a contagious result is a question about what each member writes, and stage 4's rows
answer it. `may_carry_outside_text` is the classifier's full answer and does step in.

**An extension call's result is tainted when a shape argument carries outside text.** Stage 4 found
the hole: `Image::open($upload)->encode()` from a `tainted bytes` upload compiles to plain `bytes`,
and `pixels()` returns bytes the attacker chose. `rule:security/extension-contagion` already says an
extension's result carries every argument's taint, and a component that cleans an upload is the
bypass that rule exists to stop. So an extension call's contagion reads `may_carry_outside_text`,
which steps into a shape, and the image and PDF components' terminals return `tainted bytes`. A
program that reads a file into an image then writes `tainted bytes` too, because the source's type
has an upload arm and the qualifier has no generic form. That over-taint is accepted: a response
body, storage and a file all take `tainted bytes`, and security outranks simplicity. A `Core` row's
shape argument stays out of `carries_tainted`, since no row writes a shape argument's text into its
result. `MethodSig::reads_whole_arguments` (a signature with a `contagious_return` and no
`core_class`) is that extension. `carries_contagion` reads `may_carry_outside_text` there, and the
admission clears `tainted` with `quals::untainted_toward`, which follows the declared type, since a
shape's fields compare invariantly and a manifest cannot write `tainted`.

**`Core\Bytes::at` is `Neutral`, and `Core\Bytes::join`'s parts are contagious.** `at` answers a
`uint`, which has no `tainted` form, so it refused every upload for nothing, as `length` does not.
`join` refused an `array<tainted bytes>` while `Core\Str::join` took one. Neither costs anything at
run time. `Novis\Image\Font::fromBytes` takes `tainted bytes`, since a font may be an upload and the
extension's result is tainted anyway.

**The return mark is a field on every row, and the checker reads it where it seeds the row.**
`CoreMethod::text` is a `TextOrigin`: `Unmarked`, `Derived`, `Source` or `Clean(reason)`. A row that
returns no text keeps `Unmarked`, and `taint_signatures.rs` fails on a mark on such a row. A shared
row builder writes `TextOrigin::Clean(..).on(&return_ty)`, which is `Unmarked` for a non-text return.
`nvs_types::core_lib`'s `return_of` writes `tainted` on the return type of an `Unmarked` or `Source`
row, so `MethodSig::contagious_return` needs no change: the return type already says it. `Derived`
and `Clean` check the same way, and `Clean` only adds the reason a reviewer reads. Reason: one read at
seeding costs nothing per call, and a field cannot be forgotten the way a roster beside the rows can.

**A child process's output is `Source`.** `Core\Process\Result::stdout`/`stderr` and
`Core\Process\Handle::readStdout`/`readStderr` answer `tainted bytes`. The program chose the command,
but not what `git`, `curl` or a compiler prints about data it reads, so `Core\IO`'s reason for clean
reads does not carry over. A program writes `tainted bytes` on the variable that keeps it. So are
`Core\Db\Connection::serverVersion` and `Core\Ldap\Connection::whoami`: text a server sends.
`Core\Db\Plan\Step::reason` and `::sql` are `Clean`: the planner writes them from its own sentences
and the program's schema. `Core\Http\Socket::protocol` is `Clean` too: the handshake refuses a
subprotocol the call did not offer (`transport::settled`), so the name is one the program wrote.

**What stops it.** One attempt per item, and two sessions per stage, counted in `### State`. At two
sessions a stage's open items move to `### Backlog` as declined, each with its reason, and a security
item among them gets a gap record. No stage gains an item. A `bun nv against` that is over budget
after the memo is a stop: the session records the figures under `### State` and writes `BLOCKED`.

**Lanes.** Stage 1 comes first, because its base must be recorded before any change. Stages 2 and 4
share `quals.rs` and `derive.rs`, and run in order. Stage 3 touches `crates/nvs-stdlib` and may run
beside stage 2 as a side goal once stage 1 is done. Stage 5 touches `uri.rs` and its own test, and may
run beside either. This goal starts after `safety-holes`. Gate: `docs/goals/safety-holes.md` is
deleted.

**A new expression kind fails at the parser and then at the test.** `ExprKind` is `non_exhaustive`,
so a match in `nvs-types`'s tests cannot be exhaustive. `ExprKind::name` in `nvs-syntax` matches
every variant with no catch-all arm and returns its entry of `ExprKind::NAMES`. A new variant does
not compile until it has a name, and `taint_every_form` then fails naming the kind with no row.
Reason: removing `non_exhaustive` from the AST for one test would move a crate boundary the other
enums keep.

**An error's message is `tainted`.** `Core\Json::decode` writes a repeated key of the document into
its message (`json/reader.rs:392`). `Throwable::$message` and the constructor's `$message` are
`tainted string`, so wrapping a message still compiles. Making each runtime message prove it leaves
input out would be an invariant held by attention. `Core\Issue`'s `path` and `message` stay plain:
`Segment::Key` borrows only the contract's keys, and `describe` returns fixed words.
`rule:security/tainted-sources` records it.

**A `Core` class whose `toString` returns `tainted` text is tainted.** `.`, an interpolation and
`as string` render through `toString` with no call written. `registry::renders_tainted_text` reads
the row, and `text_class` and `instance_carries_no_text` read it. Today that is `Core\Ldap\Filter`
and `Core\Ldap\Dn`.

## Stages

### 1. the request path's IR and the checker's time have a baseline, and one command compares a build against it

- [x] `tools/nv/cmd/against.ts:1`: `--record` and the compare, as the standing decision gives them.
  The help text follows `docs/agent/conventions.md` § *An `nvs` command*. The command finds programs
  with `Glob`-style walks over the two folders, and uses one `nvs check` and one `nvs run --dump-ir`
  per program.
- [x] `tools/nv/test/against.test.ts:1`: a record of the current binary, then a compare of that same
  binary against it, prints no IR difference, no diagnostic difference and a ratio between 0.9 and
  1.1. It runs once, by name.
- [x] `bun nv relink --build`, then `bun nv against --record` on the commit before stage 2. The
  commit, the corpus size and the `assertTrusted` count go under `### State`.
- [x] `docs/agent/commands.md`: one line for `bun nv against` in its table.

### 2. one exhaustive classifier decides every taint walk, and a type it does not recognise counts as tainted

**Split:** an `Explore` subagent lists every variant of `Ty` (`crates/nvs-types/src/ty.rs`), one line
each, before the classifier is written.

- [x] `crates/nvs-types/src/expr/quals.rs:1`: `text_class` and `TextClass`, as the standing decision
  gives them. `rule:security/taint-propagation`
- [x] `quals.rs`: every catch-all arm of § *Why* reads `text_class`. Each keeps its direction: those
  at `:260`, `:305`, `:315`, `:327`, `:361`, `:452`, `:503`, `:525`, `:554`, `:654`, `:680`, `:897`,
  `:931`, `:994`, `:1418`, `:1741`, `:1906`. `:1873` already answers tainted and stays.
- [x] `crates/nvs-types/src/derive.rs:2385` (`classes_reached`), `:2408` (`unqualified_text`),
  `:2529` (`db_reachable`) and `:2597` (`json_reachable`) read `text_class`. `classes_reached` steps
  into a class's type arguments. *check first*: `db_reachable` lists no `SecretString`,
  `SecretBytes` or `SecretTaintedString`, and `json_reachable` no `TaintedBytes` or `SecretBytes`.
  Write one program for each that should not compile.
- [x] `crates/nvs-stdlib/src/registry.rs:4461` and `:4313`: `carries_tainted_bytes` and
  `holds_cells_of_its_source` keep their lists. An unlisted `Core` class answers `Tainted` unless the
  registry lists it as `NoText`.
- [x] `crates/nvs-types/tests/text_class.rs:1`: an unknown `Core` class, a generic wrapper, a shape
  in an array in a nullable, and `jsonAs<{ a: { b: User } }>` each count as tainted. It runs once,
  by name.
- [x] The rule fragments `taint-propagation.md` and `tainted-qualifier.md` say "clean is the proof"
  in one paragraph each, and the `quals.rs` module doc says it once.
- [x] `cargo build`, `cargo clippy --all-targets -- -D warnings`, then `bun nv relink --build` and
  `bun nv against`. No IR difference and no diagnostic difference, since this stage changes no known
  type's answer. The ratio goes under `### State`.

### 3. a Core member's returned text is tainted unless its row says where the text comes from

**Split:** an `Explore` subagent lists every row that returns `Str`, `Bytes`, a nullable of either,
an array of either or a shape with either, one line each with its file and the member. Then writing
subagents mark the rows file by file, in disjoint batches of the 49 files. None of them builds.

- [x] `crates/nvs-stdlib/src/registry.rs:242`: the return mark on `CoreMethod`, with `Derived`,
  `Source` and `Clean(&'static str)`. A row with no mark answers tainted.
  `rule:security/tainted-sources`
- [x] Every row of the Explore list is marked. A row whose text comes from outside and was plain is a
  bug fixed here: its `about.md` change and its change entry go into stage 6's list.
- [x] `crates/nvs-types/src/signatures.rs:182`: `contagious_return` reads the mark.
- [x] `crates/nvs-stdlib/tests/taint_signatures.rs:127`: fails on a text return with no mark, lists
  every `Clean` row, and fails on an empty reason. The allow list at `:21` keeps its rows.
- [x] `rule:security/tainted-sources` gains one paragraph: the mark, and that no mark means tainted.
  `rule:security/unclassified-parameter-refuses-tainted` names the return side as its pair.
- [x] The conformance cases that go red are those whose program used a row now marked `Source`. Each
  is fixed in its program with the sink's launderer or a checked conversion, never with
  `assertTrusted` and never in its expected output, unless the expected output is the bug. A case
  whose row was marked `Source` wrongly gets `Derived` or `Clean` instead, and the case stays as it
  was.
- [x] `cargo build`, clippy, `bun nv relink --build` and `bun nv against`. No IR difference. Each
  program whose diagnostics differ is either a hole, named under `### State` with its source, or a
  wrong `Source` mark, fixed at the row. The `assertTrusted` count has not grown.

### 4. every expression form and every codec has a taint row, and a form or a type without one does not compile

**Split:** an `Explore` subagent lists every expression kind of `nvs-syntax` that can give a `string`
or `bytes`, one line each.

- [x] `crates/nvs-types/tests/taint_every_form.rs:1`: one row per expression kind, read through an
  exhaustive match in the test so a new kind fails to compile the test. Each row is a program in which
  a tainted operand reaches the result, and the result must be tainted. The forms include
  destructuring, spread, `yield`, a closure capture, `??=`, `?->`, a string-holes literal, `match` and
  a heredoc. It runs once, by name.
- [x] Each row that compiles clean is a hole, fixed at the expression's check, with its anchor under
  `### State`.
- [x] *check first*: a decode issue's `path` uses the declared field's key (`crates/nvs-stdlib/src/json.rs:5321`).
  One program decodes a document into `{ counts: array<int> }` with a bad value under a key the
  document chose, and prints the issue's path. If the key text is in it, `Core\Issue::$path` becomes
  `tainted string` (`crates/nvs-types/src/error_lib.rs:222`), and stage 6 lists it.
- [x] *check first*: `crates/nvs-stdlib/src/db/row.rs:220` and the `format!` calls of `json.rs` not
  yet read. A message that includes input text makes `Throwable::$message` `tainted string`
  (`error_lib.rs:238`). One program for each finding.
  - [x] The finding: `Core\Json::decode`'s repeated-key message quotes the key. `row.rs:220` and the
    issue messages are clean.
  - [x] `Throwable::$message` and the constructor's `$message` are `tainted string`; the ten examples
    write `tainted string`; `taint_every_form` has the program.
  - [x] Every conformance case, hostile test, snippet and app that keeps a message in a plain
    `string` writes `tainted string`. `bun nv suite` names them, and this checkpoint requires the run.
- [x] `cargo build`, clippy, `bun nv relink --build` and `bun nv against`. No IR difference. Each
  program whose diagnostics differ is either a hole, named under `### State` with its source, or a
  wrong `Source` mark, fixed at the row. The `assertTrusted` count has not grown.

### 5. no launderer returns a value its own sink reads as an instruction

- [x] `crates/nvs-stdlib/tests/launderer_matrix.rs:1`: every launderer on the roster
  (`rule:security/launderer-answers-a-carrier`'s registry guard lists it), given `.`, `..`, `/`,
  `\`, a NUL byte, CR LF and an empty string. Each answer must be safe at the launderer's own sink. It
  runs once, by name.
  - [x] a row per launderer, called through its linked symbol, and a check per sink
  - [x] the roster is read from the registry, and a launderer with no row fails
  - [x] run once, by name: the one red row was `encodeComponent`'s `.` and `..`
- [x] `crates/nvs-stdlib/src/uri.rs:3215`: `encodeComponent` throws a `LogicError` on `.` and `..`.
  Its doc at `:723` says so, and the error names `withQueryParameter`. `rule:security/launderers-are-sink-named`
- [x] Each matrix cell that is unsafe at its own sink is fixed in the launderer, as `encodeComponent`
  is.
- [x] Each cell that is unsafe only at another sink, such as the terminal escape's `..` at a path
  sink, gets one gap record under `data/gaps/` that names the carrier design as the fix.
- [x] `crates/nvs-types/src/expr/args.rs:815` (`check_arg_admitting_quals`): the `E0401` at a
  `Sink` parameter has a help line that names that sink's launderer, read from the launderer roster.
  A sink with no launderer names `Core\Taint::assertTrusted`. *check first*: some sinks may name one
  already, so list which do not before writing it. One `.nvst` case under `tests/conformance/reject/`
  for a path sink and one for a regex pattern. `rule:security/launderers-are-sink-named`
- [x] `tests/hostile/`: one attack, a request whose route capture is `..`, encoded with
  `encodeComponent` into an outbound URL's path. The call throws.
- [x] `cargo build`, clippy, `bun nv relink --build` and `bun nv against`. No IR difference. Each
  program whose diagnostics differ is either a hole, named under `### State` with its source, or a
  wrong `Source` mark, fixed at the row. The `assertTrusted` count has not grown.

### 6. the features that moved have their proofs, their pages and their change entries

- [x] `website/src/data/changes.json:1`: `changed` for `Core\Uri::encodeComponent`, for each row
  stage 3 moved from plain to tainted, and for `Core\Issue` and `Throwable::$message` if stage 4
  changed them. Stage 3 moved six: `Core\Process\Result::stdout` and `::stderr`,
  `Core\Process\Handle::readStdout` and `::readStderr`, `Core\Db\Connection::serverVersion` and
  `Core\Ldap\Connection::whoami`. Each one's `about.md` says its text is `tainted`, and how to keep
  it (`tainted bytes` on the variable, a checked conversion, or the sink's launderer).
- [x] The `about.md` and help of each: reread against what was built and stamped with
  `bun nv site --stamp`. The page on `tainted` says in one sentence that text from a `Core` member is
  tainted unless the member says otherwise.
- [x] Every `Core\Taint::assertTrusted` and `assertTrustedBytes` call in `docs/examples/`,
  `tests/hostile/`, `benches/members/` and `website/snippets/` is read. **Split:** an `Explore`
  subagent lists them, one line each. A call that a launderer, a checked conversion or a `Clean` row
  can replace is replaced. A call that stays has a reason naming a case with no launderer. The count
  before and after goes under `### State`.
- [x] `bun nv proofs --owed --goal taint-fail-closed`, and every proof it names is paid.
- [x] `bun nv site --check stale`, `changes` and `review` are green.

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

The against base is in `.agent-tmp/against/base/`, from `1ec4cda0d4`, and stays until stage 7.
Last `bun nv against`: 172 programs' IR differs only in source positions (the image component's
source gained a line), with no instruction changed. Ten compile only in the new build, because they
write `tainted bytes`, and two in neither (wording). Median check-time ratio 1.005.

`assertTrusted`: 44 calls (`against` counts 48, with comments), none replaced. 18 show the method
itself. The rest read the program's own text back through a decoder (Passkey challenge,
`Core\Serialize`, a signed cookie or link), which no launderer covers. The three in
`tests/hostile/core/Json/encode/02-...` stay, since `Core\Response\Stream::write` does not take
tainted text.

`changes.json` has six entries and 70 `unseen` records (tests that only gained a `tainted` type).
238 stale texts are reread and stamped, and the newcomer review of 36 changed texts is done.
`site --check stale`, `changes` and `review` are green. Decision `d14` is closed (option 1).

Perf: all 1062 perf-stale features are recorded. 67 `Core\Ldap` benches declared `calls 0` (or 2
and 4 for `changes` and `search`), from a figure taken on 2026-10-08. Since `4beafef130` every `Core`
call passes the call probes and counts as a call, so each declaration now states its measured count.
The base build counts the same, so the change is not this goal's. Four growth failures
(`BigInt::compareTo`, `IO\Metadata::modifiedAt`, `Test::assertSame`, `Session::get`) passed on a
second run. `bun nv proofs --gate --limit 0` is green over the whole roster, and stage 6 is done.

Sessions per stage: 1: one. 2: one. 3: two. 4: two. 5: two. 6: two.

### Next

1. Stage 7. Its newcomer review is done for every text stage 6 wrote (36 keys, stamped). Run
   `crates/nvs-lsp/tests/latency.rs` by name (`cargo test --test latency`), then `bun nv relink
   --build` and `bun nv against` once more. Record the figures under `### State`, then delete
   `.agent-tmp/against/` and run `bun nv close --goal taint-fail-closed`. The full gate runs at a
   later goal of the chain.

Tests expected red: a test outside `tests/conformance` that pins the exact text of an `E0401` at a
`Core` sink, and a test that pins the help text of a process output, `serverVersion`, `whoami` or
`Image::raw` (none found by grep).

### Backlog

- **An array literal argument with a `tainted` element is refused at a contagious array
  parameter.** `Core\Str::join([$tainted, 'x'])` and `Core\Bytes::join([...])` give `E0401` at the
  element, on the base build too, while the same array in a variable is admitted. The literal is
  inferred against the declared `array<string>` before the admission clears the bit
  (`args.rs` `check_arg_admitting_quals`). Widening the expectation risks typing a plain literal as
  tainted, so it needs its own look.
- **A help card's signature shows the registry's type, not the checked one.** The card of
  `Core\Process\Result::stdout` prints `bytes` in its signature, while the checker types the result
  `tainted bytes` (an `Unmarked` or `Source` row gets `tainted` in `core_lib`'s `return_of`). Each
  card's `ret` line now says `tainted`. The signature line needs the same answer as `return_of`.
- **`Core\Process::run`: `PATH` or not.** Its `about.md` says a name such as `git` is looked up in
  the folders of `PATH`, and its help says `PATH` is not searched. One of the two is wrong. It is not
  this goal's, since no taint moved.
- **`shape as string` compiles.** `(await $h) as string` has no diagnostic, though a shape has no
  `Stringable` and the conversion always throws when it runs. It is a type-check gap, not a taint
  hole, since no text is ever produced. `require_stringable_object` in
  `crates/nvs-types/src/expr/operators.rs` returns early for a non-class type.
