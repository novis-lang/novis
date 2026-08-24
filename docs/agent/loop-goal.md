# Loop goal

Finish **M4S Part I** — every member of `docs/spec/01-core-library.md` §§ 1–12 — and with it the part of
**M4**'s language surface that Part I cannot be written without. Read those two milestone paragraphs in
`docs/implementation-plan.md` for scope; do not re-derive them here.

The previous loop reached its acceptance list and stopped there. That list was a *threshold*, not a
milestone: it left `Core` at 39 of ~205 spec member rows, with `Core\Str` and `Core\Arr` the only
registered classes and §§ 3–12 not existing at all. This loop closes that.

The one thing standing under all of it: **there is no representation for `?T`.** `mwl_ir` gap 3 covers it —
no nullable type, and `mixed` is a representation to erase *into* rather than dispatch *on*. Every member
whose spec signature returns `?T` is unstatable today, and that is `first`, `last`, `keyOf`, `firstKey`,
`lastKey`, `find`, `findKey`, `min`, `max`, `average`, `indexOf`, `lastIndexOf`, `before`, `after`,
`slice`'s `?int $length`, `Regex::match`, `Path::extension`, `Path::relativeTo`, `ObjectMap::get` and
`Random::pick` — across every section. Two narrower shapes sit beside it: a **variadic** parameter, which
ADR 0069's `overlay`/`underlay`/`appendAll` and `Arr::append`/`prepend`/`Path::join` all declare, and a
**union return**, which `Math::abs`/`Arr::sum` declare and which `CoreTy::Union`'s own docs call blocked on
the same representation. **Start there.** Nothing else in this goal has a site to attach to until it lands.

## Acceptance

**The checks themselves live in [`loop-goal.toml`](loop-goal.toml), and only there.** Every fixture, its
exact expected output, the two suites and every named guard test are in that file as data; the driver reads
it directly, so there is nothing here that could drift out of sync with what actually runs. Read it, or run
`python tools/loop.py --list` for the same thing as a summary.

The driver runs those checks in order, short-circuiting on the first failure, so the ledger line each
iteration writes tells you exactly how far the loop got. Every check must pass. Nothing else counts as done
— not a passing unit test, not a session claiming `DONE`. What the check kinds mean, how the native/WSL legs
and the valgrind sweep are ordered, and why: [coordinator.md](coordinator.md) § *The acceptance test*.

Stage 1 is the previous loop's whole list, unchanged — **a non-regression floor, never traded for anything
above it.** Stage 4's second named test, `every_part_one_spec_member_is_registered`, does not exist yet:
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
- **Doc trimming is not loop work, ever.** Nothing measures doc size (AGENTS.md § *Length targets*), and
  [doc-cleanup.md](doc-cleanup.md)'s pass is never run from inside the loop.

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
