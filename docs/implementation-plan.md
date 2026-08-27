# MWL — Modern Web Lang: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-27. Current milestone **M4**, language completeness: every shape that compiles
> in the front end and then refuses below it, closed, before M4B's LSP is written against the
> surface. M0–M3 are done and M4S Part I is the corpus floor rather than the frontier.
> [docs/agent/loop-goal.md](agent/loop-goal.md) holds the goal's items grouped by file set, and
> `python tools/holes.py` is the live count behind them. Dependencies: `regex` + `fancy-regex` and
> `jiff` are named by the user; the rest the loop picks under ADR 0051 § 4.
>
> **Done:** M0 (setup) and M1 (front end) whole, M2 (HIR, types, IR) and M3 (baseline Cranelift
> backend) whole, M4S Part I registered — `crates/mwl-stdlib/tests/spec-members-outstanding.txt`
> holds no keys, which is this project's definition of *registered*. M1's own section lists the one
> grammar addition still owed (`autoload`, ADR 0061). Each milestone file under
> [docs/plan/](plan/) states its own acceptance.
>
> **On disk:** the workspace and its CI (three platforms, with miri, asan and fuzz legs), and the
> nine crates — `mwl-diagnostics`, `mwl-syntax`, `mwl-hir`, `mwl-types`, `mwl-ir`, `mwl-runtime`,
> `mwl-stdlib`, `mwl-codegen`, `mwl-cli` — plus `mwl-test`, `fuzz/`, `tools/`, `benches/abi-probe`,
> and the two case trees `tests/conformance` and `tests/differential`. **Each crate's own module doc
> is the authority on what it holds and what it still owes**; `python tools/brief.py` prints one map
> line each, and `python tools/disk.py` the live counts.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows
> SDK 10.0.26100 for linking, PHP 8.5.9 as the differential oracle — on the Windows `PATH` and
> inside the WSL distro alike, at the same version — `cargo-fuzz` 0.13.2 and `valgrind` under a WSL
> nightly toolchain (docs/setup.md is what a machine installs, and why).
>
> **ADR slices landed:** **each ADR's own *Verification* section is the authority on what its slice
> covers, and this field never restates one** — `python tools/brief.py --where <keyword>` routes to
> the ADR that owns a topic, and `python tools/adr.py --stats` shapes the whole set. What a crate
> still owes is its own module doc's `# Known gaps`. What landed in which session is in `git log`.
>
> **Open now:** **M4's language holes are the frontier**, ordered and grouped by file set in
> [docs/agent/loop-goal.md](agent/loop-goal.md). A hole is a shape that compiles in the front end
> and then refuses below it; it is closed when it either runs with a fixture or a `.mwlt` case
> pinning what it prints, or is refused by a **diagnostic that names the rule** — never by a panic.
> The statement dispatch has no shape left that the checker accepts: ADR 0007 § 3.3's `[int $a,
> string $b] = $pair;` lowers as the subscripts it is spelled out of and refuses what one refuses
> (`E0482`/`E0401`/`E0483`), inline HTML lowers over its raw span, a nested
> `class`/`interface`/`enum` is `E0233`, an increment takes its write target's own
> `E0479`/`E0478`/`E0480`, `unset()` is narrowed to an array element of a named holder and refuses
> every other operand (`E0234`, plus `E0413` for a static property), an element write whose root is
> only a temporary is `E0700` — the first code of the `E07xx` band the full `E04xx` one continues in
> — an increment's own target passes the parser's `E0105` gate like every other write spelling, the
> read-modify-write rewrite's own assertion has no reachable target left — its doc comment carries
> the proof, and `mwl_types`' two write-target refusals are two thirds of it — an element write
> evaluates the receiver under its root holder exactly once, PHP 8.5.9's own count, a computed
> member name (`->$name` / `->{expr}`) is `E0235` where it is written and an undeclared property is
> `E0405` on every class kind, which together leave the property-write panic no reachable target,
> all four write spellings — `=`, `⊕=`, an increment and `unset()` — agree on the three
> element-write holders that are no slot and take exactly one diagnostic each for it, an
> intermediate level of a nested element write is `E0482` unless it is an array of its own, which
> together leave `write_back_array`'s and `row_ty_of`'s panics no reachable target either, and `int
> $x;` and `;` both lower, all seven declaration spellings — `class`, `interface`, `enum`, `type`,
> `namespace`, `use` and `autoload` — are skipped at file scope and `E0233` inside a body, so an
> `autoload` written in the entry point itself resolves a class exactly as one in a bootstrap file
> does, and an expression used as its own statement is evaluated for its effects with its value
> discarded whatever shape it is, `$a = &$b;` being the one spelling refused instead (`E0701`, ADR
> 0031 § 2 has nowhere to put a reference) — which together leave both of the statement slice's
> catch-alls no reachable target. The **expression** dispatch is now the same: `Foo::class` folds to
> the class's fully qualified name as a `string` constant and `self::class`/`parent::class` with it,
> `$obj::class` and `static::class` are `E0702` — ADR 0008 binds `static` at the call, so folding it
> would silently answer the declaring class — an undeclared name in one is the ordinary `E0303`, PHP
> 8's `throw` lowers in expression position now that a union absorbs the `never` a non-completing
> branch contributes (`$v ?? throw new LogicError(…)` satisfies a `string`), a `yield` used as a
> value is `E0448`, `spawn script` is `E0703` (ADR 0006's isolates are M5) and `require` used for
> its value is `E0704` (`mwl-ir`'s known gap 22), which together leave `lower_expr`'s catch-all no
> reachable target either. The two **operator** catch-alls one level down are now the same. Unary
> `+` is the identity over `int`, `uint`, `float` and `decimal` and lowers to its operand with no
> instruction at all, which is safe rather than a silent divergence only because `-`/`+`/`~` over an
> operand ADR 0007 § 4 tabulates no row for — a `string`, a `bytes`, an `array<T>`, a `bool`,
> `null`, a `callable`, an enum case — is now `E0705` where PHP would have converted it first, an
> object keeping the "MWL has no operator overloading" sentence it already had; `@` error
> suppression is `E0236` at the parser, ADR 0020 having made every failure a `Throwable` propagated
> by checked return so there is no channel to mute, which leaves `UnaryOp`'s five variants as four
> arms and one the parser never constructs. `BinaryOp`'s 22 are the scalar table's eighteen rows
> plus `.`, `&&`, `||` and `??`, each of which `lower_expr` takes before the general `Binary` arm
> that is that table's only caller, a compound assignment desugaring through the same four. One
> level down, the **`decimal` operator table** closes on the same subtraction: ADR 0054 § 3 grants
> twelve of the 22 — the five arithmetic rows, `==`/`!=`, the four orderings and `<=>` — and of the
> ten it does not, `**` was already `E0455` and the five bit operators are now `E0706`, ADR 0007 §
> 4's `& | ^ ~ << >>` row being over `int` and `uint` alone, so a `float`, `decimal`, `string`,
> `bool`, `null` or `array<T>` operand of any of the six spellings is refused where it is written
> rather than answered wrongly below — `1.5 & 1.5` used to evaluate to `1.5`, a bit-and over the
> `f64`'s own bits, where PHP answers `1`. The other four (`.`, `&&`, `||`, `??`) never reach that
> table at all, for the reason they never reach the scalar one. **`concat_operand`'s representation
> catch-all** goes with it: nine of `mwl_ir::ty::Ty`'s fifteen are rows — `null` newly among them,
> rendering as the empty string exactly as the `?string` holding one already did, which is PHP's
> answer and keeps the static and the tagged case agreeing — four are refused a phase up by the one
> check every implicit site shares (`E0707`: a `bytes`, an `array<T>`, an enum case and a `void`
> call, each naming the spelling that says what was meant, while the explicit `as string` keeps ADR
> 0009 § 3's `bytes` row), and `ClassDesc`/`Ref` are compiler-internal representations no source
> expression ever has. **The `as` conversion table** closes the same way one level down, and
> subtracting its grid found something worse than a panic underneath it. ADR 0007 § 2's table is a
> *closed* list of rows — `as` "either produces a value of the target type or throws", so a pair
> naming no row has nothing to produce and nothing to throw — and `mwl_types` now says so where it
> is written (`E0708`): `true as int`, `$xs as string`, `$case as string`, `$case as float`, `$i as
> bytes`, `$s as array<int>`, `null as int`, and a `void` call on either side, each help naming the
> spelling that says what was meant. `null as string` goes the other way and becomes a lowering row,
> the empty string `concat_operand` already answered for the same value and PHP answers too. A
> **class** target is decided by whether the two types share a value at all rather than by a row,
> because three shapes legitimately name one: a downcast out of an erased view or an interface
> (`object as Plain`, `Comparable as Cell` — one representation on both sides, so ADR 0036 § 4
> leaves the check to the member access), a `Core`-owned class deciding for itself (ADR 0024's `as
> Core\Html\Markup`, whose own `E0417` wants a source literal), and the identical type. What is left
> is the one that is no downcast: `$foo as Bar` between two unrelated classes, which was worse than
> a panic — both erase to one pointer, so it took the free `from == to` row, nothing ran, and
> `Bar`'s slot list was then read off a `Foo`'s allocation. `mwl-ir`'s own catch-all still has
> exactly the two targets its message names, and neither of them is an object any more: `array<T> as
> array<U>`, and ADR 0024 § 5's `string as Core\Html\Markup`, which is a *rule* rather than a test —
> a source-literal string and nothing else — and waits on `Core\Html` existing at all (M7). The
> tagged operand into an object closed, and it closed as two answers rather than one. A **declared
> class** target is ADR 0007 § 6's checked way out of `mixed`, and it needed nothing new:
> `InstKind::InstanceOf` already takes a `Ty::Tagged` subject and already answers `false` for a tag
> that is not an object, so the row is that test, a `Terminator::Throw` on the false edge and one
> free `InstKind::Untag` on the true one — a `Helper` could not have carried it in any case, helper
> arguments being stored as `Value`s that a class descriptor is not. `$m as Plain`, `$m as Shape`
> through an interface and `?Plain as Plain` all agree with `$m instanceof Plain` for every tag a
> `mixed` can hold, and the ownership is `convert`'s own free row split across the two edges:
> `Untag` is a relabelling, so a borrowed operand is retained on the way out and a fresh one
> transfers instead, and the false edge releases a fresh one before it throws rather than abandoning
> it on the edge — valgrind-clean over a fixture that converts, and fails to convert, two hundred
> times. Every **other** object target names no class to test against — plain `object`, a shape, a
> `callable`, and a `Core` class, which has no descriptor in the unit for the same reason
> `instanceof Core\Uri` is `E0496` — so from an operand that is not already an object the conversion
> could only assert a tag it cannot verify, which is `$foo as Bar`'s type confusion one step
> earlier, and `mwl_types` refuses it where it is written (`E0711`). `$plain as object` stays the
> free widening row it always was, and `Core\Html\Markup` is the one `Core` exemption, its own row
> being `mwl_types::expr::quals`' to own. The third was a tagged operand into `bytes` and it is a
> row now: ADR 0009 § 3's pair is the operand's own *tag*'s wherever its static type names neither
> side of it, so `Helper::TaggedToBytes` hands the same allocation back under the other tag for a
> `string` or a `bytes` and throws for every tag § 2's table gives no row. It is the only shape of
> `as bytes` that reaches a call at all, the statically typed spelling being a free `Reinterpret`
> over that same allocation. `lower_expr`'s dispatch message is the assertion its roster already
> proved. One level *up* from all of it, **a digit run beside a `uint` is now placed at `uint`**
> rather than defaulting to `int` — ADR 0007 § 2's "untyped until placed" applied to the one
> placement a binary operator offers, its other operand — so `$u + 1`, `$u & 3` and `$u << 1`
> compile at all, where each of them used to be § 4's mixed-signedness refusal and a `uint` could
> meet only a `uint`-declared local; a digit run above `i64::MAX`, which § 4 admits "only where a
> `uint` is expected", has an operand position for the first time, and `mwl-ir` makes the same
> placement on the left-hand operand so that it does not then panic on a value that never fit an
> `int`. What stays refused is the pair with no digit run in it: a shift's *count* is an operand of
> the operator rather than a bare width, judged by the row its left operand takes, which is exactly
> what makes `mwl-codegen`'s `emit_shift` sound in reading one signedness for both the
> negative-count guard and the arithmetic-versus-logical choice. One level up from the `as` table,
> **ADR 0066 § 3's own table is closed at both ends too**. `as ?T` "yields `null` exactly where `as
> T` would throw", so a row that never throws promises a `null` no run can produce and forces a
> check on every reader after it: `$i as ?int`, `$i as ?string`, `$xs as ?bool`, `$mode as ?int`,
> `$s as ?mixed` and `Mode::Read as ?Mode` are now `E0709`, each help naming `as T`, and that is the
> one judgement the plain form never has to make. The other end is shared: § 2's closure is asked of
> the `T` *inside* the sugar rather than of the `Union([Null, T])` it interns as, so `array<int> as
> ?int`, `$flag as ?int`, `$i as ?bytes` and `null as ?int` take the same `E0708` their unsugared
> spellings already did — they reached a lowering and panicked before, the checker having skipped
> the table for every written `?T`. The class row was already absolute and is untouched (`E0473`),
> and the three never fire on one expression. What is left under `as ?T` is the opposite direction,
> a row § 3 calls **available** with no `?` helper to run it, and it is now `$m as ?array<T>` alone,
> `Lowering::convert`'s own missing row in its null-answering spelling, so the two close together.
> The `bytes` target closed with its checked twin: `Helper::ToBytesOrNull` shares
> `Helper::TaggedToBytes`'s one implementation of the two rows a tag can take into a `bytes`, and
> neither can fault at all, so unlike the text target neither pays for a landing block. The **text**
> target is closed: `Helper::ToStringOrNull` is `Helper::TaggedToString`'s twin over one
> implementation of § 2's rows rather than a second copy of them, answering `null` exactly where
> that one throws, and it takes `$b as ?string` with it — ADR 0009 § 3's UTF-8 validation is a row
> that can fail, so the `bytes` source has a `null` answer of its own rather than a helper of its
> own. What `null` does **not** stand for is an exception the operand raised on the way, which is
> why this is the one `?` row emitted with ADR 0002's error edge: a `toString()` body that throws
> propagates through both spellings alike, and a `catch` around either sees it. One level under
> that, **an object whose static type names no class now renders through its runtime one** rather
> than panicking below. `require_stringable` resolves a `toString` wherever the operand's type names
> a class and `mwl-ir` calls it, unchanged; where it names none — an erased `object`, a `mixed`, any
> other union — `mwl_runtime::stringify` asks the concrete instance's class for the same member,
> which is ADR 0036 § 4's deferral applied to the member access ADR 0028 § 1 says the conversion
> *is*. So `echo $o`, `"" . $o` and `$o as string` are one answer for one value where they used to
> be a rendering, a throw and a panic, and a class that declares no `toString` throws catchably,
> naming itself and the interface. The `Core`-owned half is now the rendering half alone. Its
> *refusal* is where it is written: `require_stringable` asks `mwl_stdlib::registry::class_renders`,
> and a class it answers `false` for is `E0710` at the site rather than a throw below it — `echo`,
> an interpolated piece, a `.` operand and `as string` agreeing because they are one check. Two
> rosters answer, and they are two rules: `Core\Uri`, `Core\Uuid` and `Core\Time\Duration` have a
> `toString` row, and the two sink carriers render through ADR 0088 § 5 with no member at all, which
> is `mwl_runtime::is_carrier`'s list read rather than copied. The class that *does* render now
> renders. `require_stringable` records the same resolved `toString` target for a `Core` class that
> a declared one gets — a `Core` member resolves out of the seeded signature table like any other —
> and `mwl-ir` asks `core_symbol_of` which of the two calls to emit, so it takes the native
> `InstKind::CoreCall` the member written out takes rather than a `CallVirtual` into a method table
> a `Core` class has no entry in. All four rendering spellings therefore agree with
> `$uri->toString()` for each of the three classes the spec gives one, and the receiver's ownership
> inverts with the call: a native member *borrows* argument 0, so a fresh receiver (`echo
> Core\Uri::parse(…)`) is the rendering site's to release rather than the callee's, which is
> valgrind-clean over a fixture that renders in a loop. What is left of that row is the same class
> reached through an **erased** operand — a `Core` object behind a `mixed` dispatches through
> `mwl_runtime::stringify`, which reads a compiled method table and so throws for a value the static
> spelling renders. That one is not this crate's: `mwl-runtime` sits below `mwl-stdlib` and cannot
> read the registry, so closing it is a question of what the two share, and `mwl-ir`'s known gap 12
> owns the two candidate shapes. Three live tools **are** the worklist and no session re-derives
> one: `python tools/holes.py` reads the refusal sites out of `mwl-ir` and `mwl-codegen` and
> attributes each to its item (`--item N` for one in full), `python tools/loop.py --list` prints the
> named `.mwlt` cases each stage still owes, and `python tools/check-migration.py` scores
> `docs/spec/02-php-migration.md`.
>
> **Blocking:** Nothing external, and nothing waiting on a decision — every design call this loop
> reaches is pre-authorized in [docs/agent/loop-goal.md](agent/loop-goal.md) § *Standing decisions*,
> which is where a new one is taken, in the session that needs it, with its reason. Picking every
> dependency but the two the user named is pre-authorized under ADR 0051 § 4.

**How the plan relates to the ADRs.** The plan is the record of *what* gets built, in what order, and how
each milestone is verified. It states decisions but does not argue them. The reasoning lives in
[docs/adr/](adr/README.md), and where a decision has its own ADR the plan links to it instead of restating
it — follow the link rather than expecting the argument here. For decisions with no ADR of their own, the
*why* is in [adr/README.md](adr/README.md) under *Decisions taken at project start* and the *mechanics*
are [docs/plan/design.md](plan/design.md) § *Architecture*.

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

| Milestone | What it builds | Loop-days |
|---|---|---|
| [M0](plan/m0.md) | Project setup (~3 days) — **done** | 0.3 |
| [M1](plan/m1.md) | Front end (~3 weeks) | 0.7 |
| [M2](plan/m2.md) | HIR, types, IR (~4 weeks) | 1.5 |
| [M3](plan/m3.md) | Baseline Cranelift backend → **Hello World** (~3 weeks) | 0.5 |
| [M4](plan/m4.md) | Language completeness — a usable CLI language (~10 weeks) | ~3 |
| [M4S](plan/m4s.md) | The `Core` API contract and its pure half (~5 weeks) | ~1.5 |
| [M4B](plan/m4b.md) | Minimal `mwl-lsp`, syntax highlighting and the VS Code extension (~3 weeks) | ~1.5 |
| [M5](plan/m5.md) | Concurrency and script isolates (~5 weeks) | ~3.5 |
| [M6](plan/m6.md) | Config, limits, capabilities, disk cache (~3 weeks) | ~1 |
| [M7](plan/m7.md) | Built-in HTTP server (~4 weeks) | ~2 |
| [M8](plan/m8.md) | Stdlib and databases (~16 weeks) | ~6.5 |
| [M9](plan/m9.md) | Extension system (~6 weeks) | ~2.5 |
| [M10](plan/m10.md) | Developer tooling and IDE integration (~14 weeks; scope shifted by ADR 0040, net change undetermined) | ~8 |
| [M11](plan/m11.md) | PHP transpiler (~10 weeks) | ~3 |
| [M12](plan/m12.md) | Optimising JIT tier (ongoing) | measurement-bound |
| [M14](plan/m14.md) | Optional wasm32 browser target | not estimated |
| [M15](plan/m15.md) | Packages, the registry and the supply chain (~8 weeks; scheduled after M6) | ~3 + a calendar floor |
| [M16](plan/m16.md) | `mwl/web`, `mwl new`, and the framework (~12 weeks; scheduled after M7 and M8) | ~4 |

Each row is a file under [docs/plan/](plan/). `python tools/plan.py --show M8` prints one
without you needing to know that, and `--show M8:verify` prints only its acceptance paragraph.
The decisions those milestones sit inside, the architecture and the verification strategy are
[docs/plan/design.md](plan/design.md).

**The two columns are not the same unit.** The parenthesised weeks are the original estimate, written for
a human team before any code existed; **Loop-days** is what this project's unattended loop actually spends,
elapsed and continuous — M0–M3 are measured, the rest projected. The measured conversion is ~23×, it is
not applied uniformly, and it carries a rework tax and four ways it breaks:
[docs/plan/velocity.md](plan/velocity.md) is the one home for all of that. Summed, the milestones left
come to **~7 weeks** against the ~99 the original column still shows. Neither figure gates anything.
