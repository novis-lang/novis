# Novis — The Web-Native Programming Language: Implementation Plan

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
> backend) whole, M4S Part I registered — `crates/nvs-stdlib/tests/spec-members-outstanding.txt`
> holds no keys, which is this project's definition of *registered*. M1's own section lists the one
> grammar addition still owed (`autoload`, ADR 0061). Each milestone file under
> [docs/plan/](plan/) states its own acceptance.
>
> **On disk:** the workspace and its CI (three platforms, with miri, asan and fuzz legs), and the
> nine crates — `nvs-diagnostics`, `nvs-syntax`, `nvs-hir`, `nvs-types`, `nvs-ir`, `nvs-runtime`,
> `nvs-stdlib`, `nvs-codegen`, `nvs-cli` — plus `nvs-test`, `fuzz/`, `tools/`, `benches/abi-probe`,
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
> [docs/agent/loop-goal.md](agent/loop-goal.md), behind one surface change taken ahead of them: [ADR
> 0107](adr/0107-by-reference-parameters-are-spelled-inout-at-both-ends.md) retires `&` as a
> by-reference marker in favour of `inout`, written before the type and again at the call site, and
> **that surface is now the tree's**: `inout` parses in the modifier slot of a parameter, a
> `foreach` value binding and a destructuring leaf, `Arg` carries the same word at the call, and
> every `&` that meant by-reference is `E0237` naming the fix — including the two *returning* forms
> (`function &f()`, `&get`) that `inout` does not replace at all, since a method hands back a value
> rather than a place, and whose AST variants are deleted with them. The checker owns the call site:
> an argument binding an `inout` parameter must say so (`E0713`) and one binding a by-value
> parameter, a spread's entries or anything reached through a `callable` must not (`E0714`), so the
> write a callee makes to its caller's storage is visible where the call is written. The three
> spellings refused because the language has no such thing keep their own codes and deliberately do
> not become `E0237` — `$a = &$b` is `E0701`, `[&$x]` and its destructuring twin are `E0483`, `use
> (&$y)` is `E0224` — because `inout` is not what replaces any of them, and each gains the new word
> in its help text. `Parser::at_intersection_amp` is the one thing ADR 0107 said would be deleted
> and is not: with the fourth meaning gone an `&` in a type always *means* an intersection, but the
> type parser is what reaches `int &$x` first, and a lookahead that consumes it leaves the site
> above nothing to report — so § 3's own text is corrected rather than left disagreeing with the
> tree. The 12 `.nvst` cases and both examples that spelled `&$` are rewritten **with their expected
> output unchanged**, which is the check that this ADR changed no semantics; **items 46, 47a and 47b
> are all landed, and Stage 0a is closed**. The rename went through `nvs-ir`, `nvs-types` and
> `nvs-diagnostics` as one token substitution and then stopped at three places it must not touch:
> `$a = &$b` (`E0701`), `[&$x]` (`E0483`) and `use (&$y)` (`E0224`) are refused because Novis has no
> reference at all, so ADR 0107 replaced no marker of theirs, and `ExprKind::Assign::by_ref`,
> `ArrayItem::by_ref` and the use-clause parser's own locals keep the old word on purpose. Six
> diagnostic constants moved with the family — `E_INOUT_ARG_NOT_A_PLACE`,
> `E_INOUT_ARG_TYPE_NOT_EXACT`, `E_CLOSURE_INOUT_PARAM`, `E_FOREACH_INOUT_ELEMENT_TY`,
> `E_FOREACH_INOUT_SUBJECT` and `E_GENERATOR_INOUT_PARAM` — while `E_BY_REFERENCE_MARKER_RETIRED`,
> `E_ASSIGN_BY_REFERENCE`, `E_ARRAY_ELEMENT_BY_REFERENCE` and `E_CLOSURE_USE_BY_REF_UNSUPPORTED`
> each name a `&` and keep it. Two of the field docs were not renames at all but corrections:
> `ResolvedCall::inout` was recorded because "a call site's own syntax says nothing about it", which
> ADR 0107 § 2 falsified outright, and `nvs_types`' two `foreach` help strings still told a user to
> "drop the `&`" that is no longer in their source. Item 47 landed as **two** cases rather than the
> one ADR 0107 § *Verification* named, because a `.nvst` has one verdict:
> `a-by-reference-argument-is-written-inout-at-both-ends.nvst` **runs** the accepted shapes — the
> word before the type on a static method, an instance method and a `foreach` value binding, and
> again at an argument naming a local, a property and a named argument's outside — and
> `the-inout-marker-is-required-at-both-ends-and-replaces-every-ampersand.nvst` pins all three
> diagnostics in one compile, `E0237` at each of the five positions PHP writes `&` in, `E0713` at an
> unmarked argument and `E0714` at a marked one against both a by-value parameter and a call through
> a `callable`. That correction is folded into § *Verification*'s own body. The `.nvst` prose caught
> up with the source items 44–45 had already moved, so nothing in `tests/` outside a PHP oracle half
> and the two deliberate refusal cases spells `&$` any more, and neither does `docs/`: item 47b
> renamed every site that named Novis's own by-reference parameter — an alias's type rule (ADR 0007
> § 1), what cannot cross a `spawn` or a copy boundary (0006, 0023), what `class_alias` does not
> affect (0015), R3's banned out-parameter (0063), the `foreach` line in the spec's overview, M4's
> own feature list and four playbook bullets — and deleted `docs/agent/loop-goal.md` § *Stage 0a*,
> whose checks stay in `loop-goal.toml` as guards under names that now cite the ADR rather than a
> deleted item number. What still spells `&$` in `docs/` is quotation and nothing else: ADR 0107's
> own 17, ADR 0031's `use (&$y)`, the three refusals this field names above, and one routing-table
> row in `docs/adr/README.md` that keeps the retired spelling as a search key for a reader who has
> not heard yet. Stage 0a was first because a case authored in the old spelling is authored twice,
> and because M4B is deferred behind this goal exactly so its `.lspt` suite is written against the
> finished surface. A hole is a shape that compiles in the front end and then refuses below it; it
> is closed when it either runs with a fixture or a `.nvst` case pinning what it prints, or is
> refused by a **diagnostic that names the rule** — never by a panic. The statement dispatch has no
> shape left that the checker accepts: ADR 0007 § 3.3's `[int $a, string $b] = $pair;` lowers as the
> subscripts it is spelled out of and refuses what one refuses (`E0482`/`E0401`/`E0483`), inline
> HTML lowers over its raw span, a nested `class`/`interface`/`enum` is `E0233`, an increment takes
> its write target's own `E0479`/`E0478`/`E0480`, `unset()` is narrowed to an array element of a
> named holder and refuses every other operand (`E0234`, plus `E0413` for a static property), an
> element write whose root is only a temporary is `E0700` — the first code of the `E07xx` band the
> full `E04xx` one continues in — an increment's own target passes the parser's `E0105` gate like
> every other write spelling, the read-modify-write rewrite's own assertion has no reachable target
> left — its doc comment carries the proof, and `nvs_types`' two write-target refusals are two
> thirds of it — an element write evaluates the receiver under its root holder exactly once, PHP
> 8.5.9's own count, a computed member name (`->$name` / `->{expr}`) is `E0235` where it is written
> and an undeclared property is `E0405` on every class kind, which together leave the property-write
> panic no reachable target, all four write spellings — `=`, `⊕=`, an increment and `unset()` —
> agree on the three element-write holders that are no slot and take exactly one diagnostic each for
> it, an intermediate level of a nested element write is `E0482` unless it is an array of its own,
> which together leave `write_back_array`'s and `row_ty_of`'s panics no reachable target either, and
> `int $x;` and `;` both lower, all seven declaration spellings — `class`, `interface`, `enum`,
> `type`, `namespace`, `use` and `autoload` — are skipped at file scope and `E0233` inside a body,
> so an `autoload` written in the entry point itself resolves a class exactly as one in a bootstrap
> file does, and an expression used as its own statement is evaluated for its effects with its value
> discarded whatever shape it is, `$a = &$b;` being the one spelling refused instead (`E0701`, ADR
> 0031 § 2 has nowhere to put a reference) — which together leave both of the statement slice's
> catch-alls no reachable target. The **expression** dispatch is now the same: `Foo::class` folds to
> the class's fully qualified name as a `string` constant and `self::class`/`parent::class` with it,
> `$obj::class` and `static::class` are `E0702` — ADR 0008 binds `static` at the call, so folding it
> would silently answer the declaring class — an undeclared name in one is the ordinary `E0303`, PHP
> 8's `throw` lowers in expression position now that a union absorbs the `never` a non-completing
> branch contributes (`$v ?? throw new LogicError(…)` satisfies a `string`), a `yield` used as a
> value is `E0448`, `spawn script` is `E0703` (ADR 0006's isolates are M5) and `require` used for
> its value is `E0704` (`nvs-ir`'s known gap 22), which together leave `lower_expr`'s catch-all no
> reachable target either. The two **operator** catch-alls one level down are now the same. Unary
> `+` is the identity over `int`, `uint`, `float` and `decimal` and lowers to its operand with no
> instruction at all, which is safe rather than a silent divergence only because `-`/`+`/`~` over an
> operand ADR 0007 § 4 tabulates no row for — a `string`, a `bytes`, an `array<T>`, a `bool`,
> `null`, a `callable`, an enum case — is now `E0705` where PHP would have converted it first, an
> object keeping the "Novis has no operator overloading" sentence it already had; `@` error
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
> catch-all** goes with it: nine of `nvs_ir::ty::Ty`'s fifteen are rows — `null` newly among them,
> rendering as the empty string exactly as the `?string` holding one already did, which is PHP's
> answer and keeps the static and the tagged case agreeing — four are refused a phase up by the one
> check every implicit site shares (`E0707`: a `bytes`, an `array<T>`, an enum case and a `void`
> call, each naming the spelling that says what was meant, while the explicit `as string` keeps ADR
> 0009 § 3's `bytes` row), and `ClassDesc`/`Ref` are compiler-internal representations no source
> expression ever has. **The `as` conversion table** closes the same way one level down, and
> subtracting its grid found something worse than a panic underneath it. ADR 0007 § 2's table is a
> *closed* list of rows — `as` "either produces a value of the target type or throws", so a pair
> naming no row has nothing to produce and nothing to throw — and `nvs_types` now says so where it
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
> `Bar`'s slot list was then read off a `Foo`'s allocation. `nvs-ir`'s own catch-all has one target
> left and it is not this milestone's: ADR 0024 § 5's `string as Core\Html\Markup`, which is a
> *rule* rather than a test — a source-literal string and nothing else — and waits on `Core\Html`
> existing at all (M7). `array<T> as array<U>` was the other, and it closed as a **walk** rather
> than as a row. § 2's check is per *element*, and what checks one element is its runtime **tag** —
> the same four bits a closure parameter's entry check compares — so `Lowering::lower_array_restamp`
> reads the element type off the annotation, which is the only place it still exists, and hands one
> tag nibble per level of `U` to `Helper::ToArrayOf`. It could not have been a row of
> `Lowering::convert`: both sides of `array<int> as array<string>` erase to one `Ty::Array`, so that
> function took its free `from == to` row and handed the `int`s straight through under the other
> declaration, which is `$foo as Bar`'s type confusion one container in. `Helper::ToArrayOfOrNull`
> is ADR 0066's spelling of the same walk over one implementation, so the checked and the
> `null`-answering rows closed together. **Nothing is copied**: an Novis array is copy-on-write, so
> the result is the operand's own allocation under one more reference and whichever view writes
> first separates itself — § 5's invariance is bought with tag tests rather than with bytes moved,
> and that section's own "a real copy" sentence is corrected rather than left to disagree. What a
> tag cannot decide is refused where it is written (`E0711`, `reject_uncheckable_element_type`,
> which is that roster's one home): a **class** element is the fixed-offset confusion again, an
> **enum** would admit any integer as a case where ADR 0010 § 5's own row throws, and a **literal
> type** or a **union** admits some values of its representation and not others. `array<mixed>` is
> the target every tag satisfies, the way round all four, and the one shape that runs nothing at all
> from an operand already an array. The tagged operand into an object closed, and it closed as two
> answers rather than one. A **declared class** target is ADR 0007 § 6's checked way out of `mixed`,
> and it needed nothing new: `InstKind::InstanceOf` already takes a `Ty::Tagged` subject and already
> answers `false` for a tag that is not an object, so the row is that test, a `Terminator::Throw` on
> the false edge and one free `InstKind::Untag` on the true one — a `Helper` could not have carried
> it in any case, helper arguments being stored as `Value`s that a class descriptor is not. `$m as
> Plain`, `$m as Shape` through an interface and `?Plain as Plain` all agree with `$m instanceof
> Plain` for every tag a `mixed` can hold, and the ownership is `convert`'s own free row split
> across the two edges: `Untag` is a relabelling, so a borrowed operand is retained on the way out
> and a fresh one transfers instead, and the false edge releases a fresh one before it throws rather
> than abandoning it on the edge — valgrind-clean over a fixture that converts, and fails to
> convert, two hundred times. Every **other** object target names no class to test against — plain
> `object`, a shape, a `callable`, and a `Core` class, which has no descriptor in the unit for the
> same reason `instanceof Core\Uri` is `E0496` — so from an operand that is not already an object
> the conversion could only assert a tag it cannot verify, which is `$foo as Bar`'s type confusion
> one step earlier, and `nvs_types` refuses it where it is written (`E0711`). `$plain as object`
> stays the free widening row it always was, and `Core\Html\Markup` is the one `Core` exemption, its
> own row being `nvs_types::expr::quals`' to own. The third was a tagged operand into `bytes` and it
> is a row now: ADR 0009 § 3's pair is the operand's own *tag*'s wherever its static type names
> neither side of it, so `Helper::TaggedToBytes` hands the same allocation back under the other tag
> for a `string` or a `bytes` and throws for every tag § 2's table gives no row. It is the only
> shape of `as bytes` that reaches a call at all, the statically typed spelling being a free
> `Reinterpret` over that same allocation. `lower_expr`'s dispatch message is the assertion its
> roster already proved. One level *up* from all of it, **a digit run beside a `uint` is now placed
> at `uint`** rather than defaulting to `int` — ADR 0007 § 2's "untyped until placed" applied to the
> one placement a binary operator offers, its other operand — so `$u + 1`, `$u & 3` and `$u << 1`
> compile at all, where each of them used to be § 4's mixed-signedness refusal and a `uint` could
> meet only a `uint`-declared local; a digit run above `i64::MAX`, which § 4 admits "only where a
> `uint` is expected", has an operand position for the first time, and `nvs-ir` makes the same
> placement on the left-hand operand so that it does not then panic on a value that never fit an
> `int`. What stays refused is the pair with no digit run in it: a shift's *count* is an operand of
> the operator rather than a bare width, judged by the row its left operand takes, which is exactly
> what makes `nvs-codegen`'s `emit_shift` sound in reading one signedness for both the
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
> and the three never fire on one expression. Under `as ?T` the opposite direction — a row § 3 calls
> **available** with no `?` helper to run it — is closed too, and it needed no helper of its own:
> `$m as ?array<U>` is the element walk again, answering `null` in exactly the places the checked
> spelling throws. The `bytes` target closed with its checked twin: `Helper::ToBytesOrNull` shares
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
> a class and `nvs-ir` calls it, unchanged; where it names none — an erased `object`, a `mixed`, any
> other union — `nvs_runtime::stringify` asks the concrete instance's class for the same member,
> which is ADR 0036 § 4's deferral applied to the member access ADR 0028 § 1 says the conversion
> *is*. So `echo $o`, `"" . $o` and `$o as string` are one answer for one value where they used to
> be a rendering, a throw and a panic, and a class that declares no `toString` throws catchably,
> naming itself and the interface. The `Core`-owned half is now the rendering half alone. Its
> *refusal* is where it is written: `require_stringable` asks `nvs_stdlib::registry::class_renders`,
> and a class it answers `false` for is `E0710` at the site rather than a throw below it — `echo`,
> an interpolated piece, a `.` operand and `as string` agreeing because they are one check. Two
> rosters answer, and they are two rules: `Core\Uri`, `Core\Uuid` and `Core\Time\Duration` have a
> `toString` row, and the two sink carriers render through ADR 0088 § 5 with no member at all, which
> is `nvs_runtime::is_carrier`'s list read rather than copied. The class that *does* render now
> renders. `require_stringable` records the same resolved `toString` target for a `Core` class that
> a declared one gets — a `Core` member resolves out of the seeded signature table like any other —
> and `nvs-ir` asks `core_symbol_of` which of the two calls to emit, so it takes the native
> `InstKind::CoreCall` the member written out takes rather than a `CallVirtual` into a method table
> a `Core` class has no entry in. All four rendering spellings therefore agree with
> `$uri->toString()` for each of the three classes the spec gives one, and the receiver's ownership
> inverts with the call: a native member *borrows* argument 0, so a fresh receiver (`echo
> Core\Uri::parse(…)`) is the rendering site's to release rather than the callee's, which is
> valgrind-clean over a fixture that renders in a loop. That row is closed at its other end too, and
> the erased half renders through the very same member. `nvs-runtime` sits below `nvs-stdlib` and
> cannot read the registry, so what the two share is the **descriptor**: `nvs_stdlib::instance` puts
> the class's registered `toString` on it as `ClassDesc::renderer`, derived from
> `registry::class_renders` — the check the compiler already makes at a written `echo` — rather than
> written down a second time, and `nvs_runtime::stringify` asks for that before the compiled method
> table a `Core` class has no entry in. It is deliberately not a row *in* that table, because the
> two calling conventions differ: a compiled method owns its parameters while a native `Core` member
> borrows argument 0, so which descriptor field an address came out of is what tells the caller
> which reference it owes. So `echo $m`, `"$m"`, `"" . $m` and `$m as string` over a `mixed` holding
> a `Core\Uri`, a `Core\Uuid` or a `Core\Time\Duration` all answer what `$x->toString()` answers,
> valgrind-clean over a fixture that renders a borrowed and a fresh operand two hundred times each,
> while a `Core` class the registry gives no `toString` throws catchably where there is no site to
> refuse it at. The two sink carriers are unchanged and needed nothing: ADR 0088 § 5 renders one as
> exactly the bytes it carries, with no member asked for at all, behind a `mixed` as through its own
> type. Two agreement tests keep the halves in step —
> `a_core_class_stringifies_exactly_where_the_registry_says_so` asks the checker and the registry
> the same question, `every_rendering_class_carries_a_renderer_or_is_a_carrier` asks the descriptor
> and the registry theirs — so a `Core` class cannot render where it is written and throw where it
> is not. **A named and a spread argument now lower at both kinds of call site**, which is item 16
> closed at both ends. A `name:` argument lands at the ABI position of the parameter its *name*
> reached rather than at its own place in the list, with every parameter no argument filled taking
> its own default in declaration order and evaluation staying in **written** order above the call —
> `nvs_types`' `ResolvedCall::arg_slots` is that mapping and `nvs-ir` only reads it, a name
> resolving against a `MethodSig`'s parameter names that no later pass holds. A `...` argument is
> one `array_spread` of the subject into the array a variadic tail already is, so how many arguments
> arrive is the subject's own run-time length rather than anything the site counted. Through a
> `callable` there is no signature to map either against, and that is where the two halves part
> company. A `name:` is refused where it is written (`E0712`): ADR 0031 § 1 gives `callable` no
> parameter list at *either* end, a closure value recording its arity and its parameter tags and
> never their names, so PHP's spelling has nothing to resolve against here at all. A `...` needs no
> parameter list to mean something, so it lowers — the whole argument list becomes one array behind
> `Helper::CallClosureArray`, which is a second helper rather than a wider `CallClosure` because
> that one's argument count is a literal `nvs-codegen` writes beside the argument slot, and a
> spread's count is exactly the fact that is not known there. Both reach the one
> `nvs_runtime::call_closure` every `Core` member's callback already does, through one shared arity
> check, so too few arguments is the same catchable `LogicError` and there is no second convention
> beside it; too many are trimmed, which is PHP's answer for a userland call as well as the spec's
> "a callback may declare fewer parameters". Valgrind-clean over a fixture that spreads a borrowed
> and a freshly built argument list two hundred times each. **An `inout` argument's copy-back now
> lands where the call is**, which is item 18 and PHP's own sequence point, so such a call lowers in
> any expression position at all rather than only as a bare statement or a plain assignment's
> right-hand side. A call site takes `Lowering::pending_refs_mark` before it lowers its argument
> list and hands that mark back to `flush_ref_writebacks` once its call has returned, which is what
> makes the staging list a stack rather than a queue drained at a boundary:
> `Adder::sum(Adder::bump($n), $n)` stages `$n` for the *outer* call before the inner one's
> arguments are lowered at all, so a flush that drained the whole list would write the outer slot
> back before the outer call had run and then lose that call's own write. Under a `?->` the
> copy-back lands inside the guard, where it belongs — a receiver that was `null` ran no callee and
> wrote nothing back, and the old statement-level flush read a slot defined only in the branch it
> skipped. The four statement-level flush sites are gone with it, and `lower_stmts`' assertion
> survives as an internal-consistency check on the call sites rather than as a refusal of the
> program. What this does **not** buy is PHP's *operand* order, and it is not meant to: Novis
> evaluates a binary operator's operands strictly left to right, so `$n + Adder::bump($n)` reads the
> left `$n` before the call and answers `5 + 7` where PHP's compiled-variable read at the `ADD`
> answers `7 + 7`. PHP's own manual leaves an expression's operand order undefined, so there is no
> specified behaviour here to be compatible with, and `Lowering::pending_refs` is that decision's
> one home. Subtracting the deferral found a leak underneath it that was older than it and that no
> fixture had reached: `return $s;` names its own local as the one binding `release_all_locals`
> skips, transferring that binding's reference straight out instead of retaining it — and an `inout`
> parameter is a `Ty::Ref` cell `release_all_locals` was never going to release in the first place
> (the caller's copy-back owns that reference), so the exemption lost the retain outright and the
> caller then freed a value its own staged slot still owned. `nvs run` printed the right answer and
> exited 127. The exemption is decided by the binding's representation now, and the pair is
> valgrind-clean over a fixture that grows a borrowed and a freshly built string through an `inout`
> parameter, and writes back through a property holder, two hundred times. **A `finally` now runs
> when its own `catch` clause's body throws**, which is item 12 and the last exit out of a protected
> region that did not run one. The clause body is lowered under a frame whose handler is that
> region's own finally-and-re-raise block rather than the dispatch that selected the clause — a
> clause does not catch what its own body raises — so the `finally` runs, the clause binding is
> released there exactly as the completing path already released it before lowering its own copy,
> and the new exception is handed on to the enclosing region carrying the same reference it arrived
> with. A `finally` that throws on its own way out therefore **replaces** the exception in flight,
> which is PHP's answer and falls out of the ordering rather than being written down anywhere: the
> re-raise's own `Terminator::Throw` is simply never reached. Where the region has no `finally` the
> frame names no handler at all and such a throw still reaches the enclosing region directly, so
> nothing is spent on a `try`/`catch` that owes nothing. Valgrind-clean over a fixture that wraps,
> re-raises through the same object, catches unbound and crosses a frame, two hundred times each.
> `tests/conformance/error/a-finally-runs-when-its-catch-body-throws.nvst` pins six shapes
> byte-for-byte against PHP 8.5.9's own output, an unbound clause, nested regions running innermost
> first and the replacing `finally` among them; `nvs-ir`'s known gap 2 loses its first half and
> `nvs-codegen`'s gap 0 is deleted outright, its two still-true sentences folded into the paragraph
> above that list. Item 18's two named cases land with it —
> `tests/conformance/lang/a-reference-argument-is-written-back-before-the-next-read.nvst` over the
> seven positions a staged call takes, and the oracle twin
> `tests/differential/lang/a-reference-argument-matches-phps.nvst`, which deliberately writes no
> read to the *left* of such a call: that is the operand-order divergence `Lowering::pending_refs`
> owns, PHP's manual leaves it undefined, and it is not a difference to pin. **An abandoned
> generator's `finally` runs**, which is item 13 whole and the item that had the design call in it.
> Every generator's state class carries a fourth synthesized method, `{name}$gen::gen#unwind`, and
> one more `Ty::Int` field, `gen#unwind`: the method reads the parked state, and where that state
> names an actual suspension — anything above `0`, since `0` is "never entered" and so no `try` has
> been entered either — it raises the flag and re-enters `advance()`, whose entry switch lands on
> that suspension's own resume block. A resume block that sits inside a `finally`-owning region
> grows a branch on the flag, and its unwind arm is lowered as **exactly what `return;` lowers to at
> that point** — `run_pending_finallys` over every enclosing region, innermost first, then
> `finish_generator` — so the ladder, the per-binding releases and the exit are the body's own
> rather than a second copy of the rules. A suspension owing nothing grows no branch at all, which
> is why a generator with no `finally` lowers byte-identically to before. **It is not a destructor
> and re-opens nothing in ADR 0028 § 2**, whose body now says so in its own paragraph: no class
> declares anything, no method name is recognized, no object gains a lifecycle hook, and the only
> code that runs is code the program had already entered and suspended inside. One convention is
> deliberately inverted and it is the reason the design works: **`unwind` borrows argument 0**,
> alone among compiled methods, because the release path reaches it at the moment a count has
> already hit zero — a consuming convention would ask that caller for a reference it no longer has,
> and `advance`'s own release on the way out would then cross zero a second time and re-enter the
> release path on the allocation it is already dismantling. The retain inside `unwind` pairs with
> that release, so the count it is handed is the count it leaves behind. The other end is now
> `nvs_runtime::object::dismantle`, which calls that entry point before it sweeps the field slots —
> where the parked locals the `finally` body reads still are — and three decisions make the call
> safe, each recorded where it is made rather than in an ADR. **The name is unspellable**: the
> method is `gen#unwind` rather than `unwind`, because the probe is made against *every* dying
> object's class and a name a program could declare would turn a user method into the destructor ADR
> 0028 § 2 says Novis does not have. **It is a descriptor field, not a probe**:
> `ClassTable::set_methods` resolves the one row once per class into `ClassDesc::unwind`, the
> precedent `ClassDesc::renderer` set, so a dying object that is not a generator pays a null test
> rather than a binary search over its whole method table. **The entry point resumes only a
> suspension that owes a `finally`** — the state values whose resume block grew an unwind arm are
> collected while `advance()` is lowered and tested for membership, because a `state > 0` test
> resumes a suspension that owes nothing and carries on running the body, which is the opposite of
> abandoning it; a generator with no owed state carries no entry point and no method row at all,
> which is what keeps it lowering exactly as it did. **The count is resurrected to one first**,
> since `gen#unwind` borrows and `advance()` releases — a pair that would otherwise cross zero and
> re-enter the release path on the allocation already being dismantled — and the allocation is freed
> below whatever the count then reads. The context the call needs comes from the thread rather than
> from a parameter: `nvs_runtime::ctx::CurrentCtx` is installed by `abi::call`, the one door from
> Rust into compiled code, because threading a context through every release primitive would put a
> parameter on the hot path of every decrement in the language to serve the one release in ten
> thousand that frees a suspended generator. One divergence from PHP is left and it is deliberate: a
> throw escaping such a `finally` is **discarded**, a release having no error edge to report it on
> and a landing pad's own in-flight exception being the thing that would otherwise be replaced —
> `Ctx::with_pending_set_aside` is that decision's home, ADR 0028 § 2's own paragraph now says so
> instead of claiming an error edge it does not have, and surfacing it wants ADR 0020's ladder. Item
> 13's two named cases land with it,
> `tests/conformance/iter/an-abandoned-generator-runs-the-finally-it-is-suspended-inside.nvst` over
> six shapes — abandoned inside the region, drained (the `finally` runs once and the release does
> not re-run it), never entered, two nested regions innermost first, and a suspension the region
> does not cover, which owes nothing — and the oracle twin
> `tests/differential/iter/an-abandoned-generators-finally-matches-phps.nvst`, which adds an
> abandonment made while an exception is in flight and agrees with PHP 8.5.9 byte for byte.
> Valgrind-clean over four scratch fixtures covering the same shapes, the throwing `finally` among
> them. **ADR 0007 § 4's ordering row is closed at both ends**, which is `emit_binop`'s
> representation catch-all subtracted from the operator end rather than from the backend's. That
> table orders the numeric types against each other and, through ADR 0013, two objects of one
> `Comparable` class; everything else PHP orders it orders by **converting** an operand first, which
> § 2 never does by itself, so an operand the table does not name has no `<` at all rather than a
> plausible answer below it. `nvs_types`' `reject_unordered_operand` refuses it where it is written
> (`E0715`), each help naming the spelling that says what was meant — `Core\Str::compare` for two
> strings, `as int` for an enum case, and nothing at all for a `bytes`, an `array<T>`, a `callable`
> or `null`, which have no ordering to name. Two rows go the other way and are answered rather than
> refused: `bool` against `bool`, which is the ordering of the one bit it already is and is PHP's
> answer too, and `null == null`, which reached the same catch-all and is now a constant in
> `emit_binop` — both operands are the single value the type has, so ADR 0090 § 2's disjointness
> check passes the pair and there is nothing to compute. The **object** family keeps ADR 0013's own
> `E0411` however the receiver was spelled, an erased `object` and a shape included, so "these two
> do not order" reads as one diagnostic rather than as two codes divided by how much the checker
> happened to know. What still reaches that catch-all is `Ty::Tagged`, whose arithmetic and ordering
> are item 24, and the three representations no source expression has — the site's own comment
> carries that roster, as the statement and expression dispatches' do. ADR 0007 § 4's table gains
> the two rows and the closure sentence rather than being left to disagree with the tree. **Item 25
> was already landed and what it owed was the check**, which is now made and pinned: `object` erases
> to the pointer a named class does, and the two descriptor fields anything below that erasure reads
> — `ClassDesc::renderer` for a rendering and `ClassDesc::unwind` for an abandoned generator's
> `finally` — are both found from the *instance* rather than from the static type, so a suspended
> generator dropped through an `object`-typed binding runs the same `finally` a `foreach` temporary
> does. **The ordering half of item 24 is closed**, which is `emit_binop`'s representation catch-all
> subtracted a second time and from the same end. An operand whose static type names no ordering row
> at all — `mixed`, a union, the `int|float` a division returns — cannot be judged where it is
> written, so it is judged from its runtime **tag**: `Helper::ValueLt`, `ValueLtEq` and `ValueCmp`
> answer § 4's rows the tags name, `>`/`>=` being the first two with their operands swapped so that
> a `NaN` operand is false for all four at once and `1` under `<=>`, which is PHP's answer. Where
> the tags name none the *same* refusal `E0715` makes statically arrives as a catchable throw
> carrying that diagnostic's own wording — the one comparison helper family with ADR 0002's error
> edge, which is why each operand goes on the owned-temporaries stack rather than being released
> inline: a throw here has an edge to leave by. Two consequences fall out of the tag being all there
> is, and both are recorded in ADR 0007 § 4's own closure rather than left to be rediscovered: two
> objects behind two `mixed`s throw, `Comparable::compareTo` being dispatched from the class the
> *site* named, and an enum case orders as the integer ADR 0047 § 5 spends no representation on
> hiding, the written spelling still being refused. Valgrind-clean over a fixture that throws two
> hundred times with a freshly built operand in flight. **Item 24's other half is closed, and with
> it every `Ty::Tagged` target `emit_binop`'s representation catch-all had.** An operand whose
> static type names no *arithmetic* row either — `mixed`, a union, the `int|float` a division
> returns — is answered from its runtime **tag** by an eleven-member `Helper::ValueAdd` family, one
> helper per operator over one table (`nvs_runtime::helpers::value_arith`) rather than eleven copies
> of the dispatch. The rows are § 4's own: `int ⊕ int` and `uint ⊕ uint` checked at every step,
> overflow throwing rather than wrapping; `/` answering the `int|float` union PHP is exact about, so
> the *answer*'s tag is still a runtime question once the operands' are known; "either operand a
> `float`" widening the integer side through the very helper `$n as float` emits, exact or throwing
> above 2^53, rather than through a silent `as f64` one representation down; and ADR 0054 § 3's five
> `decimal` rows over the same `Decimal` methods the statically typed helpers call, so the two ends
> of a row cannot answer differently. Three refusals arrive as catchable throws where only the tags
> can make them, each carrying the wording `nvs_types` uses where the static types show it: a pair
> the closed table names no row for, `int ⊕ uint`'s absent common type (`E0407`'s own sentence), and
> the overflow itself. Two rows are deliberately narrower than "the operands are numbers": a
> `decimal` under `**` or a bit operator is `E0455`/`E0706` where it is written and a throw here,
> and `float` `%` is refused at **both** ends — `nvs-codegen` lowers no static one either — because
> PHP's `%` converts to an integer where § 4's float row would not, and no ADR settles which of the
> two it is. All eleven answer `Ty::Tagged`, which `Lowering::coerce` absorbs into a declared type
> by the rows it already absorbs integer `/`'s union with. The catch-all's roster comment is
> corrected to what is actually left: the three representations no source expression has
> (`ClassDesc`, `Ref`, `Void`), the `float` rows never reaching it at all. Valgrind-clean over a
> fixture that abandons a freshly built `string` operand on the refusal's error edge two hundred
> times, and `tests/conformance/lang/arithmetic-over-a-mixed-operand-is-decided-by-its-tag.nvst`
> pins sixteen lines of the table, the six throws among them. Three live tools **are** the worklist
> and no session re-derives one: `python tools/holes.py` reads the refusal sites out of `nvs-ir` and
> `nvs-codegen` and attributes each to its item (`--item N` for one in full), `python tools/loop.py
> --list` prints the named `.nvst` cases each stage still owes, and `python
> tools/check-migration.py` scores `docs/spec/02-php-migration.md`.
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
| [M4B](plan/m4b.md) | Minimal `nvs-lsp`, syntax highlighting and the VS Code extension (~3 weeks) | ~1.5 |
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
| [M16](plan/m16.md) | `nvs/web`, `nvs new`, and the framework (~12 weeks; scheduled after M7 and M8) | ~4 |

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
