# Loop goal 1 — `Core`'s pure half, finished

Finish **M4S** — [docs/plan/m4s.md](../../plan/m4s.md) is the scope and this file does not restate it.
Every member of [docs/spec/01-core-library.md](../../spec/01-core-library.md) §§ 1–13 that does not need a
capability, a reactor, a driver or an open handle **runs, is cased, and is claimed by a row in the
migration table.**

This is order 1 of the parity program ([goals/README.md](README.md)), and it is first for a reason that
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
   pass, the OpenAPI emitter and ADR 0061 § 3's program enumeration are all specified, none implemented.
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

1. **ADR 0061 § 3's program enumeration exists.** `Core\Program::implementing<T>()` expands at compile
   time to an array literal of `new` expressions, one per non-abstract class implementing `T`, **sorted by
   fully-qualified name** so the order never depends on filesystem enumeration; a class with no
   no-argument constructor is a diagnostic naming it. `nvs-hir` has §§ 1–2 — `AutoloadMap::build` at
   [autoload.rs:162](../../../crates/nvs-hir/src/autoload.rs) and the `resolve` beside it — and nothing
   walks the resulting program to answer "which classes implement this interface".

   **It is Stage 0 because three later items are the same scan.** `#[Route]`'s table (item 6),
   `#[Command]`'s table (item 8) and the OpenAPI emitter (item 9) are all "filter ADR 0061 § 3's
   enumeration", and ADR 0077 § 5 says so outright. Writing any of them first means writing the walk
   three times and then unifying it. `crates/nvs-hir/src/autoload.rs`,
   `crates/nvs-hir/src/hierarchy.rs` (`implements_interface` is already there),
   `crates/nvs-types/src/expr/calls.rs` for the call-site expansion.

## Stage 1 — the floor

M4's entire acceptance list, inserted mechanically by `goal-switch.py` and **never traded for anything
above it.** A session that finds it has to change a floor fixture's expected output has found a bug in its
own slice. Nothing in this goal touches `nvs-ir` lowering or `nvs-codegen`, so a failure there is a real
regression and never a scope question.

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
   [ADR 0071](../../adr/0071-derived-codecs.md) §§ 2, 7.
3. **`#[Route]` builds a table while compiling.** [ADR 0077](../../adr/0077-compile-time-routing.md) §§ 1–3
   and 5: matched nominally like every other name on `ATTRIBUTES`, the path grammar of § 2, a parameter's
   type coming from the method and being what launders it, and the table built by Stage 0's scan. Its
   three compile errors are the whole point of doing it here — a duplicate route, a `{param}` with no
   matching method parameter, an unknown literal `url()` name.
4. **`#[Query]` and `#[Access]` join it**, and with them ADR 0102's four further compile errors: a
   `{name?}` outside the last position or bound to a parameter with no default (§ 4), a capture or
   `#[Query]` parameter whose type is outside § 3's list, and a `url()` key that is neither a capture nor
   a declared `#[Query]` parameter (§ 6). Same pass, same file, one group with item 3.
   [ADR 0102](../../adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md).
5. **`Core\Router::url()` and `::urlAbsolute()` are launderers over that table** — and only those two.
   `::match` and `::methodsFor` are goal 6's, because a match needs a request. Splitting the class this
   way is [01-core-library.md](../../spec/01-core-library.md) § *Milestones*'s own instruction and not this
   goal's invention.
6. **`#[Command]`/`#[Option]`/`#[Argument]` build the command table.**
   [ADR 0086](../../adr/0086-core-cli-terminal-is-a-sink.md) § 6, with its own three compile errors: a
   duplicate command name, two options sharing a spelling, an `#[Option]` on a parameter with no
   conversion from `string`. **The table only** — `Core\Command::run`, the generated `--help` and the
   completions are goal 4's, since neither argv nor a terminal is reachable before capabilities exist.

## Stage 3 — the intrinsic-folding pass

7. **[ADR 0057](../../adr/0057-intrinsic-literal-folding.md)'s closed list folds.** A short list of `Core`
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
   [ADR 0085](../../adr/0085-openapi-is-generated-from-the-route-table.md): § 1's "what supplies what" is
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
    [ADR 0088](../../adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md) § 2: an unclassified
    `string`/`bytes` parameter refuses `tainted`, and `nvs-stdlib`'s own suite fails on any member that
    ships without a classification. m4s.md calls this "part of building §§ 1–12 rather than a separate
    slice" — it is listed here because the members are built and the pass is what is left.
    [registry.rs:490](../../../crates/nvs-stdlib/src/registry.rs) is what a member's row may say.

## Stage 6 — the string header

13. **`Core\Str`'s grapheme count is lazily cached, and a concatenation corrects the boundary in O(1).**
    [ADR 0009](../../adr/0009-string-and-bytes.md) § 2 decided the grapheme cluster is the unit and
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
    [ADR 0089](../../adr/0089-convert-is-one-rule-table-with-two-modes.md) § 6.

    **Nothing here is guessed to make a number move.** A name with no home is a finding, not a `dropped`
    row: three of them turned out to be real gaps last time somebody looked.

## Acceptance

**The checks live in [`1-core-depth.toml`](1-core-depth.toml), and only there.** Read it, or
`python tools/loop.py --list`.

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
  are one attribute and no userland spelling is any of them. Structural matching is ADR 0046's rule for
  *retrieval* and is a different question.
- **A fold and its runtime path are one implementation.** If preparing a literal appears to need its own
  parser, the fold is wrong and the runtime parser is what gets an entry point — never a second copy.
- **`Core\Router` splits, and `::match` is not in scope.** Neither is `Core\Command::run`, the terminal,
  or anything that opens a file. A session that reaches one puts it in `## Backlog`.
- **The conformance floor is per class, not per corpus.** Raising the total case count without moving a
  class's `floor` column has not closed item 10, and `gaps.py` is what says so.
- **Picking every dependency but the two the user named** stays pre-authorized under ADR 0051 § 4. A new
  Rust dependency owes three things: the `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py`.

## What this goal does not touch

Anything needing a capability, a reactor, a driver or an open handle — that is goals 2–6. `nvs-ir` and
`nvs-codegen`, except where an attribute pass emits through the path `#[Json\Derive]` already uses. Doc
trimming and dependency sweeps, both of which the user fires.
