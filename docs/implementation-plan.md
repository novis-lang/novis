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
> pins sixteen lines of the table, the six throws among them. **`emit.rs`'s six remaining internal
> panics carry their roster now, and subtracting one of them found a hole rather than a check.**
> Five are internal-consistency checks and say so in the shape `emit_binop`'s comment set:
> `reinterpret` between two representations that do not share a machine type, which `nvs-ir`'s three
> producers of that instruction cannot be — ADR 0010 § 5's enum rows in either direction and ADR
> 0009 § 3's `string as bytes`, each a relabelling of one allocation; the tagged widen/narrow pair,
> whose only producer is `Lowering::coerce` and whose refused targets are the identity it answers
> first (`Ty::Tagged`) and a `void` call's result no position can declare; the refcount one, which
> is `Ty::is_refcounted`'s five rows with `Ty::Tagged` taken out of line, so an arrival is a
> `nvs-ir` site that emitted a retain without asking that predicate — the playbook's widened-operand
> trap; and the terminator and runtime-helper catch-alls, which cover all seven `Terminator`
> variants and all 76 `Helper` ones and exist only because both enums are `#[non_exhaustive]` in a
> downstream crate. The **sixth was a hole**: `reject_unary_arith_operand` refuses an operand ADR
> 0007 § 4 tabulates no row for, but a `mixed` names no row *and* no refusal, so `-$m` reached that
> catch-all and `nvs run` printed "does not lower the unary operator Neg over representation
> Tagged". It is closed the way item 24 closed the binary half, and from the same end:
> `Helper::ValueNeg` and `Helper::ValueBitNot` answer § 4's unary rows from the operand's runtime
> **tag**, over `nvs_runtime::helpers::value_neg`/`value_bit_not`, whose doc comments are those
> rows' home. `-` is the four numeric types, the two integer ones `checked_neg` so that `-i64::MIN`
> and every non-zero `uint` throw rather than wrap, worded exactly as `emit_unop` words the
> statically typed row's; `~` is `int` and `uint` alone, the same narrowing `E0706` makes where the
> static type shows it, so a `float` or a `decimal` operand is a number with no bit pattern to
> complement. Every other tag is the closed table's refusal as a catchable throw carrying `E0705`'s
> reading. Unary `+` gains no member because it has no row: it is the identity over all four numeric
> types, so `nvs-ir` returns the operand itself with no instruction, a tagged operand included. The
> result is `Ty::Tagged` for the `ValueAdd` family's reason — `-$m` is an `int`, a `float`, a
> `decimal` or a throw — and the operand is staged on the owned-temporaries stack rather than
> released inline, both helpers carrying ADR 0002's error edge. Valgrind-clean over a fixture that
> abandons a freshly built `string` operand on that edge four hundred times, and
> `tests/conformance/lang/a-unary-operator-over-a-mixed-operand-is-decided-by-its-tag.nvst` pins
> fifteen lines of the pair, the seven throws among them. The unary catch-all's own roster comment
> is now `emit_binop`'s residue exactly: the three representations no source expression has. **ADR
> 0007 § 4's arithmetic table is closed at the operand end now, and closing it took `emit_binop`'s
> operator catch-all with it.** That table's operands are the numeric types — `int`, `uint`,
> `float`, and `decimal` through ADR 0054 § 3 — and everything else PHP adds it adds by *converting*
> first, which § 2 never does by itself, so a `bool`, a `string`, a `bytes`, an `array<T>`, a
> `callable`, `null` and an object have no `+` at all and are refused where they are written
> (`E0716`), an enum keeping ADR 0010 § 5's own `E0415` so that one rule draws one code. `bool` is
> the operand the refusal exists for and the one that was worse than a panic: `nvs_ir::ty::Ty::Bool`
> is `nvs-codegen`'s `integral`, so `true + true` reached an `iadd` over the `i8` a `bool` is stored
> in and `echo`ed `1` where PHP prints `2`, while `$s - $s` and `$xs / $xs` reached the
> *representation* catch-all one gate earlier and only refused. The second half is `%` over a
> `float` (`E0717`), the one refusal both operands are numbers for: PHP converts both to an integer
> and answers one, § 4's own float row would answer a `float`, and the spec's `Core\Math::mod` row
> settles it the third way — "integer `%` is the operator" — so it is refused at **both** ends,
> `nvs_runtime::helpers::value_arith` raising the same rule as a catchable throw where only the tags
> can see it. ADR 0007 § 4 gains the closure paragraph and the `%` caveat on its float row rather
> than being left to disagree with the tree. What that leaves `emit_binop`'s operator catch-all is
> nothing: everything reaching it shares one representation and it is `float` or `bool`, the two
> rows the six early returns above do not handle, and the five surviving pairs are `Shl`/`Shr` over
> either (`E0706`), `Div`/`Mod`/`Pow` over a `bool` (`E0716`) and `Mod` over a `float` (`E0717`) —
> its roster comment now says so in the shape the other five carry, and the representation
> catch-all's own is corrected where it named `float` `%` as its neighbour's target. **Item 26 was
> already closed at the language level and what it owed was the fixture.** ADR 0007 § 2's grid gives
> a `bool` source the "anything → `string`" row and no numeric one, so `bool as string` runs through
> `Helper::BoolToString` (pinned by `conversions-that-succeed.nvst`) and `$yes as int` is the
> `E0708` the closed-grid pass already landed (pinned by `the-conversion-table-is-closed.nvst`);
> `examples/targets.nvs` still spelled the refused half, so it is a branch said out loud there now
> and `asInt=1` is unchanged. **Item 24's subscript half is closed, and closing it took
> `examples/targets.nvs` green with it.** A base whose static type names no element type at all was
> `E0482` wherever it was written, `mixed` included — but `mixed` is ADR 0007 § 2's one unchecked
> position, so it defers not only *which* array is behind the handle but *whether there is one*,
> which is ADR 0036 § 4's deferral one storage kind along from a member access. It is answered from
> the base's runtime **tag** now: `Helper::ValueIndexGet` and its `??` twin `ValueIndexOptionalGet`
> share one implementation (`nvs_runtime::helpers::value_index`, whose doc comment is those rows'
> home) and are chosen in `nvs-ir` off the base's *representation* rather than off anything the
> checker recorded, exactly as `lower_instanceof` reads its own subject's. They are a `Helper` pair
> rather than a widened `InstKind::ArrayGet` because the two differ in the one row that must not be
> shared: a non-array base is an **internal inconsistency** for the statically typed read, whose
> base is an `array<T>` by declaration, and a **catchable throw** here, carrying `E0482`'s own "only
> an `array<T>` has elements" wording so that the deferred refusal reads as the one the site makes
> wherever the type shows it. Under a `??` both failures — the absent key and the non-array base —
> answer `null` instead, which is what PHP's own null-coalescing read does for any subject at all.
> Two ends stay refused deliberately and neither is a gap: a base whose *declared* type has already
> answered the question (a scalar, an untested `?array<T>`, a union naming no array) keeps `E0482`,
> because a type that answered does not get to ask again at run time; and a `mixed` **write** target
> keeps it too, an element write having a copy-on-write buffer to separate and needing a holder to
> write the separated one back through, which a value that is only a tag does not name. ADR 0007 § 5
> gains the paragraph rather than being left to disagree with the tree. Valgrind-clean over a
> fixture that abandons a freshly built `string` base on the refusal's error edge, and a freshly
> built array on an absent key's, two hundred times each. Subtracting the refusal moved
> `examples/targets.nvs` one line further and found the last one wrong rather than unimplemented:
> `array<int> as array<string>` is § 2's per-element **check**, not a conversion, so it throws at
> element 0, and the fixture converts to `array<mixed>` — the one target every tag satisfies — with
> `converted=3` unchanged, which is what makes that the fixture's own correction rather than a
> weakening of the check. **The fixture is green end to end and matches Stage 4's `want` exactly.**
> **A call that returns `void` is not an operand of anything**, and that refusal is one step earlier
> than every other one in the band. ADR 0007 § 4's rows are about the type a value *has*, and a
> `void` call has no value for a row to be about, so the question of which row applies never arises.
> `nvs_types::expr::operators::reject_void_operand` refuses it where it is written (`E0718`), ahead
> of the whole table and ahead of the "type not yet known" pass-through the arithmetic, ordering and
> bitwise refusals share — `equality_domain` answers `None` for `Ty::Void` exactly as it does for
> `mixed`, but `mixed`'s answer comes from a runtime tag it *has* and this one has no value to carry
> one. Every binary operator is covered rather than the two the item named, `==`, `&&` and `??`
> among them, because it is one rule and the operator it was written under changes nothing about it;
> the three arithmetic prefixes take the same code through `reject_unary_arith_operand`, unary `+`
> included, the identity of nothing still being nothing. `.` is the one exception and keeps `E0707`,
> whose roster already names a `void` call among the four types with no implicit `string` form, so
> one rule still draws one code. Unrefused, none of these reached a diagnostic *or* an answer:
> `nvs-ir` lowers a `void` call to no value at all, so `V::nothing() + 1` failed the whole
> compilation with "nvs-codegen does not lower an operand used before it is defined", naming a
> compiler bug for what is a mistake in the program.
> `tests/conformance/lang/a-void-call-is-not-an-operand.nvst` sweeps fifteen spellings in source
> order with the `.` line among them, so a family that stopped refusing shifts that line rather than
> answering plausibly. **The same value in a *condition* is closed now too, and it took a second
> code rather than a wider first one.** ADR 0035 makes a condition the one place a value is tested
> without `as`, and its truthy table has a row for *every* type — which is exactly why a `void` call
> shows up there as nothing at all rather than as a mismatch, and why the operand refusal's own
> sentence ("not an operand") is the wrong one to print under `if (V::nothing())`, where no operator
> is written. So the line between the two is which table has no row rather than which syntax was
> used: `&&`, `||` and `??` are ADR 0007 § 4's operands and keep `E0718`, while the four statement
> conditions, a ternary's, the elvis spelling, `!` and `empty()` take `E0719`,
> `code::E_VOID_IS_NOT_A_CONDITION`'s own doc comment being that split's home. `nvs_types` had no
> single site that asked what a condition's type is, which is why the hole was there at all; it has
> one now — `nvs_types::expr::check_condition` wraps the check every statement condition and the
> ternary already made, and `!` and `empty()` report from their own arms, having inferred their
> operand for a reason of their own. Unrefused it reached no diagnostic and no answer either:
> `nvs-ir` lowers a `void` call to no value, so the truthy slice panicked on a representation its
> table has no row for. `tests/conformance/lang/a-void-call-is-not-a-condition.nvst` sweeps the
> eight positions in source order with the `&&` line below them, so a position that stopped refusing
> shifts that line rather than answering plausibly. **The five catch-all rosters gained the case
> that asserts they agree rather than what each answered.** `emit_binop`'s representation half and
> its operator half, `emit_unop`'s, and `nvs-ir`'s `lower_expr` and `concat_operand` ones each say
> `Ty::Tagged` has left them entirely — the tagged rows being chosen from an operand's runtime tag
> one crate up — and that claim is testable only as an agreement:
> `tests/conformance/lang/an-operator-agrees-whether-its-operand-is-typed-or-tagged.nvst` asks 45
> questions twice, once where the static type names ADR 0007 § 4's row and once through a `mixed`
> holding the same value, and counts the agreements rather than reading a line off each. Equality
> over all six representations that have one, the arithmetic, bitwise and ordering rows, the three
> unary ones, six of `concat_operand`'s nine and the five shapes `lower_expr` takes above its
> general `Binary` arm are the rows, so a family that grows a second answer fails here while still
> looking right on its own line. **A `match` over a tagged subject is that same equality row now**,
> which is the one shape the agreement case was missing and the last `BinOp::Eq` in `nvs-ir` emitted
> without asking whether either side is a tag. `Lowering::lower_match` compares each label through
> `Helper::Identical` wherever the subject's or the label's representation is `Ty::Tagged`, exactly
> as `lower_binary` already did for a written `==`, and the label itself is untouched by it: a digit
> run beside a `mixed` is still a `ConstInt`, `nvs-codegen`'s helper convention storing every
> argument as a 16-byte tagged `Value` already, so there is no widening to emit. The assert that
> used to refuse the shape survives as an internal-consistency check with the roster its neighbours
> carry — `nvs_types` has already made every label comparable with the subject (`E0466`, ADR 0090 §
> 6) and the tagged arm takes the one pairing whose types name no static row, so an arrival is a
> `nvs-ir` site that lowered a label against an expectation it then did not honour. The agreement
> case gains three rows for it — an `int` subject that hits, a `string` one that hits and one that
> falls off to `default` — and reads `agreed=48/48`. An **enum** subject is the last `match` shape
> that was refused below the checker, and it is closed one representation down rather than by a new
> row. `Ty::Enum` is a zero-byte tag over an integer and `nvs-codegen`'s `BinOp` table carries no
> row for it, so `Lowering::lower_match` relabels the subject once above its label chain and each
> label as it is lowered, through the free `Reinterpret` of ADR 0010 § 5 row 1 that a written `==`
> between two cases and ADR 0047 § 5's membership chain already used — the same move at the two
> sites that had not made it yet, `Lowering::lower_switch` being the second and failing for exactly
> the same reason. Neither relabelling is what the ownership reads: the subject's own value still
> feeds the arm-entry release and a label's still feeds its own, so a refcounted subject under an
> enum-free `match` lowers byte-identically to before, and a label still lowers at the subject's
> *declared* representation so that `Mode::Read` resolves as the case it names. Both assertions
> survive as internal-consistency checks over the relabelled pair.
> `tests/conformance/enum/a-match-and-a-switch-over-an-enum-compare-on-the-backing-integer.nvst`
> pins both backings, a label that hits, a fall-off to `default`, the fall-off with no `default` at
> all (which throws) and a `switch` over each, and the agreement case gains the row that asserts the
> typed and the tagged halves answer the same thing, reading `agreed=49/49`. `emit_binop`'s residue
> roster names all four sites that compare an enum now rather than the two it had. **A `foreach`
> subject that is not iterable was already closed and owed only its case.** ADR 0053 § 3's list is
> closed, so `nvs_types::expr::iteration::report_not_iterable` refuses every subject naming none of
> it (`E0443`) — a `void` call among them, one step earlier than the rest for the reason `E0719`
> exists, and neither the condition's code nor the operand's, a subject being neither — and nothing
> in `tests/` had ever pinned that code.
> `tests/conformance/lang/a-foreach-subject-that-is-not-iterable-is-refused-where-it-is-written.nvst`
> sweeps six shapes in source order with the one deferral, a `mixed` subject, deliberately last.
> `emit_binop`'s and `emit_unop`'s residue rosters are corrected where they called `Ty::Void` a
> representation no source expression has: a `void` call is one, and what keeps it out of both
> functions is that it is refused wherever it is written — as an operand (`E0718`), as a condition
> (`E0719`) and as an operand of `.` (`E0707`). The tagged subscript's four rows gain the
> byte-for-byte case they were owed alongside:
> `tests/conformance/lang/a-subscript-through-a-mixed-base-is-decided-by-its-tag.nvst` pins the
> element off a list and off a map, the absent key, the two non-array bases, all of it again under
> `??`, and the agreement between a declared `array<int>` base and the same value read through a
> `mixed`. **ADR 0007 § 2's grid is asserted at its boundaries now**, which is the half neither of
> its two standing cases could reach: `conversions-that-succeed.nvst` runs the middle of each row
> and `the-conversion-table-is-closed.nvst` refuses the pairs with no row at all, so
> `tests/conformance/lang/every-remaining-conversion-row-runs-or-throws.nvst` names the last value
> each `int`/`uint`/`float` row accepts beside the first it refuses — the `0 … i64::MAX` overlap in
> both directions, a `float` past `i64::MAX` and past 2^64, a `uint` one value past 2^53, and the
> digit run above `i64::MAX` that a `string` converts into a `uint` where the same run refuses into
> an `int`. The operand arrives as a parameter throughout, because an operand whose own type already
> names the value is refused where it is written (ADR 0047 § 6) and a bound this table is about has
> to arrive at run time to be a throw at all. **The three tag-decided families gain the case that
> asserts they agree with the typed spelling of the same question**, which is the one thing none of
> their own cases can assert alone:
> `tests/conformance/lang/a-mixed-value-answers-arithmetic-truth-and-a-subscript.nvst` sweeps
> thirteen values through an operator, a condition and a subscript in source order and then counts
> sixteen agreements against the declared-type twin of each, so a family that grows a second answer
> fails on the count rather than looking right on its own line. **A `catch` binding has no methods
> at all, and PHP's accessors are refused where they are written now rather than panicking below the
> checker.** Spec § 10 gives the exception tree *properties* — `message`, `previous`, `backtrace`
> and `location` — and `nvs_types::error_lib` seeds exactly those plus the synthesized constructor;
> what outlived that seeding was an **exemption** in `infer_method_call`, which skipped the
> unknown-member refusal for every reserved global class, so `$e->getMessage()` reached `nvs-ir`
> with no resolved target and panicked there while `$e->nope` — the property half, which never had
> the exemption — refused cleanly. It is the same `E0405` now, one mistake and one code whichever
> spelling reached it, and the only thing it takes of its own is a help, because this is the one
> unknown method a *ported* program writes on purpose:
> `nvs_types::expr::calls::report_exception_accessor` maps each PHP accessor to the property that
> answers the same question — `getMessage` → `message`, `getPrevious` → `previous`,
> `getTrace`/`getTraceAsString` → `backtrace`, and `getFile`/`getLine` → `location`, a throw site
> being one string rather than two — while `getCode`, the accessor with no counterpart at all since
> ADR 0002 propagates a class rather than a number, names the whole roster instead, read from
> `nvs_hir::errors::PROPERTIES` rather than copied so the help and the seeding cannot disagree.
> `tests/conformance/error/a-catch-binding-has-properties-rather-than-phps-accessors.nvst` pins all
> six spellings in one compile, the subclass binding among them, so a member the tree grows shifts a
> line rather than answering plausibly. **An inline-HTML run's *placement* is pinned now**, which is
> the half its standing case could not reach: `inline-html-is-written-verbatim.nvst` owns *what* a
> run writes, and `tests/conformance/lang/inline-html-at-file-scope-is-echoed-in-place.nvst` owns
> *where* — a run reaches `Lowering::lower_inline_html` from the ordinary statement dispatch, so it
> occupies a statement's place and nothing more: it prints between the statements written around it,
> once per iteration of a loop body, on the taken arm of an `if` and not on the other, in a method
> body when the method is called, and not at all in a `switch` case the subject does not select. Its
> last section is the agreement the shape asks for rather than a sixth line read off the output —
> five runs captured through `Core\Out::capture` and compared with an `echo` of the same literal,
> counted, so a run that grew a rendering of its own fails the count while still looking plausible
> on its own line. **A `require`d file's own top-level statements run now**, which is `nvs-ir`'s
> known gap 22 at its statement half and the last of the two file-scope shapes `holes.py --cases`
> named. Three things had to meet, and only the first was hard to place: `nvs_hir::resolve_program`
> is the one pass that joins a written path to a base directory, canonicalizes it and decides
> whether the target was already loaded, so it is the only place the `span -> SourceId` edge exists
> — it hands one out per file now (`Loaded::requires`), keeping a `by_path` map beside its `done`
> set so that a `require` naming a file some *other* file already pulled in records the same id
> rather than none. That edge rides to `nvs-ir` in `ExprTypeTable`, beside the property defaults and
> the codecs and for exactly their reason: `nvs-ir` is handed that table and not `nvs-hir`'s output,
> and a `require` is writable inside any body, so a second argument to `lower_program` would have
> changed every frame-lowering signature below it as well. `nvs_types::ProgramFile` gained **no** id
> field — `src.id()` has always been the file's own id, and a second copy is a value two dozen
> construction sites would have to supply and could supply wrongly. Then `lower_program` gives
> **every** file a script frame rather than only `files[0]` — the entry keeps the name its caller
> handed it, the rest take `file_script_label`, computed from the `SourceId` at both ends rather
> than recorded — and the `require` site emits one `InstKind::Call` to the frame it named, releasing
> the `Ty::Tagged` the frame returns exactly as a discarded object literal releases its own. It is
> called **every time** the statement is reached, PHP's answer for `require` as against
> `require_once`; the walk loading each file once is a compile-time fact, so a `require` in a loop
> body runs its target once per iteration and a nested `require` runs at its own site. **The frame
> is the file's, not the caller's**, and that is the decision this took: ADR 0021's "sharing the
> calling frame completely" was already false for variables in the tree, because `nvs_types::locals`
> checks each file's top-level body on its own, so the safe reading — declarations cross a `require`
> and variables do not — is folded into ADR 0021 § *Decision* rather than left for the body to
> disagree with. It is a divergence from PHP, where an included file does see the includer's locals,
> and the alternative would want one flow-sensitive definite-assignment analysis spanning a graph
> whose shape a `require` inside an `if` decides at run time.
> `tests/conformance/lang/a-required-file-runs-its-own-top-level-statements.nvst` pins the placement
> against the statements around it, the declaration half that never depended on a frame, two
> iterations of a loop each running a nested pair innermost-first, the untaken `if` arm that runs
> nothing, and the required file's own `$name` beside the entry's. **Gap 22's other half is closed
> with it, and `nvs-ir`'s known gap 22 is deleted outright.** ADR 0021 § 3's value form (`$c =
> require 'config.nvs';`) lowers to the very call the statement form already emitted, with its
> result kept instead of released — `lower_expr`'s arm and `lower_expr_stmt`'s are one call and one
> story, the second owning the reasoning about the frame and about running every time the site is
> reached. The frame's return type is `Ty::Tagged` by construction, so the value needs no
> conversion, and it is a fresh producer, so the consumer owns it. `E0704`
> (`E_REQUIRE_VALUE_UNLOWERED`) is retired and is never reused; `nvs_types::expr::infer`'s arm now
> checks the path and answers `mixed`, ADR 0007 § 2's one unchecked position being the whole of §
> 3's reasoning, so a typed binding takes the same `as` any other `mixed` boundary needs. The half
> that had no lowering at all was the *other* end: a file that never `return`s. § 3 names its value
> — `1`, PHP's own answer — so `lower_script`'s fall-through seal hands back a tagged `1` rather
> than the `Terminator::Return(None)` `lower_method` uses, a `Ty::Tagged` frame owing its caller a
> value on every exit. A file-scope `return expr;` needed nothing: it reaches `lower_stmt`'s
> ordinary arm through `lower_script_stmts`, and `self.ret_ty` is that frame's `Ty::Tagged`, so the
> item's lowering half was landed by the frame that carried it. What is *not* closed and is now
> backlogged rather than left implied: a `require` whose path is not a string literal runs nothing
> at all, silently, in both forms — `nvs_hir::requires`' own known gap treats it as dynamic, and the
> value form takes the same `1` for the same reason.
> `tests/conformance/lang/a-required-file-hands-a-value-back.nvst` pins the returned value, the `1`
> of a file that returns nothing, the target's `$part` beside the entry's own, the same site reached
> twice returning twice through a static the declaration owns, and the same again inside a loop
> body; valgrind-clean over a fixture that requires a freshly built `string` two hundred times.
> `an-uncompiled-construct-is-refused-where-it-is-written.nvst` loses its `E0704` half and now pins
> `E0703` alone, with both `require` forms beside it as the lines no diagnostic names. **ADR 0014's
> `PropertyObserver` exists now, and it was never a `.nvst` this tree already knew how to run.** The
> named case Stage 8 owed named a "reserved interface" `nvs_hir::interfaces` deliberately did not
> carry — `implements PropertyObserver` was `E0303` — so the item was the whole of §§ 2, 3 and 4
> rather than a case over landed work. The interface joins `Comparable` and `Stringable` on the
> `RESERVED` roster (taking no type parameters, for their reason: it is a contract an ordinary class
> implements, not a `Core` domain class), and `nvs_types::iter_lib` seeds its two members —
> `onPropertyGet(string, mixed): void` and its `onPropertySet` twin, `mixed` because one observer
> sees properties of every type and `void` because § 2 gives it nothing to return. Both being
> bodiless is what makes `nvs_types::conformance` demand the bodies, and what makes the call below
> dispatch on the receiver's runtime class. § 3's pipeline is a **second step** at the access, never
> a fallback: the checker records `ExprInfo::ObserverCalls` on every `ExprInfo::Property` and
> `ExprInfo::HookedProperty` whose receiver's class implements the interface, and
> `nvs_ir::lower::expr::Lowering::emit_observer_call` — the one home for the emission — retains the
> receiver and the settled value, tags the value to `mixed`, and emits one `InstKind::CallVirtual`
> carrying ADR 0002's error edge, so a throwing observer fails the access it was reporting. The
> **write** half reads the backing slot back where the property declares a `set` hook, because § 3
> says the observer is told the value the hook *committed* and not the caller's argument; with no
> hook the store is the commit and `v` is already it. § 4's zero cost falls out of the recording
> being compile-time: a class implementing nothing records `None` and lowers to the field load it
> always did, with no branch to measure. Three boundaries that "every property access" would
> otherwise decide by accident are now ADR 0014 § *Verification*'s to state — that section did not
> exist at all, which M4's acceptance has named since it was written. An access **inside the
> property's own hooks** carries no observer, being § 3's first step for the access already running
> the second, and observing it would report one write twice; a **`static` property** reaches
> nothing, ADR 0008 giving class storage no receiver to dispatch from; and an observer touching its
> own class's property **recurses**, exactly as any self-calling method does, because a re-entry
> guard is a second rule about which write is real and § 3's word is "unconditionally".
> Valgrind-clean over a fixture that writes and reads a freshly built `string` property through the
> observer two hundred times.
> `tests/conformance/class/a-property-observer-sees-every-write-its-class-makes.nvst` pins the
> pipeline's order against a `set` hook that echoes what it commits, the committed `42` against the
> caller's `21`, a constructor's writes, an inherited observer over a subclass's own property, a
> throwing one caught as an ordinary `Throwable`, not one line from a class that implements nothing,
> and the count of observed writes over the whole file, so a property shape that stops reaching the
> pipeline fails on the count rather than on a line. **ADR 0043 § 4's `implements I by $field;`
> delegation runs**, which is item 31 and the one hole where the front end accepted a program the
> object model then had nothing to dispatch to: `Outer::greet` resolved to the *interface*'s
> bodiless declaration, so `InstKind::CallVirtual` carried `fallback: None` and the receiver's own
> method table had no row for the name at all — `nvs_runtime::nvs_abstract_method`, a `FATAL` naming
> a compiler bug for a program that is correct. § 4 says the compiler synthesizes a one-line forward
> per required member, and the decision this took is that a forward is a real **method** rather than
> a rewrite at the call site: a receiver typed as the *interface* dispatches on the runtime class,
> so a rewrite would forward `$post->touch()` and leave `$timestamped->touch()` reaching exactly the
> nothing it reached before. The resolution is `nvs_types::conformance::resolve_delegations` — which
> members the interface requires, and which of them the class already answers with a body, are
> questions about the signature table and the class graph, neither of which `nvs-ir` holds — and it
> rides across as `nvs_types::Delegation`, one record per forward, for the reason every other entry
> in that table does. `nvs_ir::lower::call::delegation_forward` builds each one as a whole
> `Function`: read the field, retain it (a field read borrows and `CallVirtual` transfers its
> receiver), then the call, then the return, with `$this` released on both exits and every argument
> transferred straight on. The call is late-bound with **no fallback** deliberately — the field's
> declared type is the interface, whose member has no body to name — so the target is whatever the
> field's runtime class answers, which is the whole of what delegating to it means. `lower_program`
> adds the matching row to the class's method table, and skips a name the flattened table already
> answers, which is § 4's "a class may still write its own method with the same name as a delegated
> one" extended to an inherited body and an ADR 0043 § 2 interface default on the same terms. Three
> member shapes get no forward and each is a hole rather than a rule, named in
> `resolve_delegations`' own doc comment: a `static` member has no receiver to forward through, and
> a variadic or `inout` parameter list is packed and written back at the *call site*, so passing it
> straight on would pack it twice. The whole-class conformance exemption stays, and its reason is
> now stated where it is made: § 4's `E_DELEGATE_TYPE_MISMATCH` — "does `$field`'s type satisfy this
> interface" — is not built, and without it a member no forward covers cannot be told from one whose
> field cannot answer it.
> `tests/conformance/class/a-delegated-interface-forwards-to-the-object-it-names.nvst` pins the
> forward through the class and through the interface alike, two interfaces delegating to two
> different fields of one class, an own method beating the forward, a throwing delegate caught where
> the outer call is written, and the agreement between the two spellings of one call; valgrind-clean
> over a fixture that forwards a freshly built `string` and abandons one on the error edge two
> hundred times each. **ADR 0043 § 4's first bullet is checked now, and the conformance check is per
> member rather than per class.** `nvs_types::conformance::check_delegate_field` asks the question §
> 4 names — is `$field` a declared property of the class, of a **non-nullable** class or interface
> type that satisfies the delegated interface — and refuses it where the clause is written
> (`E0720`), each half being a failure the synthesized forward has no answer for: a name that is no
> property has no slot to read, a `null` in the slot names no class to dispatch on, and a type that
> does not reach the interface names no member for the late-bound call to land on. The judgement is
> `nvs_hir::implements_interface`'s, which is reflexive, so a field declared *as* the interface
> satisfies it in zero steps — § 4's own worked example's spelling. That code is what buys the
> per-member exemption: `resolve_delegations` hands back the set of members it actually supplies,
> and `check_class_conformance` runs its ordinary `E0449` walk over everything else, so a class
> delegating one interface is still judged on every other one it claims — the whole-class exemption
> used to swallow that outright. A clause whose field is refused marks that interface's whole
> obligation set covered, so the author reads one diagnostic about the field rather than one per
> member. The three member shapes the forward cannot express are refused too (`E0721`) rather than
> left to reach `nvs_runtime::nvs_abstract_method`, a `FATAL` naming a compiler bug for a program
> the front end had accepted: a `static` member has no receiver to read the field off and a variadic
> or `inout` parameter list is packed and written back at the *call site*, so a forward would do it
> twice. Each is a limit of this compiler rather than a rule about delegation,
> `resolve_delegations`' own doc comment is its home, and the way out is § 4's existing one — write
> the member on the class by hand, which beats a forward anyway.
> `tests/conformance/class/a-delegate-field-must-be-able-to-answer-the-interface.nvst` pins all four
> `E0720` shapes plus the per-member `E0449` in one compile, and
> `a-delegated-member-the-forward-cannot-express-is-refused.nvst` the three `E0721` ones beside the
> own-method way out. ADR 0043 § 4 gains both bullets rather than being left to disagree with the
> tree, and `ImplementsClause`'s own doc comment is corrected where it named `nvs-hir` as the owner
> of a check that needs the signature table. **A promoted constructor parameter is a property now**,
> which is § 4's own parenthetical and the last thing standing between `check_delegate_field` and
> the shape the ADR writes. It was not one anywhere: the parser recorded the modifiers and nothing
> below read them, so `constructor(public int $n)` declared a parameter and `$b->n` was `E0405` on a
> class that plainly had one. Four tables answer "is this a property" and all four answer it now —
> `nvs_types::signatures` records the declared type and the visibility keyword, so
> `resolve_property` finds it and ADR 0094's access check reads its level; `nvs_types::layout` gives
> it a slot in member order; `nvs_hir::members` admits `$this->n` inside the class; and
> `nvs_ir::lower::promoted_stores` emits the store the author did not write, into the entry block
> ahead of the body, so a constructor that reads `$this->n` reads what it was passed.
> `nvs_syntax::ast::Param::is_promoted` is the one home of *which* parameters those are, because
> four crates having their own spelling is how they would stop agreeing: a **visibility** keyword
> promotes and nothing else does, exactly as in PHP, `readonly` alone declaring nothing to be read.
> Three maps beside `properties` are deliberately left alone and each for its own reason, stated at
> `record_promoted_properties`: `required_properties` is ADR 0022 § 2's obligation and a promoted
> parameter discharges it **by construction**, the store being emitted from the binding rather than
> written in a body — `crate::ctor_init`'s own known gap said the opposite and is corrected;
> `property_defaults` arms a slot before the constructor runs, while a promoted parameter's `= expr`
> is the *parameter's* default, applied at the call site that omitted it and stored by the same
> assignment every other argument is; and `hooked_properties` has no spelling to record, a parameter
> list having nowhere to write a hook body. Of `lower_reassignment`'s property policy the store
> keeps the **retain** — a compiled method owns its parameters and releases each at every exit, so
> the slot needs a reference of its own — and drops the other two: no coercion, one declaration
> being one type, and no release of the old value, this being the first store the slot ever sees.
> Valgrind-clean over a fixture that constructs with a borrowed and a freshly built `string` two
> hundred times. `tests/conformance/class/a-promoted-constructor-parameter-is-a-property.nvst` pins
> the read from outside and through `$this`, the `private` keyword refused at an outside read, a
> default omitted and supplied, a subclass reaching what its parent promoted through
> `parent::constructor`, and § 4's delegation naming a promoted field, with the count of agreements
> over the file; `two-interfaces-delegating-to-one-field-both-forward-to-it.nvst` is § 4 bullet 3's
> other half, `implements A by $x, B by $x` synthesizing both member sets onto the one field with an
> own method still beating either forward. **ADR 0022 § 3's never-written storage state exists
> now**, which closes § 4 bullet 2 with it and is the last thing either ADR was waiting on. The
> state is `nvs_runtime::Tag::Unset`, one more discriminant on the value representation exactly as §
> 3's own paragraph priced it — **zero additional bytes per property** — and a slot is stamped with
> it by the very pass that arms a declared `= expr` default, `nvs_types::FieldDefault::Unset` riding
> in the same `(slot, recipe)` list rather than in a second one beside it. Until `Core\Reflect`
> exists (M6) the one declaration that reaches the state is a **`lateinit`** property (ADR 0038), §
> 2 discharging every other non-nullable one at its constructor, and ADR 0038 § 3's intraprocedural
> check already refuses the reads it can see — so what the runtime answers is the read from outside
> the class. **The two readers ask the question differently and answer it the same way, which is the
> decision in this**: a reader holding the whole slot goes by the tag
> (`nvs_runtime::nvs_object_slot_get`), while the compiled read goes by the **payload**, null in
> this state and in no other because ADR 0038 § 1 restricts `lateinit` to a non-nullable class or
> interface type — one compare on the pointer the `FieldGet` had already loaded, rather than a
> second load of the tag byte. `InstKind::IsNull` gained a `Ty::Object` row for it, one
> representation down from the tag test it already was, and
> `nvs_ir::lower::expr::Lowering::emit_never_written_guard` is that argument's one home. The throw
> is a `LogicError` — spec § 10's "a bug in the program", not ADR 0020's ladder, § 3 being explicit
> that this is catchable — and the erased read raises the same wording so that one failure reads one
> way. ADR 0043 § 4 bullet 2 is the same guard on the **forward's** own read, and it is not the null
> receiver it looks like: `InstKind::ClassDescOf` over an unwritten slot reads the *forwarding*
> class back, so the forward calls itself until the stack is gone, and `delegation_forward`'s
> guarded edge is also the one place that function's ownership is not the callee's — no call runs,
> so every argument it was transferred is released where the callee would have released it. Two
> `.nvst` cases pin it, `a-property-that-was-never-written-is-read-as-a-throw.nvst` over the outside
> read, the second read, the untouched sibling property, the write-then-read, a temporary receiver
> and a subclass instance, and `a-delegate-reached-before-its-field-is-written-throws.nvst` over the
> forward through the class and through the interface alike, a forwarded member with arguments, and
> the agreement between the forward's read and the direct one; valgrind-clean over a fixture that
> abandons a freshly built `string` argument on the refused edge and a freshly built receiver on the
> read's, two hundred times each. One number moved to make room:
> `FN_PARAM_TAG_ANY`/`CLOSURE_PARAM_TAG_ANY`, the closure parameter nibble that is deliberately
> *not* a tag, was twelve — the first number past the roster — and is fifteen, the top of the
> nibble, so that the next discriminant the roster grows cannot collide with it either;
> `nvs-codegen`'s `the_any_nibble_denotes_no_tag_at_all` is what caught that and is the guard either
> way. **A visibility keyword promotes only in the `constructor` now**, which was ADR 0043 § 4's own
> backlog line and is the last thing that ADR's promotion paragraph owed.
> `nvs_syntax::ast::Param::is_promoted` is the one home of *which* parameters promote and
> deliberately does not ask which method encloses it — that is `nvs_types::signatures`' question,
> and it was only ever asked in one direction: `record_promoted_properties` runs for a
> `constructor`, so `public`/`protected`/`private` written on any other method's parameter declared
> nothing, took no slot and was silently ignored, where PHP refuses it. It is `E0722` where it is
> written now, from `reject_promotion_outside_constructor`, the same predicate read the other way.
> Widening promotion to every method was never the alternative: a property is a slot on an instance,
> armed once where the instance is made, and a method may be called any number of times or none, so
> there is no moment for the store to be emitted at. The refusal is per parameter and in source
> order, so a position that stops refusing shifts a line rather than quietly declaring nothing, and
> the primary span is the whole parameter rather than its name, the keyword being what the
> diagnostic is about.
> `tests/conformance/class/a-visibility-keyword-promotes-only-in-the-constructor.nvst` pins all five
> — three keywords across an interface declaration, an instance method and a `static` one, and a
> keyword beside an `inout` and beside a default — with the constructor's own promoted parameter in
> the same file saying nothing, which is the half that asserts the check did not widen. ADR 0043 § 4
> gains the bullet rather than being left to disagree with the tree. **A class with no explicit
> `constructor` is held to a zero-argument arity check on `new` now**, which is item 39's second gap
> and was worse than a missing diagnostic. An arity check is a count against a *signature*
> (`nvs_types::expr::args::check_positional_arity`), so a class with no constructor had none to be
> counted against: every argument written at `new Plain(1, 2)` was inferred, checked against nothing
> and then dropped, and `nvs-ir` lowers that `new` with `ctor: None`, so the arguments were not
> evaluated for their effects either — the program compiled, ran, and constructed exactly what `new
> Plain()` constructs. `nvs_types::expr::calls::reject_arguments_to_implicit_constructor` refuses it
> where it is written, taking `E0402` rather than a code of its own because it is the same mistake
> the count already names, with the class added to the message so the author is told *why* zero. Two
> targets are exempt and neither is a class the author declared: a `Core`-owned class is constructed
> by a native symbol rather than by a `constructor` member
> (`nvs_stdlib::registry::constructor_symbol`, the neighbouring check refusing the ones that have
> none), and a class this unit has no signature for at all has already been reported as unknown, so
> a second diagnostic would name one mistake twice. An **inherited** constructor is a signature and
> is untouched: `new Derived()` on a `Derived extends Base` that declares none of its own is still
> counted against `Base::constructor`.
> `tests/conformance/class/a-class-with-no-constructor-takes-no-arguments.nvst` pins one argument
> and two against the implicit constructor, the inherited count beside them, and the accepted `new
> Plain()` and `new Derived(1)` written first so that a position that stops being accepted fails
> there rather than as a missing refusal. `nvs-types`' own known-gaps list loses the entry rather
> than keeping a gap the tree closed. **A `foreach` key binding over an `array<T>` is a `string` or
> it is refused**, which is item 39's last gap and the whole of that item now closed. ADR 0007 § 5
> gives the container **one** stored key type — "every key is a `string`. There is no integer key" —
> so the type parameter is the value's and a key binding declaring anything else is not an
> unprovable narrowing but always wrong: no array can produce an `int` key to fill it.
> `nvs_types::expr::iteration::check_foreach_key` refuses it where it is written (`E0723`), `mixed`
> among the refused, there being no second key type for it to be the union of; the normalisation
> that reads `$a[8]` as `$a["8"]` happens at the *subscript*, and there is none on the way back out
> of a loop. A binding that declared no type at all is left alone, `nvs_syntax` already reporting
> that omission and the `mixed` it lowers to being an error-recovery placeholder rather than
> something the author wrote. Unrefused it reached no diagnostic *and* no answer: `nvs-ir` lowers a
> key binding only at `string` and asserted on everything else, so a mistake in the program surfaced
> as a panic naming a compiler gap — that assertion is an internal-consistency check now and says
> so. The cursor keeps its own `E0444`, ADR 0053 § 1 giving `Iterator<T>` no key at all, so one rule
> still draws one code. That function's doc comment had argued the opposite — that ADR 0007 § 5
> fixes "the two legal key types, `int` and `string`", which that section deletes outright — and is
> corrected rather than reconciled, as is the `nvs-types` unit test that pinned the key binding as
> deliberately unchecked. ADR 0007 § 5 gains the paragraph, and `nvs-types`' known-gaps list loses
> its last entry. `tests/conformance/lang/a-foreach-key-binding-is-a-string-and-nothing-else.nvst`
> pins all five refusals in one compile — `int`, `uint`, `mixed`, a union, and a nested array's
> inner key — with the accepted `string` spelling written first, so a position that stops being
> accepted fails there rather than as a missing refusal. **Every instruction that returns a status
> carries a landing block now**, which is item 36 and `nvs-codegen`'s known gap 3 deleted outright.
> The rule used to be phrased over what a *program* can recover from: a call, a `new` and ADR 0007 §
> 4's checked integer rows took ADR 0002's error edge, while a conversion helper, the truthy table,
> `Helper::Identical` and the numeric comparison family, a `??` subscript, `as ?T`'s non-throwing
> rows and `lower_decimal_binary`'s comparisons were emitted plainly — each of them returns a status
> whose only non-`OK` value is a `FATAL` or an `EXITED`, and `nvs_ir::ir::Inst::on_error` stated the
> consequence rather than hiding it: such an outcome released nothing at all. That is not a
> cold-path saving but a leak of the whole frame, `O(requests served)` on a shape an attacker drives
> (unbounded recursion) and on one an ordinary CLI program writes (`exit()`), so the rule is the
> status now and `Inst::on_error` is its one home. The block sits on the failing edge, so nothing on
> the request path pays for it. **The other half was `Terminator::Catch`**, and it is where the leak
> actually reached a fixture: a landing block inside a `try` releases no local on its way to the
> handler — the handler still names them — so an uncatchable status returned straight out of the
> frame past every one. That terminator carries an `onward` block now, a second landing block of the
> same frame performing exactly the sweep a `Propagate` at an unprotected site does, and
> `nvs-codegen` branches to it rather than building a bare fail block of its own. Both of that
> backend's `None` arms are internal-consistency checks now — `emit_status_check`'s and
> `raise_arithmetic_error`'s — which is what enumerated the producers a grep could not:
> `lower_decimal_binary`'s comparison rows, a `foreach` cursor's own `slot + 1`, whose `int` row
> `nvs-codegen` emits checked whatever a slot index can actually reach, and the two instructions ADR
> 0053 § 4's synthesized frames build by hand — the factory's `New` of the state object and
> `gen#unwind`'s call into `advance()`, each lowered over an empty `Env` because that is what those
> frames hold at the point they emit it. A closed `nvs-codegen` gap is deleted and its number
> **retired**, never reused and never handed to a survivor, so the list has holes on purpose and
> every `nvs-codegen gap N` written elsewhere keeps meaning what it meant.
> `every_status_returning_instruction_carries_a_landing_block` asserts the rule as a sweep with a
> count rather than off a named line — the shapes that used to be exempt were exactly the ones no
> `catch` can act on, so a case naming one would go green while the rest slipped back — and
> `an_uncatchable_status_leaves_a_try_through_a_block_that_releases_the_locals` pins the `onward`
> exit's releases and its `Propagate`. Valgrind-clean over a fixture that abandons two freshly built
> strings by calling `exit()` from inside a `try`, and over five of `examples/`. **ADR 0092's record
> model exists, and `Core\Debug::dump` is its first producer** — item 33's larger half, landed as a
> new leaf crate rather than as a member module, because § 1 of that ADR puts the model and all
> three renderings in one crate that both the runtime and the front end can depend on and the
> handoff's file set predated reading it. `nvs-render` holds § 1's closed node roster, § 2's
> `Log\Level` and its fixed syslog mapping, § 5's four transformations and § 3's **plaintext**
> rendering; `nvs_stdlib::debug` holds the walk that turns a runtime value into a record, plus
> `dump` and `render` themselves. Three of the four transformations are structural rather than a
> convention a producer is asked to follow, which is what makes § 5's *"decided once, in the model"*
> true of the type and not of a review: control bytes and the bidi rule are `nvs_render::Rendered`'s
> **one** constructor, so there is no spelling that puts an un-substituted byte into the model at
> all; a cut is an `Elision` **node**, so the JSON and HTML renderings will print the same cut of
> the same value rather than each choosing its own, which is exactly what PHP and Python get wrong
> by truncating per formatter; and a cycle is a `Cycle` node naming an identity rather than a
> `*RECURSION*` string. ADR 0087's predicate is **called** rather than restated, which is why
> `nvs-render` has one dependency and it points the wrong way for now — `nvs-syntax`, whose `bidi`
> module moves *down* into `nvs-render` the moment `nvs-runtime` or `nvs-diagnostics` becomes a
> dependent, a move and not a copy, stated in that crate's own module doc and folded into ADR 0092 §
> 1 rather than left to be rediscovered. § 4's destination took a second sink on `nvs_runtime::Ctx`:
> a dump goes to **stderr** and never to stdout, so `prog | jq` keeps working, and
> `write_diagnostic` is deliberately **not** routed through `Core\Out::capture`'s stack — capturing
> a dump would swallow the very output it was written to make visible. That is a second `OutputSink`
> field rather than a fourth variant, because the two channels differ in where they go and not in
> what is written to them, and `OutputSink::Stderr` is the new variant both share. `render` is one
> member and not a second mechanism: it answers the sink's carrier (`Core\Cli\Text` today) by ADR
> 0088 § 5's rule verbatim, so `echo Core\Debug::render($x)` is singly escaped, and it is what makes
> the whole plaintext view pinnable on **stdout** — `--EXPECT-ERROR--` is also what tells the
> `.nvst` runner a case is expected to *fail*, so a successful run has no way to read standard
> error, and the two halves of ADR 0092's own M4 bullet are split accordingly:
> `tests/conformance/core/a-dump-renders-one-record-through-one-plaintext-view.nvst` pins every node
> kind a `mixed` can reach — the eight scalars, both array shapes, an object with its class, its
> identity and its declared properties, two closure arities, a self-referential `lateinit` cycle and
> the `CR` substitution — and asserts that two `dump` calls put **nothing** on standard output,
> while `a_dump_writes_to_the_diagnostic_channel_and_not_to_the_output` asserts the other side of
> the same statement in Rust. **Item 33 is closed — both halves of ADR 0092 § 5's redaction row, and
> the caps case with them.** ADR 0092 § 5 states redaction as two halves of one rule about one
> record — a property whose *declared* type carries `secret` becomes a `Redacted` node, and a
> `secret` value handed straight to the dump is refused by `nvs check` — and neither can be decided
> from a value's tag, `secret` being a qualifier on a declared type. The second half needs nothing
> below the checker for exactly that reason, and is where the qualifier is last visible at all:
> `Core\Debug::dump` declares `mixed ...$values` and `render` a `mixed $value`, both of which a
> `secret string` satisfies, so `nvs_types::expr::quals::reject_secret_debug_argument` refuses the
> argument where it is written (`E0724`), per argument and in written order, so a `dump($a, $secret,
> $b)` names the one it is about. **`render` is refused on the same terms as `dump`** rather than as
> a widening of the item: § 5's closing paragraph makes the renderings non-bypassable — there is no
> `dumpRaw` and no rendering selected by an argument — so the member answering the same record's
> text as a `Core\Cli\Text` carrier is the same disclosure one `echo` later, which § 4's terminal
> bullet refuses with no carrier bypass in any case; refusing only `dump` would have left `echo
> Core\Debug::render($secret)` as the way round both bullets. It takes its own code rather than
> `E_SECRET_THROWABLE_MESSAGE`'s because the help has to distinguish the value the author handed
> over from the value a record redacts for them, and because a dump goes to the diagnostic channel a
> person reads rather than into a message a program carries. Two shapes it does not reach and
> neither is a gap in this rule: a `...$xs` spread hands over a subject whose *element* type carries
> the qualifier, ADR 0033's unmodelled container axis, and a `secret` value behind a property or an
> array element reaches the walk instead — which is the other half.
> `tests/conformance/reject/a-secret-value-cannot-be-dumped.nvst` pins both members, both bases and
> the composed `secret tainted string`, with the accepted `$plain` and `tainted` spellings written
> first so a position that stops being accepted fails there rather than as a missing refusal;
> `tainted` is deliberately accepted, § 5's closing paragraph making the record's framing what keeps
> a dumped tainted value safe. **The property half is a bit per field slot, carried down the
> vertical rather than recomputed at any point on it**, because nothing below the checker could
> recompute it: a `secret string` is byte-identical to a `string` in every representation under the
> qualifier — same tag, same slot, same allocation — so a walk holding only a value has no question
> to ask. `nvs_types::expr::type_is_secret` decides it at the one end where the qualifier still
> exists, `nvs_ir::lower::field_slots` joins it onto the slot order in the *same* walk that already
> answers each slot's representation (two walks over one table could disagree about which
> declaration won a slot, and a redaction that named the wrong slot discloses the value it was meant
> to hide), `nvs_ir::ir::Class::secret_fields` carries it,
> `nvs_runtime::ClassTable::set_secret_fields` puts it on the descriptor beside `field_tags`, and
> `nvs_stdlib::debug`'s walk reads it off the **instance** — which is what makes a `secret` property
> redact through a `mixed` exactly as through its own type, and a *nested* object's `secret`
> property redact by its own class's declaration one level down. The value is never walked at all
> rather than walked and discarded, so a redacted slot contributes no elision node saying how long
> it was. `ClassDesc::field_is_secret` is deliberately a `bool` rather than `field_tag`'s `Option`:
> that one distinguishes "unknown" from "admits several", and there is no third answer to whether a
> type carries a qualifier — `false` for a slot nothing told, which is safe only because a
> *declared* `secret` property always reaches the join. An ADR 0036 shape literal's field is the one
> declaration-shaped thing that carries no bit, its type being *inferred* from its initializer, and
> it joins the `array<T>` element as ADR 0033's unmodelled container axis in `nvs_stdlib::debug`'s
> own known gap 1 rather than as a hole in this rule.
> `tests/conformance/core/a-secret-typed-property-is-redacted-wherever-it-is-dumped.nvst` pins the
> slot beside its plain neighbours, the same instance through `mixed`, `secret` alone against
> `tainted` alone and the two composed, an inherited slot and the subclass's own, a nested object,
> an ADR 0043 § 4 promoted parameter, and the agreement between the typed and the erased spelling
> counted rather than read off a line. The three caps gain the case they were owed alongside —
> `a-dump-cuts-at-its-caps-and-not-one-entry-early.nvst` names the last value each carries whole
> beside the first it cuts (1024/1025 bytes of one scalar, 100/101 entries of one container, 8/9
> levels of container), asserts each cut is an `Elision` node in the model rather than a truncation
> a rendering chose, and pins that a redaction is not an elision and consumes none. Two smaller gaps
> are stated at `nvs_stdlib::debug` rather than implied: an enum case dumps as its backing integer,
> because ADR 0010 § 5 spends no tag on hiding one and a `mixed` cannot tell; and the `Throwable`
> producer of § 6 is not here at all, its walk belonging to `nvs-runtime`'s fatal path and therefore
> waiting on the crate edge above. **ADR 0046's attach grammar is the ADR's own now, and the payload
> rule under it is closed** — item 32's first two slices, the ones below the retrieval § 4 still
> owes. The grammar had drifted into PHP's shape rather than this ADR's: `Attribute` carried a
> `Name` plus a `CallArgs`, so `#[Route(path: "/x")]` was an *argument list* that happened to be
> named and the bare `#[{...}]` form § 1 gives equal standing did not parse at all. It is one
> payload for both forms now — `Option<Name>` plus ADR 0036 § 2's own `ObjectLiteralField`s — and
> the parenthesized run is parsed by the very function that parses an object literal's fields
> (`Parser::parse_object_literal_fields`, factored out of `parse_object_literal_expr` rather than
> copied), so an attribute payload takes that literal's rules instead of a second set of its own: no
> positional value, no shorthand, no computed key, each refused where it is written by the
> diagnostic the literal already had. The named form written with no list at all (`#[Audit]`)
> attaches an empty literal, which is what keeps § 1's two forms one shape rather than three.
> `nvs_types::derive` reads the payload where it read arguments, and *loses* a check by it — ADR
> 0071 § 3's "a `#[Json\Field]` argument must be named" was the object literal's rule restated one
> crate up, and the parser makes it unspellable now. Then **§ 2's compile-time-constant-only rule is
> a check** rather than a sentence in an ADR: `nvs_types::attributes` walks every attach site a
> declaration owns — its own groups, each property's and that property's hooks', each const's, each
> method's and each of its parameters', and each enum case's — and refuses a field value that is not
> a literal, a class constant or an enum case (`E0725`), per field and in source order, so a payload
> with two computed values reads as two mistakes. The admitted list is **closed** and that is the
> decision in it: an `ExprKind` this pass does not name is refused, so a shape the grammar grows is
> refused until someone decides it belongs in a constant pool — which is why it is its own walk
> rather than a row on the pass that folds an enum case's value, a pass whose whole job is to be
> willing to fold. An interpolated string is the row worth naming: it reads a variable by
> definition, whatever it interpolates, so it is not the string-literal row one syntax along. ADR
> 0046 § 2 gains the enforcement site rather than being left to name a pass that does not do this.
> `tests/conformance/reject/an-attribute-payload-is-a-compile-time-constant.nvst` pins all five
> refusals in one compile — a variable, a call, a `new`, an interpolation and a parameter's own
> attribute — with both attach forms and eight accepted field shapes written first, so a position
> that stops being accepted fails there rather than as a missing refusal. **§ 1's other half is
> closed with it: a named attribute's `Name` is a shape-typed `type` alias and the payload is
> checked against it.** `nvs_types::attributes` resolves the name in the same walk § 2's rule uses —
> `nvs_hir::resolve_ref` and then the alias table, so a `use` import and a namespace place it
> exactly as they place any other name — and answers three ways, deliberately. A name **nothing
> declares** is the ordinary `E0303`, an attribute name being no second namespace and so owed no "no
> such attribute" of its own; a name that resolves to something that is **not a shape-typed alias**
> is `E0726`, the class spelling being what PHP would have instantiated and a `type Id = int;` the
> same mistake one step along; and a shape gets the check the form exists for — the attached literal
> against it through `nvs_types::expr::is_assignable`, ADR 0036 § 3's width subtyping verbatim, so
> an extra field is fine and a missing or mistyped one is the ordinary `E0401` rather than an
> attribute-shaped diagnostic. A payload § 2 has already refused is *not* then checked against the
> shape, so the author is told about the value they wrote before they are told what it failed to
> satisfy, and the literal is inferred over an empty scope because § 2 has just proved no variable
> is in it. The one exemption is ADR 0071 § 1's **compiler-recognized** attributes, whose closed
> `Core`-owned roster (`crate::derive::ATTRIBUTES`) names no shape at all and whose payloads are
> that pass's own option check — closed and `Core`-owned precisely so it stays an exemption rather
> than an escape hatch. Two standing cases moved with the rule rather than around it, which is what
> proves it reaches the sites they cover: the § 2 case declares the aliases its five attribute names
> always implied, and ADR 0061's autoload case now harvests a `type` alias where it harvested a
> class, the prefix lookup being the same either way and what a name is looked up *for* not being
> that ADR's question. `tests/conformance/reject/an-attribute-name-is-a-shape-typed-type-alias.nvst`
> pins all five refusals in one compile — the undeclared name, a class, an alias for a scalar, a
> mistyped field and a missing one — with both forms and the extra-field widening written first.
> **ADR 0033 § 4's fifth sink is landed, and item 32 owes only §§ 4-5's retrieval** — the call-site
> `<T>` of § 6 needed nothing, `parse_call_type_args` having landed with ADR 0107's neighbours. The
> sink exists *because* of ADR 0046 § 2 rather than in spite of it: a payload admits only
> compile-time constants and a class constant is one of them, so the storage class a `secret` value
> lives in is the only way one could reach a payload at all, every other spelling being refused
> already for being computed — which is what makes it one check over one expression kind rather than
> a walk of its own. It is the one sink whose qualifier cannot be read off an inferred type, and
> that is a gap one crate over rather than a choice: `signatures.rs`'s own known gap leaves a class
> constant's declared type unmodelled, so `Class::TOKEN` infers `mixed` at every expression site and
> `is_secret` over that answers `false` for a value that plainly is one. So the bit is read off the
> declaration's own annotation in `crate::consts`' walk — the last place the qualifier exists — and
> rides on `ConstEntry` beside ADR 0047 § 2's folded value, one `bool` rather than the type, because
> a bit is all the sink asks for and modelling the type is the gap above rather than this one. It is
> read **syntactically**, off `nvs_syntax::ast::TypeAtom`'s four secret rows, because that pass runs
> before the first annotation is interned — which is the whole reason it is a pass of its own — and
> a composite carries the qualifier exactly when one of its members does, which is the safe
> direction for a sink either way. `ConstTable` grew one public question (`is_secret`) beside `get`
> and one private ancestor walk under both, so the two cannot disagree about *which* declaration a
> name means, and an inherited constant is therefore the same refusal through the subclass. The
> refusal itself is `nvs_types::expr::quals::reject_secret_attribute_constant` (`E0727`), beside its
> four siblings, called from `crate::attributes`' payload walk at every value it reaches rather than
> only at a payload's top level — a `secret` constant nested inside an array or an object literal is
> folded into the same constant pool. There is deliberately **no `Core\Secret::reveal()` way out of
> this one**, that being a call and a payload admitting none, so the help names the fix that exists:
> keep the secret out of the metadata and let the attribute carry the *name* of where to read it
> from. ADR 0033 § 4 gains the bullet rather than being left to name a sink no pass makes.
> `tests/conformance/reject/a-secret-class-constant-cannot-reach-an-attribute-payload.nvst` pins all
> eight refusals in one compile — both attach forms, both bases and the two composed, a constant
> nested in an array and in an object literal, an inherited declaration, and the
> property/method/parameter attach sites — with the plain constant and a `tainted` one written
> first, `tainted` being accepted on purpose since ADR 0024 answers a question about a value's shape
> and no ADR names a payload as a taint sink. **ADR 0046 §§ 4-5's structural retrieval runs, and
> item 32 is closed but for one target spelling.** `Core\Attributes::get<T>` and `::all<T>` are
> registered like any other `Core` class (`nvs_stdlib::attributes`) and implemented by nothing at
> all: § 5 says a declaration's attached-attribute list is static and § 2 has already proved every
> payload value is a compile-time constant, so `nvs check` **replaces the call with its answer** — a
> compiled-in `null`, the matched literal itself, or the array of them — and the two registered
> symbols name a body that aborts precisely so a call that slipped past the fold is loud rather than
> plausible. The matching is § 4's *structural, not nominal* rule verbatim: an attached literal is
> an answer exactly when it satisfies `T` under `crate::expr::is_assignable`, ADR 0036 § 3's width
> subtyping and the same test a shape-typed binding goes through, whether it was attached bare or
> under a name and whatever that name was — so there is no second namespace of attribute-kind names
> for unrelated frameworks to collide in, and a bare `#[{...}]` is retrievable exactly like a named
> one. `nvs_types::retrieval` is that pass's one home. Three decisions in it are worth naming. The
> **table is built whole, ahead of the walk** (`build_attribute_table`), for `build_const_table`'s
> reason: a retrieval may be written above the declaration it asks about, in the same file or
> another, so a table filled as the walk descends would answer differently depending on source
> order. The target is **inspected syntactically**, exactly as ADR 0033 § 4's sinks inspect a
> literal argument — `Foo::bar(...)` here is a written name and never a closure value, which is what
> makes a compile-time answer possible at all — and § 4's two overlapping spellings are *joined*
> rather than ordered: `constructor` plus a member name is both "the property named that" and "the
> constructor parameter named that", which are the same declaration for an ADR 0043 § 4 promoted
> parameter, so both rosters are consulted. And the fold's **value is a constant**, `ConstArg`
> gaining a `Shape` and an `Array` variant that `nvs-ir`'s `emit_const_arg` materializes — the shape
> through the very `shape_class_label` a written literal of the same field set gets, because two
> classes for one shape would make the field offsets they agree on a coincidence. Four refusals land
> with it, each where it is written: two matches under `get` is `E0728` naming `::all<T>` as the fix
> (§ 5's "a genuine improvement over PHP/Java/C#", where that is a question a test run answers), a
> `T` that is not a shape is `E0729`, a target that names no declaration is `E0730`, and a matched
> payload holding a value with no constant form is `E0731` — a **user-declared** class constant,
> whose value `signatures.rs`'s own known gap leaves unmodeled, and an enum case, which reaches a
> program through `ExprInfo::EnumCase` rather than through a constant. Neither is refused where it
> is *attached*: § 2 admits both, and an attribute nobody retrieves costs nothing. A computed
> `$member` needs no refusal either — § 4's *Consequences* already fixes it as an empty result, so
> `get` folds to `null` and `all` to the empty array.
> `tests/conformance/core/an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst` pins the named
> and the bare form retrieved through one shape each, the width-subtyping widening, the compiled-in
> `null`, two matches under `all` in attach order, all four § 4 target spellings, the computed
> member name, and the agreement between a retrieved payload and a hand-written literal of the same
> shape counted rather than read off a line;
> `tests/conformance/reject/an-attribute-retrieval-is-refused-where-it-cannot-be-folded.nvst` pins
> all four codes in one compile with the accepted `all<T>` spelling written first. **Item 32 is
> closed, and the last spelling was a name-resolution question rather than the retrieval pass's.**
> ADR 0022 § 2 gives every class a constructor, definitely, so § 4's class target names one whose
> body writes none — and `nvs_hir::members`' `StaticCall` arm exempts exactly that reference from
> its undefined-member check. The guard is `CallArgs::FirstClassCallable` at the **call site**
> rather than a flag threaded into `check_member_ref`, which cannot see an argument list at all; a
> *written* `Foo::constructor()` on such a class resolves to no signature in `nvs_types` either, so
> it would reach `nvs-ir` with no target recorded and panic there, and `E0309` is what keeps that a
> diagnostic about the program. No second refusal is added beside it: a `nvs_types` code would draw
> two codes for one mistake, and a conditional exemption leaves the one that already fires exactly
> where it was. Both retrieval cases drop the `public function constructor() {}` they were written
> around, with their expected output otherwise unchanged, which is the check that the exemption
> reaches the shape the ADR writes;
> `tests/conformance/reject/a-synthesized-constructor-is-referenced-and-not-called.nvst` pins the
> boundary — the retrieval against a bodiless class accepted first, then the written call and the
> ported `parent::constructor()` refused in source order. **ADR 0046 has the *Verification* section
> M4's acceptance names for it**, written over the four cases that pin it rather than the one an ADR
> usually owes: two run and two refuse, because a `.nvst` has one verdict and a compile that reports
> a diagnostic runs nothing after it. It states the three things no case can assert — that *3*'s
> repeatability is asserted by a count rather than off a line, that *6*'s explicit `<T>` needs no
> case because every retrieval is written through it, and that "no runtime lookup exists" is
> verified by a program running at all, `nvs_stdlib::attributes` registering two symbols whose body
> aborts. **ADR 0007 § 6's narrowing list has two more of its four spellings now, and both are one
> function each beside the `!= null` one they join.** `nvs_types::locals`' `narrow` had read `==
> null`/`!= null` and nothing else, so a union reached a member's own operations only through an
> `as`; it now tries three residue functions in turn, no two of which can match one condition.
> **`instanceof` proves the class it names**, on the edge where the test holds alone — a `!` inverts
> which edge that is rather than removing it, which is the guard clause a ported program writes,
> while the plain false edge proves nothing at all, every other class the declared type admits and
> `null` besides still being in it. The class comes from `crate::expr_table::ExprInfo::InstanceOf`,
> recorded a moment earlier when the condition was checked, rather than resolved a second time: a
> name is placed by the namespace and the imports of the site that wrote it, and this walk carries
> neither. The residue is restricted to a **class** — a declared one or a reserved global exception
> class — because the narrowing is discharged as an unchecked `nvs_ir::ir::InstKind::Untag` wherever
> the slot is `Ty::Tagged`, and an interface on the right would name ADR 0053 § 2's
> `Iterable`/`Iterator` with type arguments this test does not supply; `$x instanceof Comparable`
> therefore narrows nothing, which is a limit of the pass and is stated at `instanceof_residue`
> rather than left to be rediscovered. A subject whose declared type can hold no object narrows
> nothing either, that being `E0496`'s own question asked a second time so that a reported mistake
> cannot also hand `nvs-ir` a class where the slot holds a scalar. **A comparison against a written
> literal proves that literal's own type**, which is ADR 0047 § 4's guard row — `==` on the edge
> where it holds and `!=` on the edge where it does not, either operand order, ADR 0090 § 1 having
> left one symmetric operator. The type is built from the literal's **text** rather than from what
> the condition inferred, because an operand's inferred type is recorded nowhere and re-inferring
> one would report its escape-grammar diagnostics twice; that is also why the roster is closed at a
> string and an integer literal, which cook to a value with no context at all, while an enum case
> needs the writing site's namespace and reaches `ExprInfo::EnumCase`, which carries the case's
> backing value rather than its type. The residue must be a **subtype of what the local was
> declared**, so this is a guard reaching one member of a union and never a re-declaration. It costs
> nothing below the checker: ADR 0047 § 5 gives a literal type its base type's representation
> exactly, so the narrowed read is the read it already was. **ADR 0007 § 6's narrowing list is
> closed at all four spellings.** The last two were one function each beside the three residues they
> join. **A `match (true)`/`switch (true)` label is a condition**, so each arm body is checked under
> whatever `narrow` installs for that label — `nvs_types::locals::is_true_literal` is the one home
> of which subject qualifies, and only the *written* literal does: what makes the spelling narrow is
> that the label's own truth is what selected the arm, while a `bool` local holding `true` says only
> that the two agree, and `$flag == ($x instanceof Foo)` proves the class on neither edge. A
> `default` arm and a comma-separated run of labels are deliberately given nothing — `match (true)`
> takes the first label that held, so reaching a later arm says the earlier ones did *not*, which is
> a residue this pass has no way to express, § 6's narrowings each naming a type rather than
> removing one. Both sites take it, the `Match` expression arm in `nvs_types::expr` and the `Switch`
> statement one in `locals`, and a case falling through to the next is the known gap that module doc
> already names rather than a new one. **A comparison against an enum case narrows to that case's
> own type**, which is ADR 0047 § 4's guard row over the third of the three spellings
> `literal_residue` admits and the one that could not be read off the operand's text: a string and
> an integer literal cook to a value with no context at all, while which enum a written `Read`
> belongs to is a question about the writing site's namespace and its imports, which that walk
> carries neither of. So it is read back off `ExprInfo::EnumCase`, recorded when the operand was
> checked a moment earlier, exactly as `instanceof_residue` reads its own class — and that record
> gained the enum's `QName` and the case's name beside the backing value it already carried, because
> the value alone cannot answer it: two cases of two enums may share one integer. The residue is §
> 3's `Ty::EnumCase` and not the whole enum, which is the entire point of the guard, and it costs
> nothing below the checker for § 5's reason, a case being its backing integer in every
> representation. Two cases pin the pair, `a-match-and-a-switch-over-true-narrow-per-arm.nvst` over
> the three residues under both subjects with the non-`true` subject and the `default` arm beside
> them, and `an-enum-case-comparison-narrows-its-subject.nvst` over the whole-enum subject, the
> declared union of two cases, either operand order and the `!=` edge — each counting its agreement
> with the plain `if` spelling of the same guard rather than reading it off a line. **A method call
> is refused wherever the receiver's type names no class**, and that is the answer to whether a
> union takes a code of its own: it does not. `E0477` was the *erased* family alone — a plain
> `object`, an ADR 0036 shape — while a `Dog|Cat`, a `string`, an `array<int>` and a `void` call's
> result each resolved nothing, reached `nvs-ir` with no target recorded and panicked at
> `lower/expr.rs:2389`, naming a compiler gap for a mistake in the program. It is one code across
> the whole family now, because it is one mistake and it is the same resolution that fails: a method
> is resolved against a class, and none of those names one. That is also where the call half parts
> company with the property one, which splits a *deferral* off from `E0495` — ADR 0036 § 4 answers
> an erased property read with a name-keyed runtime fetch, and a call additionally needs a signature
> to check its argument list against and a return type for the position it sits in, which no
> receiver here supplies, with no `__call` to fall back on. Only the help splits, three ways because
> the fix does: a receiver that can hold an object is narrowed, `can_hold_an_object` being that
> question's one home and asked here for the third time; one that cannot is converted or declared
> `mixed`, which is the property half's own wording; and a `void` call yields no value for either
> fix to be about. A receiver that may be `null` whose non-`null` half *is* a class is left alone —
> `E0459` named it on the way in and it is one mistake rather than two, which is again the property
> half's guard read a second time. `mixed` is the one receiver deliberately still deferring: ADR
> 0007 § 2 makes it the one unchecked position, so `nvs-ir`'s panic roster names it alone now and
> the lowering that answers it from the receiver's runtime class is the next group.
> `docs/adr/README.md` § *Decisions taken at project start* owns that split, and the paragraph
> beside it now owns the deferral's own **convention**: a call through a `mixed` receiver is
> marshalled by the receiver's own descriptor rather than by a per-method thunk. Almost nothing has
> to be marshalled at all, which is the fact the design turns on — ADR 0002 makes one calling
> convention normative for every call, so `nvs_runtime::abi::NvsFn` already takes an array of
> 16-byte tagged `Value`s and one tagged `out` slot, and `nvs-codegen`'s `store_value`/`load_value`
> already write each argument and each return *with* its tag while a typed callee reads only the
> payload half. So a site holding tagged values has tagged slots to fill and gets its answer back
> tagged for the `mixed` the call's own type is, with no conversion in either direction. What is
> missing is the callee's **declared shape** — its arity and which tag each parameter requires —
> without which slot *i* is reinterpreted at the callee's own representation and an `int` handed to
> a `string` parameter is an arbitrary dereference rather than a fault, the identical hole
> `nvs_runtime::closure`'s module docs describe for `callable`, arrived at from the other side. The
> method row on `ClassDesc` therefore carries them the way a closure object already carries
> `FN_ARITY` and `FN_PARAM_TAGS` — the same nibble word, the same `CLOSURE_PARAM_TAG_ANY` for a
> parameter whose representation is itself a tag — resolved once per class in
> `ClassTable::set_methods` as `renderer` and `unwind` already are, with `check_param_tags` the one
> implementation both paths share rather than a second copy of ADR 0007 § 2's `int`-into-`float`
> widening. The thunk loses on three ranks of the priority ordering at once: `nvs-codegen` would
> emit the tag rules a second time where a safety check wants one implementation (1), a thunk is a
> second frame that still needs the same name lookup to be found at all (3), and it spends a whole
> compiled function per method in every unit against sixteen bytes on a descriptor (5). Three shapes
> are answered by a catchable throw rather than by dispatch, each because the row cannot describe
> them and not as a rule about erasure: a non-`public` member, since a `mixed` receiver is outside
> every class by construction; a variadic or `inout` parameter list, packed and written back at the
> *call site*, which is the limit `E0721` already names for ADR 0043 § 4's synthesized forward; and
> a `Core`-owned class, whose native members *borrow* argument 0 where a compiled method owns its
> parameters — the very difference that made `renderer` its own descriptor field rather than a row
> in the table. **The descriptor half of that convention is built.** `nvs_runtime::MethodRow` is
> what a method table holds now — the compiled address plus the callee's declared shape, its `arity`
> and the `FN_PARAM_TAGS` nibble word, in the encoding a closure object already carries and through
> the one packing both go through (`nvs_ir::lower::pack_param_tags`), so the two paths
> `check_param_tags` serves cannot be handed two encodings. `nvs-codegen` records each function's
> shape in the very declaration pass that gives it a `FuncId`, under the very
> `{declaring}::{method}` label the address is later found by, so the two halves of a row cannot
> describe two different callees; and the receiver is subtracted in `MethodShape::of` alone, because
> a row describes what a *call site* writes while `nvs_ir::ir::Function::params` describes what the
> callee declares — off by that one, every nibble judges the argument beside the one it describes,
> which is not a wrong count but a shifted word. The `public` bit is the one fact with no source
> below the front end and it travels the whole way now: `nvs_types::layout` reads it off the
> declaration and `nvs_ir::ir::Class::methods` carries it as a third element, an absent visibility
> keyword reading as `public` because `nvs_syntax::check_declarations` already refuses the source
> that omits one and because the synthesized members write no modifier at all — an exception
> constructor, ADR 0053 § 4's state machine, an ADR 0043 § 4 forward, a closure's `invoke`. One
> field was added that the ADR's paragraph implied rather than named, and it is the difference
> `renderer` already stands on: `MethodRow::native` is true for exactly the `Core`-owned rows
> `nvs_stdlib::instance` puts in a descriptor, whose addresses are ADR 0002 helpers that **borrow**
> argument 0 where a compiled method owns its parameters and for which that crate holds no signature
> at all — so the `Core` refusal that paragraph names arrives as a bit the row states rather than as
> a shape a caller could believe. **The checker's half of that deferral is built.**
> `nvs_types::expr::calls::infer_method_call` records `ExprInfo::ErasedCall` for a `mixed` receiver
> — the member name and nothing else, every written argument filling its own position because there
> is no signature to map one against — which is the checker saying "dispatch on the value" where
> every other receiver naming no class says `E0477`. Three spellings the deferral cannot express are
> refused where they are written instead, and none of them is a rule about erasure: a `name:`
> argument takes `E0712` for the same reason it takes it through a `callable`, the method row that
> marshals the call carrying the callee's arity and its parameter tags and never their names; an
> `inout` marker takes `E0714`, an `inout` parameter list being packed and written back at the *call
> site*, which a call whose callee is chosen when it runs cannot do — the limit `E0721` already
> names for ADR 0043 § 4's synthesized forward; and ADR 0027's first-class callable spelling
> `$m->method(...)` is `E0732`, because it makes no call at all but names a closure **value**, which
> carries its callee's arity and parameter tags in the value itself and so would need a class this
> site does not have. The two refusals shared with `callable` are one walk over one wording pair
> (`report_args_with_no_parameter_list`), so the two sites cannot grow two answers to one question.
> **The lowering and the dispatch are landed, and `lower_method_call`'s panic roster is empty**: a
> `mixed` receiver is one `Helper::CallErasedMethod` (`nvs_ir::lower::call`'s
> `lower_erased_method_call`), whose own doc comment argues the shape — the receiver travels still
> **tagged**, `ReceiverProof::Erased` emitting no `Untag`, because nothing proved it holds an
> object; the member name is an `InstKind::ConstStr`, which is an immortal address in the unit's
> data section rather than an allocation per call; and every argument is packed into **one array**
> rather than one helper slot each. The packing is `CallClosureArray`'s reason and here it holds at
> every site, not only a spread's: a helper's argument count is a literal `nvs-codegen` writes
> beside the slot, while what this list is judged against is a callee chosen when the call runs. The
> call's type is `Ty::Tagged`, `mixed` being the only answer the checker has for a target it cannot
> name, and its ownership is the closure call's throughout — receiver and array both borrowed,
> released on both edges. `nvs_runtime::dispatch::call_erased_method` is the other end and it makes
> every judgement a checker would have made, each as a **catchable** throw: a receiver whose tag is
> no object, a class whose table has no such member, a member that is not `public`, a `native` row,
> too few arguments for the arity the row records, and an argument whose tag is not the one the
> parameter requires. That last one is not a second copy of anything — `check_param_tags` takes a
> **word** now rather than a closure object, so ADR 0007 § 2's `int`-into-`float` widening is
> written once and the `callable` path and this one cannot drift, `closure_param_tags` being the
> closure half of where the word comes from. Two refusals are worded off the class rather than off a
> row, and both are only reached on the failing edge: a **`Core`-owned** class carries no compiled
> method table at all — only `nvs_stdlib::instance`'s engine-protocol rows — so `has no method`
> would be a plausible wrong answer for a member the spec plainly gives it, and the reserved `Core`
> namespace (ADR 0011 § 2) is what makes the descriptor's own name enough to tell. The `Core`
> refusal that the ADR's paragraph names therefore arrives twice over, as the `native` bit for a row
> that exists and as the namespace for the members that never reached a table. Valgrind-clean over a
> fixture that calls through a freshly built receiver and abandons a freshly built `string` argument
> on the refusal's error edge two hundred times.
> `tests/conformance/lang/a-call-through-a-mixed-receiver-is-dispatched-on-its-value.nvst` pins the
> dispatch itself — a subclass override reached through a receiver that names neither class, a
> spread whose count is its own length, the one implicit widening, the callee's own throw travelling
> back on ADR 0002's error edge, all six refusals, a `?->` receiver that ran no callee — and counts
> nine agreements between the typed and the erased spelling of one question rather than reading one
> off a line.
> `tests/conformance/reject/a-call-through-a-mixed-receiver-refuses-what-it-cannot-defer.nvst` pins
> all three in one compile with the positional call and the `...` spread — whose count is its own
> run-time length — written first.
> `tests/conformance/reject/a-method-call-through-a-receiver-that-names-no-class-is-refused.nvst`
> pins all four refusals plus the nullable receiver's single code in one compile, with the two
> spellings that do resolve — a named class, and a union narrowed by `instanceof` — written first,
> so a position that stops resolving fails there rather than as a missing refusal. Three live tools
> **are** the worklist and no session re-derives one: `python tools/holes.py` reads the refusal
> sites out of `nvs-ir` and `nvs-codegen` and attributes each to its item (`--item N` for one in
> full), `python tools/loop.py --list` prints the named `.nvst` cases each stage still owes, and
> `python tools/check-migration.py` scores `docs/spec/02-php-migration.md`.
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
