# Loop goal

Finish **M4S Part I** — every member of `docs/spec/01-core-library.md` §§ 1–12 — and with it the part of
**M4**'s language surface that Part I cannot be written without. Read those two milestone paragraphs in
`docs/implementation-plan.md` for scope; do not re-derive them here.

The previous loop reached its acceptance list and stopped there. That list was a *threshold*, not a
milestone: it left `Core` at 39 of ~205 spec member rows, with `Core\Str` and `Core\Arr` the only
registered classes and §§ 3–12 not existing at all. This loop closes that.

The keystone that used to stand under all of it — a representation for `?T` and for the `mixed` tag — is
**built**: `mwl_ir::Ty::Tagged` is that representation, and the variadic and union-return shapes beside it
are `registry::CoreTy::Variadic` and `CoreTy::Union`. Every `Core` signature the spec writes can now be
stated. **What comes first now is Stage 0 below, not `Core` breadth.**

## Stage 0 — catch up before anything else

**Eleven ADRs (0080–0090) landed after the milestones that own their work were reported done.** Two of
them change a milestone's *built* behaviour rather than adding to a later one, and the debt beside them is
what M1 and M2 never finished. **ADR 0094 joins the list from the other direction** — a decision taken
while the loop is running, which reopens M1's declaration grammar rather than adding to a later milestone,
so it is item 3 here instead of scheduled work somewhere ahead. **Items 10–17 arrive the same way**, from
a review of what PHP's last three years of CVEs and performance work say about MWL: each was settled with
the user, and each is here rather than scheduled to a milestone because the emit site, the ABI or the
member it changes is being written *right now* — item 15 alone is the difference between `$a[] = $v`
costing 9.4× what PHP's interpreter charges and roughly an eighth of it. **16 and 17 came from a second
pass over the same ground and are already done**, both being small enough that queueing them would have
cost more than doing them; they are listed so neither is re-opened and so the review's full output is in
one place. Until this section is empty, **a session takes its group from here, in this
order, and does not open a Stage 3 `Core` slice.** The reason is compounding cost, not tidiness: the
spelling ADR 0090 deletes had reached 48 files and 93 lines before item 1 rewrote them, and every fixture
written while an item here is open is written against a rule that is about to change.

The machine-checkable half is `loop-goal.toml`'s `stage = "0 catch-up"` block, which `tools/loop.py` runs
**before** the program legs. Every test it names must exist and pass; most do not exist yet, and writing
one is how an item finishes.

1. ~~**ADR 0090 § 1 — `===`/`!==` stop parsing** (M1).~~ **Done.** `E0232` names each spelling at the
   lexer, which consumes the three characters and pushes the two-character token so one file still
   reports every one of its own problems; `TokenKind::EqualsEqualsEquals`/`BangEqualsEquals` and
   `BinaryOp::Identical`/`NotIdentical` are gone, and the whole corpus is rewritten.
2. ~~**ADR 0090 § 2 — two statically disjoint operands do not compile** (M2).~~ **Done.** `E0466` over
   the whole of that ADR's table, from `mwl_types::expr::operators`' `reject_disjoint_equality`, which
   `==`/`!=`, a `switch` label and a `match` arm all reach (§ 6). The predicate is one-sided on purpose —
   it refuses only where disjointness is provable from the two types alone — and that function's own doc
   comment owns why.
3. ~~**ADR 0094 — every member declaration writes a visibility** (M1).~~ **Done.** `E0122` from
   `mwl_syntax::check_declarations` — the post-parse declaration walk that was `check_casing`, renamed
   because it now answers two questions rather than one — over every property, class constant and method
   in a `class`, `interface` or anonymous-class body, with a bare `(set)` naming the pair it is missing
   (§ 3) and a class-body `var` redirected in the parser (§ 4). A plain constructor parameter stays exempt
   and an `enum` body still reports only `E0220`. The corpus rewrite landed in the same slice; it was far
   smaller than estimated, because a `mwl-types` or parser fixture never reaches that walk. It is the
   *declaration* half of visibility; item 6 is the *access* half, and neither waits on the other.
4. ~~**ADR 0090 §§ 2, 3 and 5 run**~~ (M3/M4; `mwl-ir`'s gap 19 owns the rest). **Done.** Every row of
   § 3's table runs: a string pair calls `mwl_str_eq`, an array pair the new
   `mwl_runtime::mwl_array_eq`, an object pair an inline pointer comparison, a `mixed` or union operand
   § 5's `Helper::Identical` over `value_identical`, and § 2's numeric domain a `Helper::NumericEq` over
   the new `mwl_runtime::numeric_identical` — settled in `mwl-ir`'s `lower_binary` beside the `decimal`
   and `Tagged` arms, so `mwl-codegen` keeps its "a `BinOp` has one representation" invariant. The null
   test moved with item 1, and the conformance rows dropped for the numeric domain are back. § 5's
   numeric row closed last: `value_identical`'s four numeric representations delegate to
   `numeric_identical`, so the row has one answer however it is reached, and `value_hash` canonicalizes a
   numeric to the `f64` it coincides with so the set index agrees with the comparison. That decides
   `Core\Arr` too — `contains([1.0], 1)` is `true` — under the strict-identity standing decision below,
   and `mwl_runtime::identity`'s own module doc owns every row of it.
5. ~~**ADR 0047 § 4 — the literal and enum-case type atoms are checked** (M2).~~ **Done.** All three
   atoms intern (`Ty::StringLiteral`, `Ty::IntLiteral`, `Ty::EnumCase`), § 4's assignability widens each
   to its base by one recursion, § 6 refuses a conversion the operand's own value disproves (`E0469`/
   `E0470`), and § 5 costs nothing until a checked `as`: a union whose members share one representation
   erases to it, and `lower_literal_membership` emits one comparison per member with the accepted set
   named at the throw. ADR 0010 § 5 rides the same chain in both directions — an enum out to its backing
   integer, and an integer back in, checked against every case of the declaration. What is left belongs
   to **ADR 0007 § 2** rather than to either of them: a `mixed` operand converts to `string` and to
   `decimal` and to nothing else, because `mixed as int` needs a helper that throws where
   `Helper::ToIntOrNull` answers `null` (`mwl-ir` gap 20).
6. ~~**`private`/`protected` are enforced** (M2).~~ **Done.** `E0471` from
   `mwl_types::expr::members::check_member_visibility`, keyed on the accessing class
   (`Ctx::current_class`) and never on the receiver's static type, so a second instance of the declaring
   class is as reachable as `$this` and the identical line at file scope is not. A property reaches it
   through `resolve_property_owned` (`$obj->n`, `Foo::$n`, and a write through the same span), a method
   through `check_method_visibility`, which `$obj->m()`, `C::m()` and `new C(...)` all take — a `private`
   constructor is the singleton idiom it was written to be. Where ADR 0043 § 3's private-interface-method
   rule already fired, only that more specific diagnostic is reported. The one declaration still outside
   it is a promoted constructor parameter, which no table records as a property.
7. ~~**`Comparable`/`Stringable` carry their member signatures** (M2).~~ **Done.** `iter_lib` seeds all
   four reserved interfaces with their members, `layout` seeds each a descriptor so `instanceof` answers,
   `require_stringable` records the `toString()` ADR 0028 § 1's four implicit sites desugar to, and
   `implements_interface` is reflexive so a value typed at the interface itself satisfies it.
8. ~~**ADR 0061 — `autoload` parses and resolves** (M1 grammar, M2 fixpoint).~~ **Done.** Both file-scope
   forms parse to `StmtKind::AutoloadDecl`, and `mwl_hir::requires::resolve_program` runs § 1's lookup as
   a fixpoint over the require-graph worklist: one AST walk per file harvests its requires, its `autoload`
   declarations and every name it uses where a class is meant — attributes included
   (`requires::walk_attributes`), so a class named only by `#[Route(...)]` autoloads. `E0315`–`E0318` are
   its four diagnostics, and ten `tests/conformance/lang/` cases pin the lookup, the shadowing rule,
   ordered probing, the exact on-disk spelling, the silent skip and each diagnostic. § 1's
   `mwl check --autoload-map` prints the resolved map, what a glob passed over and what was shadowed.
   One thing is left, and it is the cache's rather than this ADR's: § 5's probe trace is produced and
   dropped rather than folded into ADR 0042's key.
9. ~~**ADR 0069 — `array + array` does not compile** (M4's *Verify* list).~~ **Done.** `E0467` from
   `reject_array_combination`, naming `Core\Arr::underlay`; `+=` reaches it through `binary_result` and
   reports once, because the recovery type is the array operand rather than `mixed`.

10. ~~**A release build checks integer overflow.**~~ **Done.** `overflow-checks = true` was already on
    `[profile.release]`, with the measured basis and the reasoning in the manifest comment beside it; what
    was missing was the thing that fails when the line is deleted, and that is now
    `crates/mwl-runtime/tests/manifest_policy.rs` — the shape `mwl-codegen`'s `backend_policy` already
    uses, reading the `[profile.release]` block with its comments stripped, because that block's own prose
    names the setting several times while explaining it. Nothing else: the cast half is already enforced
    by `[workspace.lints.clippy]` plus `verify.py`'s `-D warnings`.
11. **`secret == secret` lowers to the constant-time helper**
    ([ADR 0033 § 5](../adr/0033-secret-qualifier-for-confidential-values.md) owns the rule and its cost).
    The arm goes in `mwl_ir`'s `lower_binary`, beside the equality work item 4 landed — which is why it is
    dated now rather than scheduled.
12. **`$s as ?Uri` and `$s as ?Uuid` compile, and `isValid` is deleted from both**
    ([ADR 0066 §§ 1, 3](../adr/0066-nullable-conversion-operator.md) owns the roster and why `Duration` is
    not on it; `uri.rs`'s module doc owns the call-site consequence). `Uri::isValid` is not written yet, so
    deleting it now costs nothing and costs a member plus its tests once it lands.
13. **`Core\Uri` compares by normalized components, with two guards.**
    `crates/mwl-stdlib/src/uri.rs`'s module doc owns the rule, why it does not contradict
    that module's own "`parse` reports, it does not normalize", and the two guards it requires — a
    `parse` → serialize → `parse` property and a differential corpus against the PHP 8.5 oracle. Add a
    `fuzz/fuzz_targets` entry beside `lex.rs` and `parse.rs`.
14. **A call-stack limit rides the safepoint's emit site**
    ([ADR 0020 § 1](../adr/0020-error-escalation-ladder.md) owns the mechanism, the 8 MB ceiling, the two
    tiers and the measured cost). File set: `crates/mwl-runtime/src/ctx.rs` for the `stack_limit` field
    beside the safepoint word, `crates/mwl-codegen/src/emit.rs`'s `emit_safepoint` for the compare. It is
    dated now because that emit site exists and lowers to nothing yet (`mwl-ir` gap 14), and the plan
    states that retrofitting it means rewriting the backend. Expect a one-time step in `abi/frame_depth`,
    a benchmark that does nothing but call; say so in the commit.
15. **A list-shaped array is packed** — [`array.rs`](../../crates/mwl-runtime/src/array.rs)'s module doc
    owns the decision, the PHP comparison that dates it, and what the ABI addition costs if it waits. Two
    points from that section worth repeating here: [ADR 0007 § 5](../adr/0007-explicit-type-system.md) is
    **unchanged** — this is representation, not semantics — and `mwl_array_get_index`/`set_index` are a
    compatible addition only while nothing depends on the current ABI. Give it the first
    `docs/perf/history.ndjson` entry, with a `php_ratio`, per
    [ADR 0026](../adr/0026-performance-measurement-methodology.md); that file not existing is why nothing
    caught this.
16. ~~**Cranelift's stack probes are on.**~~ **Done.** Cranelift defaults `enable_probestack` to
    *false*, and off means a frame larger than the 4 KiB guard page can move the stack pointer past it
    in one step — a stack clash, which is a memory-safety bug rather than the clean crash a guard page
    exists to produce. `Jit::new`'s own comment owns the reasoning and the measurement: under callgrind
    on this tree the retired-instruction count is unchanged to five significant figures with the flag on
    (92,237,951 off vs 92,237,800 inline, call-heavy; 55,399,358 vs 55,399,652 at 200 frames deep),
    because a probe is emitted only above 4 KiB and no MWL frame is that big *yet*. **It is not item 14
    under another name** — that counts depth against a `Ctx` field, this catches one oversized frame
    skipping the guard, and neither covers the other.
17. ~~**One allocation guard, not one per member.**~~ **Done.** `mwl_runtime::affordable` is the single
    place a count-shaped argument becomes a refusal, and its own doc comment owns why — including that
    it is *not* a budget yet, and that [ADR 0004](../adr/0004-memory-for-simplicity.md)'s
    `[limits.hard]` per-request ceiling attaches there when the M6 arena carries it. It replaced four
    hand-written copies (`Core\Bytes`, `Core\Str::repeat`, both `Core\Random` draws) and, more to the
    point, reached the three members that had **no** check at all: `Core\Arr::fill`/`padStart`/`padEnd`
    through `append_copies`, and `Core\Str::padStart`/`padEnd` through `padding_run`. `Arr::fill($n, 0)`
    was an unbounded run for any `uint` a caller chose — measured at 110 bytes and ~1.2 µs *per entry*,
    so 100M entries is ~11 GB and about two minutes — and is now a catchable throw. The gate test is the
    invariant rather than the behaviour: it fails when someone writes copy number five.

**Already done, and listed so it is not re-opened:** ADR 0087's lexer half is built — `mwl_syntax::bidi` is
the one predicate, the lexer makes it `E0008` over comments, string literals and inline HTML per line, and
seven `.mwlt` cases pin it. Its two sink halves are M7's and M8's, not catch-up.

**Not in this stage, deliberately:** ADR 0088's registry classification, 0086 § 6's command table and
0085's OpenAPI emitter are all M4S work that lands with the milestone the loop is already inside; 0081–0084
belong to milestones that have not started, as does 0093's `mwl service`. **0091 and 0092 join them, and
neither carries catch-up debt**
— unlike 0090 they invalidate no built behaviour and no written fixture, so there is nothing to rewrite
before Stage 3 continues. What they *do* carry is scheduled work, in the milestone that owns each piece:
0092's record model, plaintext rendering and `Core\Debug::dump` are M4; `[mode]`, `[log] format`/`level`
and `[http.errors] detail` are M6; the HTML rendering and `[debug] inline` are M7; `Core\Log` and JSON
Lines are M8; the compiler-diagnostic rendering is M10. The two documentation corrections they *did* owe —
the spec's mis-attributed `Core\Debug` row, and two ADR examples writing a `string` log level — landed with
the ADRs themselves.

## Acceptance

**The checks themselves live in [`loop-goal.toml`](loop-goal.toml), and only there.** Every fixture, its
exact expected output, the two suites and every named guard test are in that file as data; the driver reads
it directly, so there is nothing here that could drift out of sync with what actually runs. Read it, or run
`python tools/loop.py --list` for the same thing as a summary.

The driver runs those checks in order, short-circuiting on the first failure, so the ledger line each
iteration writes tells you exactly how far the loop got. Every check must pass. Nothing else counts as done
— not a passing unit test, not a session claiming `DONE`. What the check kinds mean, how the native/WSL legs
and the valgrind sweep are ordered, and why: [coordinator.md](coordinator.md) § *The acceptance test*.

Stage 0 is the catch-up list above, and it runs before the program legs so the ledger names it while it is
unfinished. Stage 1 is the previous loop's whole list, unchanged — **a non-regression floor, never traded
for anything above it.** Stage 4's second named test, `every_part_one_spec_member_is_registered`, does not exist yet:
writing it is this loop's real definition of done, because it reads the member rows out of the spec file
itself and fails naming every one with no registry entry. A count of conformance cases is a proxy; that test
is not.

**The expected output in that file is frozen; a fixture's *source* is not.** The seven new fixtures were
written against the spec by someone who could not compile them, so every `Core` signature, every enum
namespace and every options bag in them is a reading of the spec that may be wrong. Correcting one is a bug
fix, not a decision. In particular the fixtures spell a `Core`-owned enum `Core\RoundMode`, `Core\Unit`,
`Core\Digest`, `Core\Charset` — flat under `Core`, following `Core\Order`, which is the only such enum that
exists today; if the loop places them elsewhere, fix the fixtures. What may never change is the expected
output, or the fixture's reason for existing. Re-freeze, record why in the commit message, move on. That is
not licence to weaken a check to make it pass.

## Standing decisions — pre-authorized, do not stop the loop for these

Every one of these was settled with the user before the loop started. Implement it; do not re-open it.

- **Decide and record; never `BLOCKED` for a design call.** The `?T` representation, the `mixed` runtime
  type tag, the variadic and union-return `CoreTy` shapes, and every question they raise downstream are
  yours to settle under AGENTS.md's priority ordering. Record each in the home AGENTS.md already names — a
  paragraph in `docs/adr/README.md` § *Decisions taken at project start*, or the crate's own module doc.
  **Do not open a numbered ADR for these.** Reserve `BLOCKED` for a decision that is expensive to reverse
  *and* has no safe default.
- **`?T` and the `mixed` tag are one design, decided once.** A nullable value and an erased one both need a
  runtime discriminant, and choosing two unrelated shapes for them is the mistake to avoid. Whatever is
  chosen goes in `mwl-ir`'s module doc for the IR half and `mwl-runtime`'s for the heap half, with the cost
  it spends per value stated, per AGENTS.md's memory rule.
- **A `catch` binding is re-scoped to its own handler block.** Today it is function-scoped, so two clauses
  on one `try` cannot both bind `$e` and `E0406` fires on the second — PHP allows it and every PHP program
  writes it, so the differential corpus cannot grow past it. Make the change and record the rule in
  `mwl-types`' own module doc; if the implementation forces the opposite conclusion, keep the current rule
  and record *that*, with the reason.
- **`array<T>` becomes element-covariant on read, and object identity is settled.** The invariance that
  refuses `Arr::flip($stringArray)` is widened — an MWL array is a copy-on-write *value*, so a covariant
  read cannot be aliased into an unsound write — and the one strict-identity comparison over two `Value`s
  that `contains`/`diff`/`intersect`/`unique`/`ObjectSet` all need is defined in `mwl-runtime`, deciding
  what object identity means there. Both recorded in the owning crate's module doc.
- **Three M4 language holes are in scope, because the corpus cannot be written around them.** Compound
  assignment (`mwl-ir` gap 16 — a desugar of `$x op= e` to `$x = $x op e`), `for`/`switch`/`match`
  (`mwl-ir` gap 1 — every terminator they need already exists), and `decimal`'s IR representation
  (`mwl-ir` gap 15 — ADR 0054's 16-byte register pair). ADR 0070's **duration literal** is in scope too:
  `Core\Time\Duration::parse` shares its grammar and its implementation, so build the literal first and
  `parse` is the same parser reached from a second entry point.
- **Dependencies: two are named, the rest are yours.** `Core\Regex` binds **`regex`** as ADR 0056's
  linear-time default *and* **`fancy-regex`** as its budgeted opt-in backtracking tier — both tiers land,
  closing that ADR rather than half of it. `Core\Time` binds **`jiff`**, whose type set maps almost
  one-to-one onto § 4's `Instant`/`DateTime`/`Duration`/`Zone`. Every other outside crate — JSON, hashing,
  UUID, CSV, base64/base32, Unicode segmentation and normalization, WHATWG encodings — you pick against
  [ADR 0051 § 4](../adr/0051-standard-library-tiers.md)'s two questions, keeping `cargo deny check` green
  and recording each pick with its reasoning in that module's own doc comment. A new dependency owes three
  things (AGENTS.md): the `[workspace.dependencies]` line with a comment saying why that crate,
  `cargo deny check`, and `python tools/gen-attribution.py`. Stop only if a needed capability has no
  pure-Rust option at all — that is a real `BLOCKED`, naming the capability.
- **`Core\Uri::parseQuery` implements PHP's bracket convention in full**, and its spec row is amended to
  match: `a[]=1&a[]=2` builds a list, `a[b]=c` builds a map, nesting to arbitrary depth. The return type is
  therefore not `array<string>` — pick the spelling the type system can actually state once `?T`/`mixed`
  land, and write it into the spec table. This also settles the answer `Core\Request::query` owes at M8;
  say so in § 12 rather than leaving two open questions.
- **There is no `Core\Str::editDistance`.** The question is closed: fuzzy matching is not a Tier 0 concern
  under ADR 0051's six tests. `docs/spec/02-php-migration.md`'s `levenshtein` row already says so.
- **`Core` is native Rust, all of it.** Every §§ 1–12 member is a Rust function in `crates/mwl-stdlib`,
  reached through the signature table the checker knows and the helper symbol codegen emits. No part of
  `Core` is written in MWL.
- **Spec §§ 1–12 only.** § 13 (`Reflect`, `Ast`, `Attributes` retrieval, `Program`, `Decimal`, `BigInt`,
  `Test`) stays out; each depends on something outside `Core`, and that file's own *Milestones* section
  says where each lands. § 10's exception tree already exists in `mwl_types::error_lib` — what it still
  owes is the constructor's `{previous: $e}` options shape, `$e->location`, and `ParseError::issues`,
  which [ADR 0071](../adr/0071-derived-codecs.md) § 5's one-throw-lists-every-bad-field rule needs.
- **Type variables stay compiler-owned.** `<T>` machinery is for declarations the compiler owns. User code
  gets exactly two things: implementing a compiler-owned generic interface at a concrete type, and an
  explicit call-site type argument (`Core\Json::decodeAs<User>`, `new Core\ObjectMap<Tag, int>()`).
  **User-defined generic classes stay deferred.**
- **No cycle collector.** Refcounting only; a cyclic graph in a CLI script is retained until the process
  exits, and Stage 6's leak check stays scoped to acyclic fixtures so it remains a true signal.
- **`Path` and the two legs.** `Core\Path` emits `Path::SEPARATOR`, which differs between the native
  Windows leg and the WSL one — so a fixture or a `.mwlt` case that asserts a built path must normalize it
  (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) or assert something separator-free. A case that
  bakes in one platform's separator passes one leg and fails the other.
- **Backlog items are off-path unless the goal needs them.** If a slice is not on the path to the
  acceptance list, put it in `## Backlog` in the handoff and move on.
- **Doc trimming is not loop work, ever.** Nothing measures doc size (doc-style.md § *Length targets*), and
  [doc-cleanup.md](doc-cleanup.md)'s pass is never run from inside the loop.
- **Four further decisions from the same review are settled in their own ADRs and are *not* this goal's
  work** — [0095](../adr/0095-ambiguous-input-is-refused-never-repaired.md) (HTTP parsing, cookie names,
  multipart caps, `Core\IO::within`), [0096](../adr/0096-a-route-without-a-declared-access-decision-does-not-compile.md)
  (`#[Access]`, CSRF) and [0018](../adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md)'s
  test axes. Their milestones have not started; do not re-open them and do not schedule them here.


## The gaps that actually sit on the path

Named because none is visible from either milestone's text, and each is work rather than a question. Each
panics naming itself rather than miscompiling, so hitting one is loud.

- **`?T` has no IR arm at all** (`mwl-ir` gap 3), and neither does `null`. This is the keystone above.
- **A compound assignment does not lower** (`mwl-ir` gap 16), and **`for`/`switch`/`match` do not lower**
  (gap 1). `examples/match.mwl` needs all four.
- **`decimal` has no IR representation** (`mwl-ir` gap 15) — the keyword, the type atom and ADR 0054 § 3's
  arithmetic table are all live in the front end, so a program that declares one reaches
  `lower_decl_type` and panics. `examples/numbers.mwl` declares two.
- **Only ADR 0007 § 2's free and total conversion rows lower** (gap 4). ADR 0066's `as ?T` needs the
  *checked* rows plus a non-throwing form of each, and `examples/nullable.mwl` uses `"4x" as ?int`.
- **`&&`/`||`/`!`/ternary lower only where a mutable `cur: &mut BlockId` is already owned** (gap 5).
  Nested inside a call argument they panic — and five of the seven new fixtures write a ternary inside
  one.
- **No variadic, named or spread call argument** (gap 8), and `mwl_types` does not positionally
  type-check one either. ADR 0069's three combination members and `Path::join` all need it.
- **`$f(...)` does not lower** (gap 9) — only native `Core` code calling back through
  `mwl_runtime::call_closure` works today. `Core\Out::capture` and `Regex::replaceWith` are native, so they
  are fine; a `.mwlt` case that calls a closure variable directly is not.
- **Neither `.` nor `as string` covers a `Stringable` operand** (gap 12). `Duration` is `Stringable` by
  § 4, and `$d->toSeconds()` is the way around it in the fixture.
- **A static property reads but does not write** (gap 6), a nested array write `$grid[0][1] = v` is
  refused, and neither `ArrayGet` nor `ArraySet` models an absent key at runtime.
- **Class-member `private`/`protected` is not enforced at all** (`mwl-types`' gap list), and its reserved
  `Comparable`/`Stringable` interfaces carry no member signatures — which `Core\Heap`'s ordering and
  `Duration`'s `Stringable` both need.
- **`new` on an `abstract` class is not refused, and `$n->foo()` on a scalar receiver panics `mwl-ir`** —
  both missing `mwl-types` diagnostics.
- **An abandoned generator never runs the `finally` it is suspended inside** (`mwl-ir` gap 18) — the one
  PHP divergence the corpus has found and not closed.
- **`crates/mwl-ir/src/lib.rs`'s module doc is a slice-by-slice changelog** of exactly the kind AGENTS.md
  forbids. It is the one doc in the repo genuinely owed a trim. Backlog, not a reason to stop.
