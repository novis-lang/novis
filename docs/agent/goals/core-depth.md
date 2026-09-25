---
milestone: M4S
---
# Loop goal 1 — `Core`'s pure half, finished

Finish **M4S** — [docs/plan/m4s.md](../../plan/m4s.md) is the scope and this file does not restate it.
Every member of [docs/spec/01-core-library.md](../../spec/01-core-library.md) §§ 1–13 that does not need a
capability, a reactor, a driver or an open handle **runs, is cased, and is claimed by a row in the
migration table.**

This is goal `core-depth` of the parity program ([goals/README.md](README.md)), and it is first for a reason that
compounds the way M4's operator table did: **every goal above this one is written against `Core`.** A
driver returns `Core\Time\Instant`s, the server returns `tainted string` from `Core\Request`, an isolate
copies a `Core\ObjectMap`. A member that is registered but wrong, or registered and untested, is a defect
that gets built on five times before anyone trips over it.

## What "finished" means here, and why it is checkable

M4S Part I is already **registered** — `crates/nvs-stdlib/tests/spec-members-outstanding.txt` holds no
keys, which is this project's definition of that word. Registered is not finished. Three things separate
them, and each has a tool that counts it so no session re-derives one:

1. **Depth.** `python tools/gaps.py` prints the median conformance cases per member per class, thinnest
   first, and its `floor` column is the worst member of each. A class whose median is 5 and whose floor is
   2 has members nothing has ever really exercised. This goal raises the **floor to 3 for every class**,
   which is deliberately not a median: a median rises by writing more cases for members that already have
   some, and that is the metric a corpus grows around rather than into.
2. **The passes that exist only on paper.** Four compiler-recognized attributes, the intrinsic-folding
   pass, the OpenAPI emitter and `rule:programs/implementing`'s program enumeration are all specified, none implemented.
   `nvs_types::derive::ATTRIBUTES` at [derive.rs:77](../../../crates/nvs-types/src/derive.rs) is the
   closed list they join, and it holds five names where the ADRs name twelve.
3. **The migration rows.** `python tools/check-migration.py` was at **25%** (291 of 1151 functions) when
   this goal was written. This goal takes the domains M4S owns — strings, arrays, numbers, dates, regex,
   encoding, JSON, paths — to a row each. That is not a documentation slice: a row is where the audit
   happens, and three functions reached a full member-by-member review with no home at all precisely
   because prose was carrying the argument.

**A member is *done* when it runs, its errors are asserted, and PHP agrees where the spec says PHP is the
oracle.** A registered signature with a body that throws is not done, and `gaps.py`'s *unasserted error
paths* list — 68 sites when this was written — is the inventory of the middle one.

## Stage 0 — the catch-up, and it is one item

1. **`rule:programs/implementing`'s program enumeration exists.** `Core\Program::implementing<T>()` expands at compile
   time to an array literal of `new` expressions, one per non-abstract class implementing `T`, **sorted by
   fully-qualified name** so the order never depends on filesystem enumeration; a class with no
   no-argument constructor is a diagnostic naming it. `nvs-hir` has §§ 1–2 — `AutoloadMap::build` at
   [autoload.rs:162](../../../crates/nvs-hir/src/autoload.rs) and the `resolve` beside it — and nothing
   walks the resulting program to answer "which classes implement this interface".

   **It is Stage 0 because three later items are the same scan.** `#[Route]`'s table (item 6),
   `#[Command]`'s table (item 8) and the OpenAPI emitter (item 9) are all "filter `rule:programs/implementing`'s
   enumeration", and `rule:routing/table-is-opt-in` says so outright. Writing any of them first means writing the walk
   three times and then unifying it. `crates/nvs-hir/src/autoload.rs`,
   `crates/nvs-hir/src/hierarchy.rs` (`implements_interface` is already there),
   `crates/nvs-types/src/expr/calls.rs` for the call-site expansion.

## Stage 1 — the floor

M4's entire acceptance list, inserted mechanically by `goal-switch.py` and **never traded for anything
above it.** A session that finds it has to change a floor fixture's expected output has found a bug in its
own slice. Nothing in this goal *writes* `nvs-ir` lowering or `nvs-codegen`, so a failure there is a real
regression and never a scope question — with one declared exception, which owns what M4 left standing
there rather than scheduling any of it.

15. **M4's seventeen lowering refusals keep an owner across the goal switch.**
    The ratchet is what lets them stand.
    `nvs-ir` type-checks each of these shapes and then refuses it. M4's item list anchored
    every one, and the switch carried M4's *check* into this stage without carrying the *items* that made
    it green, so `every_refusal_is_a_diagnostic_or_decided` began failing on sites nothing in this tree
    had touched. This item is that inventory, so the gate can tell a carried gap from a new one.
    Numbered fifteenth because the fourteen below keep the numbers this goal's TOML comments and
    `python tools/holes.py --item N` already use.

    - `crates/nvs-ir/src/lower/call.rs:773` and `:1104` — an argument list through a `callable` that is
      not plain positional, and a by-reference argument from something other than a bare local or a
      compile-time-known property.
    - `crates/nvs-ir/src/lower/control.rs:744`, `:978` and `:989` — a `switch` label at a representation
      other than the subject's own, a `foreach` key binding outside `rule:types/arrays`'s one stored key type,
      and a `foreach` over an `rule:iteration/two-interfaces` `Iterable`/`Iterator` subject.
    - `crates/nvs-ir/src/lower/convert.rs:574` — a truthy condition over a representation the conversion
      slice does not carry.
    - `crates/nvs-ir/src/lower/exception.rs:32` — `throw` on a representation that is not an object.
    - `crates/nvs-ir/src/lower/expr.rs:1670`, `:2548`, `:2737`, `:3800` and `:3836` — a `match` label at a
      foreign representation, an instance call and a static call with no resolved target in the
      typed-expression table, `instanceof` against a subject that cannot hold an object, and `clone` on
      one.
    - `crates/nvs-ir/src/lower/mod.rs:2276`, `:2705` and `:2792` — an array-element write through a shape
      that is not a bare local, a compile-time-known property or a static property, and the two declared
      type lists that do not yet spell every atom `rule:types/grammar` allows.
    - `crates/nvs-ir/src/lower/stmt.rs:269` and `:1465` — a local declaration shape the control-flow
      slice does not lower, and `unset` on anything but an array element with an explicit subscript.

    **This goal does not close them and is not judged on them.** It is `Core`'s pure half, and no `Core`
    member reaches one of these shapes; a session that finds itself editing a file above has taken the
    wrong slice. What makes standing acceptable is the second half of the same gate: `CEILING` in
    `crates/nvs-ir/tests/refusals.rs:66` holds the total at seventeen and **may never rise**, so a
    refusal added beside a carried one fails the run even though attribution claims its file. Each
    closes the way M4 required — it lowers, or a diagnostic naming its rule refuses it, never a panic
    however well worded — in the first goal that writes `nvs-ir` lowering again.

    **The recurrence is the switch's bug, not this file's.** `tools/goal-switch.py` carries a goal's
    `[[check]]` blocks forward and its unclosed items not at all, so a carried check whose green depends
    on an item list arrives without its basis; until that is fixed, every goal in
    `docs/agent/goals/` inherits this paragraph by hand.

## Stage 2 — the four attribute passes, which are one pass

Grouped because they are literally one file set: a name on `ATTRIBUTES`, a recognizing pass in
`nvs-types`, and a table the compiler carries. `#[Json\Derive]` is the worked example already on disk —
[derive.rs](../../../crates/nvs-types/src/derive.rs) is 175 lines to the pass and it is the shape to copy,
not an example to read for inspiration.

2. **`#[Json\Derive]`'s three open gaps close.** Its own module doc lists them: § 2's codec-reachable type
   test is not applied, § 7's refusal of a class that hand-writes both halves is not applied, and a field
   whose declared type has no decoder is `CodecTy::Opaque` refused at the `decodeAs<T>` that runs rather
   than at the declaration that wrote it. All three are diagnostics at the declaration.
   [derive.rs:40](../../../crates/nvs-types/src/derive.rs) is where they are written down.
   `rule:core-classes/derive-field-list` and `rule:core-classes/derive-generates-what-is-missing`.
3. **`#[Route]` builds a table while compiling.** `rule:routing/route-attribute`, `rule:routing/path-grammar`, `rule:security/route-capture-is-laundered-by-its-type` and `rule:routing/table-is-opt-in`: matched nominally like every other name on `ATTRIBUTES`, the path grammar of § 2, a parameter's
   type coming from the method and being what launders it, and the table built by Stage 0's scan. Its
   three compile errors are the whole point of doing it here — a duplicate route, a `{param}` with no
   matching method parameter, an unknown literal `url()` name.
4. **`#[Query]` and `#[Access]` join it**, and with them `rule:routing/the-servers-match-dispatches-nothing`'s four further compile errors: a
   `{name?}` outside the last position or bound to a parameter with no default (§ 4), a capture or
   `#[Query]` parameter whose type is outside § 3's list, and a `url()` key that is neither a capture nor
   a declared `#[Query]` parameter (§ 6). Same pass, same file, one group with item 3.
   `rule:routing/the-servers-match-dispatches-nothing`.
5. **`Core\Router::url()` and `::urlAbsolute()` are launderers over that table** — and only those two.
   `::match` and `::methodsFor` are goal `server`'s, because a match needs a request. Splitting the class this
   way is [01-core-library.md](../../spec/01-core-library.md) § *Milestones*'s own instruction and not this
   goal's invention.
6. **`#[Command]`/`#[Option]`/`#[Argument]` build the command table.**
   `rule:tooling/commands-are-compiled`, with its own three compile errors: a
   duplicate command name, two options sharing a spelling, an `#[Option]` on a parameter with no
   conversion from `string`. **The table only** — `Core\Command::run`, the generated `--help` and the
   completions are goal `core-part-ii`'s, since neither argv nor a terminal is reachable before capabilities exist.

## Stage 3 — the intrinsic-folding pass

7. **`rule:expressions/intrinsic-literals`'s closed list folds.** A short list of `Core`
   members take an argument that is really a small program — a regex pattern, a URI, a date-format string,
   a format string — and when that argument is a compile-time constant the compiler validates it during
   checking and *prepares* what the runtime would have built on first use. Nothing of this exists: three
   modules mention the ADR in a doc comment ([format.rs:46](../../../crates/nvs-stdlib/src/format.rs),
   [time.rs:73](../../../crates/nvs-stdlib/src/time.rs),
   [regex.rs:29](../../../crates/nvs-stdlib/src/regex.rs)) and no pass reads any of them.

   **The soundness rule is absolute and is the item's whole shape**: the prepared path and the runtime
   path share one implementation, so folding can never produce a different answer, only an earlier one. A
   fold that needs its own copy of the parser is the wrong design, and the check that says so is a case
   asserting the same malformed literal is a compile error *and*, behind a non-literal, the identical
   runtime throw. Its own group, because it is the only item that reaches into `nvs-types`' constant
   folding and `nvs-stdlib`'s parsers at once.

## Stage 4 — the emitter

8. **`nvs build --openapi` writes a deterministic 3.1 document.**
   `rule:routing/api-document-is-generated-from-the-route-table`: § 1's "what supplies what" is
   the whole design — the route table supplies paths and parameters, `#[Json\Derive]`'s field list
   supplies schemas, and `#[Api]` supplies **only what the types cannot say**. `Core\Api` joins
   `ATTRIBUTES` and the four contradiction cases become compile errors.
9. **`nvs api diff` is the same slice**, not a follow-up. The classification is mechanical over two
   emitted documents, so it costs a comparison rather than a design — § 4 is the gate it implements.
   `crates/nvs-cli/src/main.rs:175` is where a subcommand is added and `crates/nvs-cli/src/info.rs` is
   the shape a non-running subcommand takes.

## Stage 5 — the depth pass, and the qualifier classification

10. **Every `Core` class's conformance floor reaches 3.** Take the group from `python tools/gaps.py`'s
    right-hand column, which names the three thinnest members of each class with their anchors; the
    thinnest classes print first and that is the order. This is the one item in the program that a session
    may always fall back to when its own group is blocked, and it is the reason the corpus reaches M4's
    own 1000-case figure without anybody writing cases for their own sake.
11. **Every `Core` member's error paths are asserted.** `gaps.py`'s *unasserted error paths* section is
    the list — 68 sites, 65 of them `Fault::fatal`. **Judge before writing**: a `Fault::fatal` may be an
    internal invariant no program can reach, in which case the answer is a comment saying so at the site
    rather than a case; a `thrown` is a boundary a case can catch and echo. The two in
    `crates/nvs-stdlib/src/csv.rs:512` and `crates/nvs-stdlib/src/test.rs:692` are the second kind.
12. **The per-parameter qualifier classification is complete.**
    `rule:security/unclassified-parameter-refuses-tainted`: an unclassified
    `string`/`bytes` parameter refuses `tainted`, and `nvs-stdlib`'s own suite fails on any member that
    ships without a classification. m4s.md calls this "part of building §§ 1–12 rather than a separate
    slice" — it is listed here because the members are built and the pass is what is left.
    [registry.rs:490](../../../crates/nvs-stdlib/src/registry.rs) is what a member's row may say.

## Stage 6 — the string header

13. **`Core\Str`'s grapheme count is lazily cached, and a concatenation corrects the boundary in O(1).**
    `rule:types/string-is-utf8` decided the grapheme cluster is the unit and
    `nvs_stdlib::granularity` seams it; that ADR's *Consequences* names the cached count as still owed and
    m4s.md places it here. It belongs in `NvsStr`'s header at
    [string.rs:250](../../../crates/nvs-runtime/src/string.rs), beside the builder at `:311`.
    `a_grapheme_index_costs_more_than_a_code_point_index` in `benches/abi-probe` is the guard that already
    exists and must stay green — this item makes the *repeat* index cheap, never the first one.

## Stage 7 — the migration rows

14. **Every PHP name in M4S's domains has a row.** `python tools/check-migration.py --report` lists what
    is open; the domains this goal owns are strings, arrays, numbers, dates, regex, encoding, JSON and
    paths. A row is `member`, `language` or `dropped` and each has a rule:
    [02-php-migration.md](../../spec/02-php-migration.md) § *How to read a row* is that rule and this file
    does not restate it. **A cell that is exactly one `Core` member spelling is the rename `nvs convert`
    applies**, so a cell naming two members or a rewrite must carry its rule id from
    `rule:tooling/convert-three-tables`.

    **Nothing here is guessed to make a number move.** A name with no home is a finding, not a `dropped`
    row: three of them turned out to be real gaps last time somebody looked.

## Stage 8 — the digest roster

16. **`Core\Digest` carries the algorithms a program written now actually names.** Six cases ship —
    `Crc32`, `Md5`, `Sha1`, `Sha256`, `Sha384`, `Sha512` — against PHP's roughly sixty, and the gap that
    matters is not the long tail. [01-core-library.md](../../spec/01-core-library.md) § 11's table is the
    roster's home and lands first. `crates/nvs-stdlib/src/hash.rs:142` is the enum,
    `crates/nvs-stdlib/src/hash.rs:386` its dispatch, and `crates/nvs-stdlib/src/hash.rs:318` the ordinal
    read every stream's slot goes through — bare rather than linked so `python tools/holes.py --item 16`
    lists them. Numbered sixteenth for item 15's reason: the fifteen below keep the numbers this goal's
    TOML comments and that tool already use.

    **Three cost no crate at all.** `Sha224`, `Sha512_224` and `Sha512_256` are already in the pinned
    `sha2 0.10.9` — one `DIGEST` case, one `DigestKind` variant, one `kind_of` arm and one `digest_of`
    arm each. **`Sha512/256` is the one worth arguing for**: faster than SHA-256 on 64-bit hardware,
    structurally immune to length-extension, published by NIST, and today unspellable.

    **`Blake3` is already decided and is not yet free.** [Cargo.toml:62](../../../Cargo.toml) declares it
    and `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` commits the artifact cache to it, but
    nothing consumes it, so it is absent from `Cargo.lock` and this is the slice that resolves it. It does
    not implement `digest 0.10`'s traits without `traits-preview`, so it takes its own `digest_of` arm and
    does **not** ride the `mac!` macro; and HMAC-BLAKE3 is a construction nobody uses, because BLAKE3's
    keyed mode is native — so it stays outside `STRONG` until a keyed member is designed rather than
    being bent into `hmac`. It is also the one algorithm PHP cannot compute at all.

    **`sha3` and `crc32c` are the two that earn a crate.** All four SHA-3 cases ride the same `digest`
    trait set into `digest_of` *and* `hmac_of` with no hand-fitting, and `crc32c` is the checksum S3 and
    GCS stamp objects with, so it is interop this runtime will meet. **The rest stays out.** `xxh*`,
    `murmur3*`, `fnv1*`, `adler32` and `joaat` are table hashes, and putting one in the enum that holds
    `Sha512` is the confusion `StrongDigest` exists to prevent; `md2`, `md4`, `ripemd*`, `whirlpool`,
    `tiger*`, `snefru*`, `gost*` and `haval*` are dead everywhere but a compatibility matrix.

    **The ordinals are ABI** — `kind_of` at `:318` reads them back out of a stream's slot — so a case is
    appended at 6 and up and the list is never reordered. Widening `STRONG` is backward compatible and
    narrowing it is not, which is the only direction this roster grows. `DIGEST`'s doc comment currently
    claims its cases are "ordered weakest first so that `STRONG` is a contiguous tail"; that stops being
    true here, and correcting it is part of the same edit rather than a follow-up.

    **What it spends:** nothing per request — a digest state is stack-held and released before the member
    returns — and binary size for each algorithm's own tables. That is priority 5 for priority 2, the
    trade AGENTS.md's ordering already authorizes. A new crate owes the three things the standing decision
    below names, and nothing here needs a capability, a handle or a reactor, which is why it is goal `core-depth`'s
    and not goal `core-part-ii`'s.

## Acceptance

**This goal is retired, and its checks are the floor stage of the live goal** —
`../loop-goal.toml`, where `python tools/loop.py --list` reads them. Every switch
since has folded them forward again, which is what let `chain.py --retire` delete the copy that used to
be here.

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.** Settle it under AGENTS.md's priority ordering
  and record it in the home AGENTS.md already names — a paragraph in `docs/adr/README.md`
  § *Decisions taken at project start*, or the crate's own module doc.
- **ADR slots for this goal: none.** Every design this goal reaches is already argued — 0057, 0061, 0071,
  0077, 0085, 0086, 0088, 0089, 0102. A session that believes it needs a new number has almost certainly
  found a section of one of those it has not read; the ADR is the answer, and if it genuinely is not,
  that is a `BLOCKED`.
- **The four attribute passes extend `ATTRIBUTES`, never widen the matching rule.** A name on that roster
  is matched *nominally* after `nvs_hir::resolve_ref`, so `#[Core\Route]` and a `use Core;`d `#[Route]`
  are one attribute and no userland spelling is any of them. Structural matching is `rule:attributes/inert-metadata`'s rule for
  *retrieval* and is a different question.
- **A fold and its runtime path are one implementation.** If preparing a literal appears to need its own
  parser, the fold is wrong and the runtime parser is what gets an entry point — never a second copy.
- **`Core\Router` splits, and `::match` is not in scope.** Neither is `Core\Command::run`, the terminal,
  or anything that opens a file. A session that reaches one puts it in `## Backlog`.
- **The conformance floor is per class, not per corpus.** Raising the total case count without moving a
  class's `floor` column has not closed item 10, and `gaps.py` is what says so.
- **Picking every dependency but the two the user named** stays pre-authorized under `rule:packaging/a-c-dependency-answers-two-questions`. A new
  Rust dependency owes three things: the `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py`.

## What this goal does not touch

Anything needing a capability, a reactor, a driver or an open handle — that is goals `concurrency` through `server`. `nvs-ir` and
`nvs-codegen`, except where an attribute pass emits through the path `#[Json\Derive]` already uses.
Item 15 *owns* seventeen standing `nvs-ir` refusals without scheduling one of them; owning is not
touching, and it is there so the floor's own gate has an answer rather than a hole. Doc
trimming and dependency sweeps, both of which the user fires.
