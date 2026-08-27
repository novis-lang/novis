# MWL — Modern Web Lang: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-27. Current: **M4**, language completeness. M3 is done, spec §§ 1-12 is
> registered whole, the previous loop's Stage 0 catch-up list is closed, and **M4S Part I stands at
> 552 of its 600 conformance cases with both named guards green** — it is now the floor and the
> corpus gate of a new goal rather than the frontier. **The loop is aimed at M4**: every shape that
> compiles in the front end and then refuses below it, closed, before M4B's LSP is written against
> the surface. [docs/agent/loop-goal.md](agent/loop-goal.md) holds the 43 items, grouped by file
> set, and `python tools/holes.py` is the live worklist behind them — 45 refusal sites in `mwl-ir`
> and `mwl-codegen`, attributed to the item that names their function.
> `crates/mwl-stdlib/tests/spec-members-outstanding.txt` holds no keys: the ratchet
> `spec_registry_coverage.rs` reads is empty, which is this project's own definition of Part I being
> *registered*, and all seven Stage 3 fixtures produce their frozen output. **Every representation
> the library needed is built and recorded in the crate that owns it** — `mixed`/`?T`/every union is
> `mwl_ir::Ty::Tagged`, strict identity is `mwl_runtime::identity`, `decimal` is
> `mwl_runtime::decimal`, a `Core`-owned instance is an ordinary MWL object
> (`mwl_stdlib::instance`), a variadic tail is one `array<T>` argument, and a sink's carrier is a
> `Core` instance whose one slot `mwl_runtime::value_to_string` renders (ADR 0088 § 5). **ADR 0087's
> lexer half is built** — `mwl_syntax::bidi` is the one predicate, `E0008` at the lexer, and a
> substitution at both sinks at M7/M8. **ADR 0090 is built whole**, §§ 1, 2, 3 and 5 alike, and
> **ADR 0069's `+`/`+=` refusal** is `E0467` naming `Core\Arr::underlay`. **ADR 0071's first
> compiler-recognized attribute is built** — `#[Json\Derive]`, matched nominally in
> `mwl_types::derive`, decoder included for a scalar-fielded class — and `ParseError` carries § 10's
> `issues`. **ADR 0070's duration literal lexes, types and runs.** What M4 still owes is stated as
> items rather than prose: the operator table (promotion, integer `/`, overflow, the bitwise rows,
> `**`, `<=>`, `++`/`--`), control flow (`do`/`while`, `break 2`, a two-representation ternary,
> `finally` through a throwing `catch`, an abandoned generator's `finally`), calls (a closure
> through the variable holding it, named and spread arguments, `&$x` anywhere), targets (a static
> property, a nested element write), `mixed` dispatch, ADR 0007 § 2's last conversion rows, and the
> five declared features with no slice at all — `PropertyObserver`, ADR 0043's `by`-delegation, ADR
> 0046's retrieval, ADR 0092's `Core\Debug::dump`, ADR 0079's `#[Test]`. Dependencies: `regex` +
> `fancy-regex` and `jiff` are named by the user; the rest the loop picks under ADR 0051 § 4.
>
> **Done:** M0 (setup); M1 (front end — lexer with dual mode, inline HTML, heredoc/nowdoc and
> interpolation, the full parser, and the M1-scoped grammar of ADRs
> 0024/0031/0033/0036/0037/0043/0046/0049/0050/0066; `crates/mwl-syntax/tests/corpus_parse.rs` parses the
> local `php-src` checkout and a 5-minute WSL `cargo fuzz run lex`/`parse` both find zero panics).
> **M1 was re-opened for four grammar additions**, each accepted after it was reported done and each
> blocking its ADR's already-scheduled M2 slice; **`decimal` (0054) is built, grammar and checker rows
> alike, and 0047's literal/enum-case type atoms are built as grammar** — their checker rows are still
> owed, and the atoms are refused by name until then. The **duration literal (0070)** is now built too,
> lexer through IR; `autoload` (0061) is the one left. M1's own section lists it.
>
> **On disk:** the workspace, CI on three platforms with miri, asan and fuzz legs beside it,
> lint/deny/fmt/notice policy, `mwl-diagnostics`, `mwl-syntax` (+ `duration`, ADR 0070's one
> grammar, and `bidi`, ADR 0087's one predicate), `mwl-hir`, `mwl-types` (+ `layout`, `core_lib`,
> `error_lib`, `iter_lib`, `generics`, `conformance`, `defaults`, `derive`), `mwl-ir`, `mwl-runtime`
> (+ `alloc`, MWL's own `#[global_allocator]` in an optimized build and the `sanitizer` feature that
> takes it back out from under ASAN, `object`, `array`, `throwable`, `closure`, `identity`,
> `decimal`, and `ctx`'s capture stack and carrier roster), `mwl-stdlib` (`Arr` × 36, `Str` × 39,
> `Math` × 38, `format`, `granularity`, `ordering`, `cldr`, `instance`, `issue`, `Order`,
> `NormalForm`, `RoundMode`, `Unit` and `Weekday`, `Regex` × 6 plus `Regex\Match` × 4 over
> `regex`/`fancy-regex`, `Time` × 7 plus `Time\Instant` × 9, `Time\DateTime` × 14, `Time\Duration` ×
> 19 and `Time\Zone` × 4 (+ `UTC`) over `jiff`, `Json` × 4 over `serde_json`, `Path` × 9 (+
> `SEPARATOR`) over nothing at all, `Random` × 6 over `rand`, `Uuid` × 4 (+ `toString`) over `uuid`,
> `Uri` × 4 over nothing at all, `Encoding` × 2 over nothing at all, `Hash` × 4 plus `Hash\Stream` ×
> 2 over the RustCrypto family, `crc32fast` and `subtle`, `Csv` × 2 over `csv-core`, `Out` × 1
> answering the `Cli\Text` carrier, `ObjectMap` × 9, `ObjectSet` × 9 and `Heap` × 5 over
> `identity_store`, all three iterable through `cursor`, and the conformance-coverage gate),
> `mwl-codegen`, `mwl-cli` (`ast`, `check`, `run`, `test`, `info`), `mwl-test` (+ `case`, `expect`,
> `run`), `tests/conformance` × 603 (in `array`, `class`, `core`, `enum`, `error`, `iter`, `lang`
> and `reject`) and `tests/differential` × 167, `fuzz/`, `tools/`, `benches/abi-probe`.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows
> SDK 10.0.26100 for linking, PHP 8.5.9 as the differential oracle — on the Windows `PATH` and
> inside the WSL distro alike, at the same version — `cargo-fuzz` 0.13.2 and `valgrind` under a WSL
> nightly toolchain (docs/setup.md is what a machine installs, and why).
>
> **ADR slices landed:** checker-side rules for ADRs 0007, 0010, 0013, 0014, 0015, 0021, 0022, 0024,
> 0027, 0028, 0029/0030, 0033, 0036, 0037, 0038, 0054, 0062, and 0043's syntax +
> default/private-method slice; end-to-end for 0007 §§ 2 and 4's `%` row, 0010, 0013, 0014 § 1, 0023
> § 1, 0035 § 4, 0031 §§ 1-2, 0065, 0036 § 2's anonymous literal in every position it writes and
> § 4 **whole** — the shape-field read and write, the plain-`object` receiver's read and write, and
> the missing-name throw all name-keyed and all right through a widened view, with every named
> class's slots carrying the declared tag that write is checked
> against, **0029/0030** (the casing checker existed but no pipeline called
> it), **0053 and 0009 in full**, **0054 §§ 1–4 in full** (`Core\Decimal`'s own roster is M8's, and
> that ADR's *Verification* says so), and **0066 §§ 1–3 for the checked numeric targets** — a
> nullable binding, parameter, property and return, `null` itself, `??` and now `as
> ?int`/`?uint`/`?float`/`?decimal` all run, leaving that ADR's enum target and its
> cannot-fail/no-conversion *refusals* owed; its **class** refusal is built and is now absolute
> (`E0473` for every class target, the withdrawn parse roster included), with § 3a's `tryParse` the
> member that answers a parse instead, and **0056 §§ 1, 2 and 5** — both engines are bound, the tier is chosen by the pattern and
> the backtracking budget throws, leaving § 3's compile-time tiering (which waits on ADR 0057) and §
> 4's pattern sink (which waits on a qualifier the registry can state), and **0070 in full but its §
> 3 constant** — `30s` lexes as one token, types as `Core\Time\Duration` and runs, over the one
> grammar `mwl_syntax::duration` holds for the lexer, `Duration::parse` and M6's `mwl.toml`; what is
> owed is the *immortal* constant-pool value, which `mwl-runtime`'s gap 3 owes a string literal too,
> and **0071 §§ 1–3, § 5 and § 6 in full for a scalar-fielded class** — the nominal match, the field
> list, both per-field overrides, the three refusals, `ParseError::issues` and now the generated
> *decoder* behind `Core\Json::decodeAs<T>` all run: a decode is an ordinary `new`, every bad field
> is reported from one throw, and `?T` accepts a present `null`. What that decoder still owes is §
> 2's wider codec-reachable set and § 4's two default-bearing rows, both `mwl_stdlib::json`'s own
> gaps. and **0087's lexer half in full** — `mwl_syntax::bidi` is the one predicate, `E0008` is a
> hard error with no suppression over comments, string literals and inline HTML per line, and that
> ADR's two sink callers are M7's and M8's, and **0090 § 1 in full** — `===`/`!==` are `E0232` at
> the lexer, the two token kinds and the two `BinaryOp` variants are deleted, and the null test
> narrows and lowers through `Eq`/`NotEq`, and **0090 § 2 with 0069 § 2** — the two operand
> refusals, `E0466` for an equality between two statically disjoint types (which § 6 points a
> `switch` label and a `match` arm at as well) and `E0467` for `+`/`+=` with an array operand.
> Instance calls dispatch on the receiver's runtime class. Each ADR's own *Verification* section
> says what its slice covers, not this field.
>
> **Open now:** **The frontier is M4's language holes, and there are 43 of them**, ordered and
> grouped by file set in [docs/agent/loop-goal.md](agent/loop-goal.md). A hole is a shape that
> compiles in the front end and then refuses below it, and it is closed when it either runs with a
> fixture or a `.mwlt` case pinning what it prints, or is refused by a **diagnostic that names the
> rule** — never by a panic. **`python tools/holes.py` is the worklist and no session re-derives
> it**: it reads the refusal sites out of `mwl-ir` and `mwl-codegen` live, attributes each to the
> item whose prose names its function, and prints what no item claims (45 sites, 13 items, 2
> unattributed today). `--item N` is one item in full. `python tools/loop.py --list` prints the 32
> named `.mwlt` cases each stage owes and which are still to write. **Stage 0 is the operator table,
> and its last two items are open**: ADR 0007 § 4's promotion rows run (`1 + 1.5`, `$n < $f`, and §
> 2's implicit widening at a binding), integer `/` answers PHP's `int|float`, `+`/`-`/`*` and unary
> `-` throw `ArithmeticError` on overflow instead of wrapping, all six bitwise operators lower with
> PHP's own shift rules, `**` answers every numeric row and throws where that row has no integer,
> and `<=>` answers `-1`/`0`/`1` for every scalar `<` already orders — `$x++`/`--$x` lower in either
> position over a target whose address is computed once, and `==` over two enum values answers one
> representation down. **Stage 0 is closed**: its four named cargo guards are written, and `break
> N`/`continue N` lower at any level with a level naming no target refused as `E0475`, so `python
> tools/holes.py` is down to 11 items and 42 sites. **Item 11 is closed too**: a ternary's, an
> elvis's and a `match`'s branches in two representations join at the tagged one — the erasure of
> the union the checker already typed the whole expression as, never a promotion of one branch into
> the other — and an arm-less `match` is `E0476` where it is written rather than a panic below it,
> so the worklist is down to **10 items and 39 sites**. **Item 25 is closed as well**: `object` is a
> declared type wherever a class name is one — the representation arms were already on disk, and
> what this session owed was the check that nothing below reads a class *label* off one, which found
> exactly one reader and refused it. A method call through an erased receiver (a plain `object`, or
> a shape) is `E0477` where it is written, because ADR 0036 § 4 gave the *property* half a
> name-keyed runtime fetch and stopped there on purpose. `holes.py` still shows item 25 at 2 sites,
> and both are honest but misattributed: they are `lower_decl_type`/`lower_checked_ty`'s
> **catch-alls**, whose remaining uncovered shapes are `decimal`, `never`, `iterable`,
> `self`/`static`/`parent`, a shape type and an intersection as a *declared* type — a different hole
> that shares item 25's file and that no item names yet. Even so, `tools/loop.py` runs it before the
> program legs, because every fixture and every case in every stage below is written against those
> rules. **Item 22's own hole is closed**: a nested `$grid[0][1] = v` flattens to its root plus one
> key per level, descends, and writes every level back with the outermost last, so ADR 0007 § 5's
> separation runs at every level rather than being silently dropped above the innermost one.
> **Auto-vivification came with it and was not optional**: `InstKind::ArrayGet` answers an absent
> key with a null-shaped value, so descending through one aborted the process rather than refusing,
> and the fix is one new `Helper::ArrayRowForWrite` whose answer is uniformly one owned reference —
> a retain of the row that was there, or a fresh empty array where PHP would build one. `$g[9][0] =
> 1` over an empty `$g` now prints what PHP prints. **Item 22 is closed.** Both holders that still
> stood under it are diagnostics now rather than panics, and they are one function:
> `check_write_target`, beside `check_assign` in `mwl_types::expr::assign`, called from both the
> plain and the compound arm once the target has been checked. An element write through an ADR 0014
> § 1 hooked property is **E0478** — PHP raises "indirect modification of overloaded property" at
> run time, so refusing where it is written is the compatible answer and not a divergence. A
> nullsafe assignment target is **E0479**, which PHP also refuses. An element write through a
> property whose receiver erased to a shape or a plain `object` is **E0480**, the element-write twin
> of item 25's E0477: ADR 0036 § 4 gave such a property a name-keyed runtime *fetch* and stopped
> there, which is enough to read one and not enough to give a separated array a slot to land in.
> Every subscript spelling takes the same refusal, because the walk goes to the chain's root —
> `[0]`, `[0][1]`, `.=` and `[]` alike — while `$this->p[0] = v` inside the property's own hook
> still writes the backing slot and runs. `python tools/holes.py` is down to **9 items and 38
> sites**. **That append at an intermediate level now lowers too**: `$g[][0] = 1` is PHP's "start a
> fresh row and write into it", so the flatten walks every `Index` level rather than only the
> subscripted ones, an append level's row is an empty `InstKind::ArrayNew` because it has nothing to
> descend into, and the climb back out stores it with an `InstKind::ArrayAppend` — which is why the
> fresh row's key is never named anywhere in the target. **Gap 23 is closed, and it took the whole
> `Index` arm with it.** `$a[]` anywhere but a level of a plain `=`'s target chain is **E0481** —
> `echo $a[];`, `unset($a[])` and `$a[] .= "x"` alike — with the legal spans marked by a walk down
> the target chain before the target is checked, since the arm that reports is inside that check.
> `$a[] .= "x"` is the one spelling PHP *accepts*, appending because the element that is not there
> yet reads as `""`; declining that coercion is now ADR 0007 § 7 **row 10**, the § 7 table's first
> new row in a long while. And a subscript whose base declares no element type at all is **E0482** —
> a `mixed`, a scalar, a `string` (ADR 0009 § 2 indexes one through `Core\Str`, not through a
> subscript), or a `?array<T>` no test narrowed — which is the `mixed $m; $m["0"] = 1;` slice of the
> previous group and closes both of `lower_index`'s panics plus the assignment arm's matching one.
> Two suppressions keep it from double-reporting: a base that already reported its own error, and a
> write-target level whose chain root `check_write_target` is about to refuse by name
> (E0478/E0479/E0480). `python tools/holes.py` is down to **35 sites**, still 9 items. **One live
> bug went with it**, found by the same file set and not on the worklist because it was never a
> panic: an element write through a *narrowed nullable* receiver (`?Box $m = new Box(); if ($m !=
> null) { $m->rows["0"] = "w"; }`) was rejected by cranelift, because `write_back_array`'s property
> arm lowered the receiver and stored through it without the `untag_receiver` every other write
> through a property already goes through — a `?T` local being one tagged slot wide however narrow a
> condition proves it. **Reading an absent key throws now**, which closes the live abort the
> previous group found on its way past: `InstKind::ArrayGet` answered a missing key with a
> null-shaped value and every consumer read it as its declared type, so `array<string> $a = []; echo
> $a["9"];` died in `mwl-runtime` on a null dereference — a non-unwinding panic, exit 0, nothing on
> stderr. It is ADR 0007 § 7 **row 11** instead: PHP warns and yields `null`, MWL has no `null` to
> put in an `array<string>`, and row 8's rule (absent storage is never a zero value) now holds at
> runtime as well as at check time. A stored `null` in an `array<?T>` is **not** an absent key and
> reads back unchanged, which is why the read is built on an `Option` from the table rather than on
> the null-shaped answer. The instruction is fallible now — ADR 0002's error edge, `emit_fallible`,
> a landing block — and `mwl-codegen` emits it as one helper call to `mwl_array_required_get` rather
> than as a choice between two borrowing primitives, so the `Ty::Str`/`Ty::Int` key split is decided
> by the key's own **tag** at runtime and the two read signatures are gone from `Signatures`. The
> rendered key a `uint` subscript builds is staged on the owned-temporaries stack instead of
> released inline, because the read below it can now leave through the landing block;
> `tools/leak-check.sh` is green over a fixture that throws with one live. **The one read that does
> not throw is the one under a `??`**, which was a live wrong answer rather than a worklist item:
> `lower_coalesce` short-circuits away a left operand whose representation is not `Ty::Tagged`, so
> `$a["k"] ?? "d"` never ran its `??` at all and the read under it threw where PHP yields the
> default. `mwl_types` marks the operator's immediate left operand (`Env::coalesce_guarded`), the
> `Index` arm answers `?elem_ty` for a marked one and records it on the `ExprInfo::Index` entry, and
> `mwl_ir::InstKind::ArrayGet` carries an `AbsentKey` that picks between throwing and answering
> `null` — the guarded read being infallible, `Ty::Tagged`, and one call to a new
> `mwl_array_optional_get` beside the required one. ADR 0007 § 7 row 11 states the exception in its
> own cell, since `??` is defined as "absent or `null`, without the warning" and refusing there
> would refuse the very spelling PHP offers for the safe read. Only the immediate operand is
> guarded: `$a["k"]["j"] ?? "d"` still throws at the inner level, because the `??` marks only its
> own immediate operand. **Every `?T` narrows out of `null` now, not just a `?Class`**:
> `mwl_types::locals`' `narrow` keeps whatever dropping `null` leaves — an `array<T>`, a scalar, a
> class — and what unblocked that is where the fact is *discharged*. It is recorded on the variable
> read's own span (`ExprInfo::NarrowedRead`) and turned into one unchecked `Untag` in `mwl-ir`'s
> `Variable` arm (`Lowering::untag_narrowed`), rather than at each consumer that wants the narrow
> representation, so a subscript base, a `foreach` subject, an array-write root, a call argument and
> a receiver all see it with no site left to forget — one forgotten site would have been a cranelift
> rejection, not a panic. `if ($m != null) { echo $m["k"]; $m["j"] = "w"; }` over a `?array<string>`
> runs, and so do `$n + 1` over a narrowed `?int` and `$s . "c"` over a narrowed `?string`. The
> array-write root re-tags on the way back into the local, because the slot is one tagged slot wide
> however narrow the guard is and the narrowing ends with it. `E0482` is the *untested* nullable
> array only now, and its help names the test rather than the two workarounds it used to; only a
> **binding** narrows, so a nullable array straight out of a call still has to be bound to one
> first. **An array literal now takes its element type from a `?array<T>` expectation**:
> `check_array_literal` strips the expectation's `null` before it looks for the `Ty::Array`, so
> `?array<string> $m = ["k" => "v"];` type-checks instead of being an `array<mixed>` at an `E0401` —
> and so does a `return` of one and an argument of one, the three positions being one helper reading
> one expectation. The element type reaches the elements with it, so an untyped numeric literal in
> an `?array<uint>` becomes a `uint` by being placed. **`??` now guards every level of a subscript
> chain under it**, not only its immediate operand: `$a["nope"]["j"] ?? "d"` threw where PHP yields
> the default, because the inner read was never marked. Every `Index` level below a `??` is marked
> now, a guarded level drops `null` from its base type before reading the element type off it —
> which is what lets a guarded read's own guarded base resolve at all — and `mwl_array_optional_get`
> answers `null` for a `null` array rather than the tag mismatch a well-typed program otherwise
> cannot produce. `$m["k"] ?? "d"` over an untested `?array<string>` is PHP-identical too, that
> being the one position where a nullable array needs no `!= null` test, and ADR 0007 § 7 row 11's
> cell states the chain-wide rule. **Item 1 is behaviourally closed and its 11 sites are
> catch-alls**: every row of ADR 0007 § 4's promotion table runs today — `$n + $f`, `$n * $f`, `$n
> ** $f`, `$n < $f`, `$n <=> $f`, `$u + $f` — so `mwl-codegen`'s mismatched-representation refusal
> is reachable only by a pair no widening exists for (two `string`s under `<`, an `Enum` under any
> operator), and the playbook bullet that said otherwise is corrected. **Unbuilt in the library**,
> none of it a registration gap: `Core\Json::decodeAs<T>`'s wider codec-reachable set and its two
> default-bearing rows (`mwl_stdlib::json` gaps), ADR 0088's qualifier classification
> (`mwl_stdlib::hash`'s module doc), and ADR 0086 § 1's substitution table (M8,
> `crates/mwl-stdlib/src/cli.rs` gap 1). **Decided and unbuilt, and out of this goal's scope** —
> ADRs 0091, 0092 § 2's log levels, 0093, 0097 and 0100 § 3; their work is M6, M7, M8 and M10.
> **Item 17's `&value` half is closed outright, and its spread half type-checks.** `&value` as an
> array-literal element is **E0483** — ADR 0031 § 2 removed by-reference capture and ADR 0023 fixes
> an element as a copy, so an aliasing element has no owner in either and it is a shape the language
> does not have rather than a lowering `mwl-ir` has not learned; `lower_array_literal`'s assert
> names only `...spread` now, and every element list is walked at every depth so a nested `&` is
> refused where it is written. `[...$a]` is checked in the same loop and by the same move the
> surrounding elements already use: the subject takes `array<T>` for the literal's own `T` as its
> *expectation*, so a mismatch is an ordinary `E0401` naming both array types and element covariance
> falls out of it — an `array<string>` spreads into an `array<mixed>` for the same reason reading
> one does. **E0484** is only what a position naming no `array<T>` at all is left with, a `mixed`
> binding or parameter being the reachable one, so `[...$s]` over a `string` is one diagnostic
> however it is written. Nothing is recorded on the `ExprInfo` side for a spread, and that is a
> decision rather than an omission: the lowering reads `ArrayItem::spread` off the AST it already
> walks and its own `lower_expr` hands back the subject's `Ty::Array` beside the value, so an entry
> would be a second copy of two facts it holds already. **Item 17 is closed.** A `...spread` element
> is one `InstKind::ArraySpread`, one runtime call (`mwl_array_spread`) that walks the subject once
> — not a lowered loop over the `foreach` cursor, which would spend three calls and a branch per
> entry and put a control-flow join inside an expression that has none. **The key semantics are
> PHP's, and ADR 0007 § 5 is their home**: a key that reads as a canonical decimal integer is
> renumbered under the destination's own append counter, every other key is preserved in place, so
> `[...$xs, ...$ys]` concatenates and `[...$defaults, ...$overrides]` overrides by name. That is the
> same canonical-decimal reading `$a[]`'s counter has always kept, selecting between the two
> *writes* the language already has rather than between two rules — which is why it does not re-open
> ADR 0069, whose own Context paragraph now says where the line is: a spread is written at the site
> with both arrays in view, where `merge($a, $b)` is a name whose behaviour changes underneath one.
> A keyless element of a literal *containing* a spread is an `ArrayAppend` rather than a
> lowering-time index, since how many entries arrived is the subject's run-time length, so such a
> literal does not share the keyed shape's one remaining divergence. Because a renumbered key is an
> append, a spread throws exactly where an append does, and `["9223372036854775807" => "z", ...$a]`
> prints PHP's own *"Cannot add element…"* message. Two new refcount edges, both valgrind-green: the
> subject is **borrowed**, so a freshly-built one is staged as an owned temporary, and the array
> under construction is staged too and re-pointed after every write — it is named by no local, so
> without that its throwing edges had nothing to release. `python tools/holes.py` is down to **34
> sites**; item 17 still lists 5, all of them other items' panics that share `lower/expr.rs`. **Item
> 16's checker half is closed.** A `name:` or `...` call argument is mapped to the parameter it
> fills before anything is typed, and that mapping is the fact `mwl-ir` cannot re-derive — a name
> resolves against `MethodSig::param_names`, which no later pass holds — so it is recorded as
> `ExprInfo`'s new `ArgSlot` list on `ResolvedCall::arg_slots`. A named argument is checked against
> *its own* parameter, so an out-of-order list type-checks and a mis-named one is an ordinary
> `E0401` at the parameter the name reached. Five refusals came with it and PHP shares four: a
> positional argument after a `name:` or `...` is **E0488**, a parameter filled twice is **E0487**,
> a name reaching no fillable parameter is **E0486**, and a `...` with no variadic tail left to land
> in — the callee declares none, or a fixed parameter is still unfilled — is **E0489**, because how
> many entries an array holds is a run-time fact and a spread that could fill a fixed parameter
> would leave the call's arity uncheckable. **E0485 is the one rule PHP has no counterpart for**: a
> `Core` member's parameters are types in `mwl_stdlib::registry` and carry no names at all, ADR 0063
> R2's options bag being its by-name surface, so `param_names` is `None` there and a `name:` is told
> to write the bag instead. A spread's subject takes the tail's element type one level up as its
> *expectation*, `array<that>` — which is why **E0484 has no call-site twin**: that code is what a
> position with no `array<T>` expectation is left with, and a variadic parameter always has one, so
> `...$s` over a `string` is `expected array<T>, found string`, the same mistake named better. The
> arity check for a named list names the unfilled parameter rather than counting, and is suppressed
> behind any argument that reached no parameter at all, one mistake being one diagnostic. **Item 16
> is closed.** `lower_call_args` no longer walks the written list against `param_tys` position by
> position: it reads `ResolvedCall::arg_slots` off `ArgSig` and places each written argument at the
> ABI position of the parameter it fills, so **evaluation stays in written order while the callee's
> typed slots fill in declaration order** — the two disagree exactly when a `name:` reorders a call.
> A fixed parameter no argument filled takes its own default afterwards, which is the hole-filling
> an omitted trailing argument already got except that a hole may now sit in the middle of the list.
> A `...` argument is one `InstKind::ArraySpread` into the tail array rather than one entry of it,
> reusing item 17's instruction and its runtime; the written-out entries keep their single
> `ArrayNew`, because they are always a *prefix* — a positional argument cannot follow a `...` and a
> `name:` never reaches the variadic parameter — so a call with no spread emits exactly the
> instructions it emitted before. **The key rule is ADR 0007 § 5's, unchanged**: an integer-looking
> key renumbered under the tail's own append counter, every other preserved, which is byte-identical
> to PHP's own unpacking on every input PHP accepts, string keys included. The one divergence is ADR
> 0007 § 7 **row 12**, the § 7 table's second new row this goal: `f(...["k" => "v", "0" => "z"])` is
> a run-time fatal in PHP, which re-reads a string key as a `name:` and then refuses the integer one
> behind it, and is accepted here because MWL's variadic tail *is* the array and has no by-name
> surface for a later key to be out of order with. **One live bug went with it**, off the worklist
> because nothing panicked: a `string ...$parts` parameter bound its own *body* at `string` — the
> type each argument is checked against — while the caller has always passed the one array it
> collected them into, so no user-declared variadic method could read its tail at all (`foreach
> ($parts as string $p)` was `E0443: foreach cannot iterate a string`) and `mwl-ir` declared that
> ABI slot at the element's representation rather than `Ty::Array`. Both halves bind `array<T>` now,
> in `mwl_types::check`'s parameter loop and in `mwl_ir::lower::lower_method`'s. `python
> tools/holes.py` is down to **33 sites**, and item 16's last one is not its own: it is the
> `CallArgs::FirstClassCallable` arm, the `Class::method(...)` spelling that is `mwl-ir` gap 1.
> **Item 20 is closed, and `&$v` is a write-through rather than a copy-back.** A by-reference
> `foreach` drops the second reference to the array the by-value one takes — the one thing that
> reference buys, ADR 0007 § 5 separating on the first write so the cursor keeps walking the
> snapshot, is exactly what `&$v` must not do — and walks the subject variable's own `Env` binding
> instead, so the local stays the single owner it always was and the loop releases nothing after
> itself. Every rebinding of `$v` writes the entry it came from *at the point it is written*
> (`Lowering::write_through_element`, hooked into the three places that re-point a local's slot:
> `bind_local_value` for an assignment and every compound form, `write_back_holder` for a `&$v`
> argument's copy-back, and `write_back_array` for `$v[0] = e`), which is what makes a `break`, a
> `return` and a throw out of the body all leave standing what the body had already written — PHP's
> `&$v` is a true alias, and this is that alias one store later, without a copy-back to emit at four
> different edges. The array's binding is re-pointed by each write, `InstKind::ArraySet` consuming
> one reference and producing the holder's, so the subject takes a header phi like any reassigned
> local, and a nested `foreach ($grid as array<int> &$row)` re-points the outer entry at the row the
> inner loop separated. Two refusals came with it and PHP shares both: a subject that is not a plain
> variable is **E0490** — a temporary has nowhere to leave the write, and ADR 0053 § 1's cursor has
> no element storage at all — and a binding wider than the element type is **E0491**, an `array<T>`
> read being covariant where a write is not. Every row is byte-identical to PHP, including an entry
> the body appends mid-loop and a throw with a written entry live, and it is pinned that way in
> `tests/differential/lang/a-foreach-by-reference-matches-phps.mwlt`; `tools/leak-check.sh` is green
> over all three fixtures. `python tools/holes.py` is down to **32 sites**. **Item 19 is closed, and
> it split into two refusals rather than a lowering.** A `&$x` parameter is a contract between the
> two ends of one call — the site stages the cell, the callee's frame dies first, the site copies
> back — and the two declarations that break that ordering are refused where they are written. A
> **generator** declaring one is **E0492** (`mwl_types::check::check_generator_by_ref_params`):
> calling one runs none of the body, so the staged cell is gone before the first `advance()` while
> the parked frame would still be addressing it. A **closure** declaring one is **E0493**
> (`mwl_types::expr::calls::report_by_reference_parameter`): its type is `callable` and carries no
> parameter list, so no call site could know to stage anything, and ADR 0031 § 2's by-value capture
> lets it outlive every frame in scope where it was written. The decision paragraph is
> `docs/adr/README.md` § *Decisions taken at project start*, and neither is a lowering left
> unwritten — copying the value in would stop being a reference, which is the whole observable point
> of `&$x`. The third site went with them and needed no rule of its own: only `lower_method`'s
> parameter loop ever binds a `Ty::Ref`, so with E0492 standing no `&$x` binding can be live across
> a `yield`, and `lower_yield`'s assert is an internal-consistency check now rather than a hole.
> **Item 19 is closed outright, and its last piece was a lowering rather than a refusal**: a closure
> *capturing* an enclosing `&$x` parameter takes ADR 0031 § 2's by-value snapshot — one
> `InstKind::RefLoad` at the literal, at the pointee representation `Lowering::pointee_of`
> remembers, then the same retain every refcounted capture already takes, so the environment object
> owns its own reference and the closure is safe to outlive the call that staged the cell. Copying
> is what makes it safe, and it is also what PHP's arrow function does (capture by value at
> creation), so every row is byte-identical and pinned that way in
> `tests/differential/lang/a-closure-capturing-a-by-reference-parameter-snapshots-it.mwlt`;
> `tools/leak-check.sh` is green over a fixture that returns the closure and invokes it after the
> frame that staged the cell is gone. **An object literal writing one field name twice is E0494**,
> where it is written rather than a panic below it: a shape's fields are a set, and the two sides
> would not even agree on which write survives — `mwl_types` interns the shape reading the *first*
> of the pair, while the class `mwl-ir` synthesizes carries one slot per name and would keep the
> last. PHP's nearest neighbour is a duplicate *array* key, where the last write wins silently, and
> that reading is deliberately not carried over: an array is a map and a shape is a record.
> `mwl_types::expr::literals::check_object_literal` owns the rule and its reasoning, the duplicate's
> own initializer is still checked so its errors arrive in the same run, and ADR 0063 R2's options
> bag — written with the same braces — keeps its own `E0304` at the argument checker, so one
> spelling has two owners and neither reports twice. `python tools/holes.py` is down to **27 sites
> and 7 items**. **A property access through a `mixed` receiver is ADR 0036 § 4's name-keyed
> fetch**, read and write alike, rather than the panic that stood at `lower_property_access`'s
> catch-all arm: `mixed` is ADR 0007 § 2's one unchecked position, PHP accepts the read, and
> deferring every question it raises — is this an object, does that object carry this name — is what
> the position is for. What it could not reuse unchanged is the *untag*. Every other tagged receiver
> reaching a member arrives with a tag `mwl_types` already proved, so `Lowering::untag_receiver`
> compares nothing, and an unchecked untag over a `mixed` holding an `int` is a pointer the next
> instruction dereferences. `ReceiverProof::Erased` therefore emits no untag at all: the whole
> tagged value travels to `InstKind::SlotGet`/`SlotSet`, whose runtime helpers now take the receiver
> as one 16-byte `Value` **by address** and check its tag where they already check its name. That
> costs two stores on a path that was already one call, and priority 1 buys them outright. A
> receiver that turns out not to be an object is then a catchable throw in PHP's own wording —
> *"attempt to read property `name` on int"*. **Every receiver whose declared type can hold no
> object at all is `E0495` instead**, where it is written: a scalar, an `array<T>`, and a union
> naming no single class, which was the same panic by a second route the worklist item had not
> measured. That is ADR 0007 § 7 **row 13**, row 8's rule ("nothing makes an absent thing read as a
> zero value") at the one storage kind a *declared* type already answers before the program runs,
> and `mixed` is the exception that keeps PHP's timing. A nullable receiver keeps its own `E0459`
> and is never reported twice. `python tools/holes.py` is down to **24 sites**, still 7 items.
> **`instanceof` answers through an erased subject now, and both of its sides are checked.** A
> `mixed`, or a `?Box` no test narrowed, is the shape the operator exists for and was the one shape
> it could not lower: `mwl-ir` asserted a `Ty::Object` subject, so `$m instanceof Box` aborted the
> process rather than answering. Such a subject travels as a whole `Value` **by address** — the
> shape ADR 0036 § 4's name-keyed access already takes, `mwl_codegen::emit::materialize_receiver` —
> and `mwl_runtime::mwl_value_instanceof` reads its tag, answering `false` for every non-object
> exactly as PHP does. The proven `Ty::Object` path is untouched and pays nothing: the branch is in
> `emit_instanceof`, which already had a pointer in hand. **Every subject whose declared type can
> hold no object is `E0497` instead**, where it is written — a scalar, an `array<T>`, an enum, a
> union naming no class — which is ADR 0007 § 7 **row 14**, the same call ADR 0090 makes for two
> statically disjoint types under `==`, and `mixed`/`object`/a shape/any union holding a class all
> keep the run-time test. **The right-hand side is `E0496` unless it names a class this program
> declares**: the dynamic `$x instanceof $name` form (ADR 0007 § 2 has no computed class names, the
> line `$$var` and `eval` are already on), an enum (ADR 0010 makes a case a named integer, so no
> value is ever an instance of one) and a `Core` class (a registry signature, which
> `build_class_layouts` lays no descriptor out for); a name resolving to nothing at all takes the
> ordinary `E0303`, exactly as `new Undeclared()` reports it, one mistake being one code wherever
> the name is written. **One live leak went with it**, off the worklist because nothing panicked:
> `lower_instanceof` never released a subject nothing names, so `(new Dog()) instanceof Animal` lost
> the object every run — the test only *reads* its subject and answers a `Ty::Bool`, so a fresh one
> is released once it has answered, the same rule `lower_clone_expr` already applies to its own
> operand. `tools/leak-check.sh` is green over fixtures exercising both paths with a refcounted
> `mixed` live. `python tools/holes.py` is at **25 sites**, still 7 items — it read **26** at this
> session's head, so the 24 quoted above predates a change in the tool's own inventory rather than
> this slice. **`true` and `false` are `bool`'s literal types now**, which is what closed
> `closed_literal_set`'s own panic. ADR 0007 § 3 has always had the two atoms and ADR 0047 § 1 calls
> its string and int literals their generalisation, but nothing ever *placed* a `true` expression at
> one: `true $t = true;` was `E0401` and `$m as true` aborted in `erase_checked_ty` naming a
> representation it had no arm for. They are placed
> (`mwl_types::expr::literals::infer_bool_literal`), widened (`TypeInterner::literal_base`) and
> erased (`Ty::Bool`) exactly as § 1's two are, so `var $b = true;` still infers `bool` where the
> position names no singleton, and `bool $b = $x as true;` is an ordinary assignment. `$m as true`
> is `as bool` plus § 5's membership test, and for a `mixed` operand that test is
> `Helper::Identical` against the runtime tag — so a `mixed` holding `1` is a miss where a
> statically typed `int` `1` is a hit, which is the split the string and int sets already had rather
> than a new one. What is left after a proven tag is one unchecked `InstKind::Untag` and not ADR
> 0035's truthy table, which would answer `true` for the very value the test just refused. **The
> panic itself is gone rather than diagnosed**: the `closed` predicate and the member map walked the
> *same* atom list twice, so the catch-all was an internal-consistency check between a list and
> itself, and one `collect::<Option<_>>` pass makes "this target is not a closed set" one answer in
> one place. **A second live abort went with it**, found by measuring the item's own spellings
> rather than by the worklist: a set whose members share no representation (`$s as 1|"a"`) erases to
> a tagged target, and `convert` had no `(_, Ty::Tagged)` arm at all, so it died on `Str as Tagged`.
> That row is one `InstKind::Tag` — the instruction `Lowering::coerce` already emits where a
> *declaration* is the wider side — plus the single retain a borrowed operand owes, because a
> conversion's result is a fresh value its consumer owns; `$s as mixed` is the same row written
> plainly. A failed membership test names a `bool` operand as `true`/`false` now
> (`mwl_runtime::rendered_operand`) rather than as "a `Bool` value". `python tools/holes.py` is down
> to **24 sites and 6 items**. **A `mixed` in a condition, and `$m as bool`, are ADR 0035 § 2's own
> last table row now** rather than a process abort: `truthy_convert`'s arms are that table one
> representation at a time, and the row for a value whose type the compiler erased is one
> `Helper::ValueTruthy` reading the tag. The runtime side already existed —
> `mwl_runtime::value_truthy` was written for native `Core` code holding a `Value` a closure
> returned — so the whole slice was giving compiled code a way to reach a row that was already
> correct, and its doc comment now names both callers instead of disclaiming the second. **No untag
> is emitted**, for `ReceiverProof::Erased`'s reason: an unchecked one over an `int` payload is a
> pointer the next instruction dereferences, so the whole tagged value travels to the helper. **One
> divergence is now visible and is not this slice's to fix**: an enum case tagged into a `mixed`
> reads as its backing integer, so `mixed $m = Rank::Bronze;` (backed by `0`) is falsy where ADR
> 0035 § 4 makes every statically-typed case truthy. That is `mwl_codegen::ty::tag_of`'s decision —
> ADR 0010 § 6 reserves an enum tag and nothing writes one — and its comment's premise ("deciding an
> enum's tag would be deciding half the same question twice") has expired now that `mixed` *is*
> `Ty::Tagged`; the backlog carries it, and `Core\Reflect::typeOf` over the same value is the same
> gap by a second route. **`bytes` in a condition lowers too**, which was the same panic one arm
> away and a live abort for `bytes $b = "abc" as bytes; if ($b) { … }`: it is falsy iff **empty**,
> deliberately dropping the one-octet `"0"` case a `string` has only because PHP reads a string as a
> possible number, and ADR 0009 makes `bytes` the type that never converts to one. ADR 0035 § 2's
> table carries the row and the paragraph under it carries the reasoning;
> `mwl_runtime::value_truthy`'s `Tag::Bytes` arm had already taken the same reading, so the two
> spellings of one buffer agree by construction. `truthy_convert` panics for `Ty::Void` alone now,
> which ADR 0007 keeps out of value position. **`as ?T` over a literal or enum target runs now**,
> which is ADR 0066 § 3 row 2's "non-throwing twin" reaching the one target family it never had.
> What blocked it was not the chain but the *target*: `lower_conversion`'s nullable arm read it off
> the `?T`'s **inner** AST node, and `mwl_types::lower::lower_type` records a checked type once, at
> its own entry point, so a nested `Type` node has no entry at all and `lower_decl_type` fell back
> to the AST — where a name-shaped atom is a class whether or not it names an enum. `$m as ?Mode`
> therefore died on `Tagged as ?Object`, which reads as a missing conversion row rather than as a
> missing table entry. The target is the whole annotation's checked type minus `null` instead (`?T`
> interns as `T|null`, `Lowering::nullable_target_atoms`), and the accepted set is built from those
> atoms rather than from a `TypeId` — this crate holds the interner by shared reference and can
> intern none. **The form is built as the twin, not as a redirect of the throwing one.**
> `landing_block` ends in `Terminator::Catch`/`Propagate` with a pending `Throwable`, so re-pointing
> the checked lowering's error edge at a null-producing block would have to discard that object and
> account for its reference, which is `lower/exception.rs` plumbing for a form that needs no
> exception to exist. Two substitutions on the non-nullable arm buy the same thing:
> `lower_literal_membership` takes a `miss: Option<BlockId>` — `None` is today's
> `Helper::LiteralMismatch` throw, `Some` a jump — and the base conversion runs through
> `convert_or_null` wherever its row can fail (`conversion_can_fail` is `Lowering::convert`'s own
> free/total/widening row list read as a question). **A `Ty::Tagged` operand into a literal set
> needs no conversion at all**: the result of `as ?T` is a `Ty::Tagged` value and on a hit the
> operand already *is* one, which is also what keeps a `mixed` holding `1` out of a set naming `"1"`
> — the coercion ADR 0047 § 4 refuses. **A fallible base keeps its tagged answer and compares
> through `Helper::Identical`** rather than untagging first, because an `Untag` of the `null` a
> failed conversion produced reads a zero payload and an enum with a case backed by `0` would then
> *hit* on a conversion that failed; a `null` matches no member, so the miss edge answers it with no
> test of its own. Ownership is one rule for both shapes — the answer is an owned `Ty::Tagged` value
> the hit path hands the consumer and the miss path releases, a release being a runtime no-op for
> every tag that owns nothing — and `tools/leak-check.sh` is green over a fixture covering both
> edges with a fresh refcounted operand live. `python tools/holes.py` still reads **24 sites, 6
> items**: `convert_or_null`'s catch-all is unchanged, `$b as ?string` and `$m as ?array<T>` still
> reaching it, and this slice widened neither. **M4S Part I is the floor, not the frontier**:
> conformance is at 603 of the goal's new 750 and differential at 167 of 165, `python tools/gaps.py`
> still ranks the thin classes, and a `Core` depth slice is a legitimate slice when a group is
> blocked — never a reason to leave a language item unfinished. `docs/spec/02-php-migration.md` is
> 31% classified (`python tools/check-migration.py`).
>
> **Blocking:** Nothing external, and nothing waiting on a decision — every design call this loop
> reaches is pre-authorized in `docs/agent/loop-goal.md` § *Standing decisions*, including the ones
> this goal added: an abandoned generator's `finally` runs through a resume-to-unwind entry point
> and **not** a destructor (so ADR 0028 § 2 stands, with one sentence folded into it), `int / int`
> widens at the binding rather than at the operator, a nullsafe assignment target and an element
> write through a hooked property are both compile errors because PHP refuses both, `&value` as an
> array-literal element does not exist (ADR 0031 § 2 left it no owner), and a `Core` class
> stringifies exactly where the spec gives it a `toString`. The representation questions the
> previous goal settled are settled still — `?T`/`mixed` (`mwl_ir::Ty::Tagged`), object identity
> (`mwl_runtime::identity`), element-covariant array reads (`mwl_types::expr::is_assignable`), a
> `catch` binding scoped to its own handler — and picking every dependency but the two the user
> named remains pre-authorized. Stages 1 and 8's floor passes whole on both legs with no
> intermittently red test left. **The one thing this goal forbids outright** is growing the
> allowlist in `every_refusal_is_a_diagnostic_or_decided` to make a run go green: every entry on it
> is a bullet in § *Standing decisions*, and a session that needs a new one takes the decision
> there, in the same session, with the reason.

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
| [M4B](plan/m4b.md) | Minimal `mwl-lsp` and the VS Code extension (~3 weeks) | ~1.5 |
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
