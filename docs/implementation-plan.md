# MWL — Modern Web Lang: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance for you, not a
     check: nothing verifies it, and no session should ever be spent trimming to a number. History
     lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md's "Writing docs
     here" section. -->

> **Status:** 2026-08-26. **M3 is done, spec §§ 1-12 is registered whole, and Stage 0's catch-up
> list is closed.** `crates/mwl-stdlib/tests/spec-members-outstanding.txt` holds no keys: the
> ratchet `spec_registry_coverage.rs` reads is empty, which is this project's own definition of Part
> I being *registered*. All seven Stage 3 fixtures produce their frozen output,
> `examples/collect.mwl` having landed with `Core\Out::capture` — and with two bugs a parse error
> had been masking in it since it was written (`Core\Uuid::isValid`, which spec § 11 says does not
> exist, and an `Arr::first` result indexed through the nullable-array hole the playbook names). All
> seventeen original Stage 0 items and its bench-review batch 18-22 are done; **the frontier is
> Stage 4's two counts**, conformance 486 of 600 and differential 141 of 150. **Nothing below them
> is red, and `verify.py` is deterministic again**: the uncaught-throw test read a freed exception
> object because the compiled `Unit` owns the class descriptors and the codegen harness dropped it
> while the `Ctx` still held one — `mwl_codegen::Unit::install_in`'s own doc comment is now the home
> of that obligation, and both embedders call it. **The depth pass is section by section, and every
> section has now taken its turn** — `encoding`, `hash`, `csv`, `validate`, `out`, `heap`, `uuid`,
> `path`, `json`, `random`, `bytes`, § 9's two collections, `regex`, `math`, `uri`, `time`, `str`
> and finally `arr`. What each pass pins is its own case's `--TEST--` line, and the four case shapes
> they reuse are named in `Open now`; neither is restated here. **A depth slice lands its claim as a
> new case file**, because the gate counts files and deepening one in place moves nothing. The goal
> is unchanged: **M4S Part I in full plus the M4 surface it cannot be written without**. Every
> representation that blocked a section is built and recorded in the crate that owns it —
> `mixed`/`?T`/every union is `mwl_ir::Ty::Tagged`, strict identity is `mwl_runtime::identity`,
> `decimal` is `mwl_runtime::decimal`, a `Core`-owned instance is an ordinary MWL object
> (`mwl_stdlib::instance`), a variadic tail is one `array<T>` argument, and a sink's carrier is a
> `Core` instance whose one slot `mwl_runtime::value_to_string` renders (ADR 0088 § 5). **Every M4
> control-flow statement lowers but `do`/`while`**, and ADR 0070's duration literal lexes, types and
> runs. **ADR 0087's lexer half is built** — `mwl_syntax::bidi` is the one predicate, `E0008` at the
> lexer, and a substitution at both sinks at M7/M8. **ADR 0090 §§ 1 and 2 are built**: `==`/`!=` are
> the only equality spellings, `===`/`!==` are `E0232`, and two statically disjoint operands are
> `E0466`; § 3's string, array and object rows are one runtime helper each and still owed. **ADR
> 0069's `+`/`+=` refusal is built**, `E0467` naming `Core\Arr::underlay`. **ADR 0071's first
> compiler-recognized attribute is built** — `#[Json\Derive]`, matched nominally in
> `mwl_types::derive` — and `ParseError` carries § 10's `issues`. **Cranelift's stack probe is
> emitted inline**, the default `outline` strategy having called a `__cranelift_probestack` symbol
> this JIT never registers. Dependencies: `regex` + `fancy-regex` and `jiff` are named by the user;
> the rest the loop picks under ADR 0051 § 4.
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
> **On disk:** the workspace, CI on three platforms, lint/deny/fmt/notice policy, `mwl-diagnostics`,
> `mwl-syntax` (+ `duration`, ADR 0070's one grammar, and `bidi`, ADR 0087's one predicate),
> `mwl-hir`, `mwl-types` (+ `layout`, `core_lib`, `error_lib`, `iter_lib`, `generics`,
> `conformance`, `defaults`, `derive`), `mwl-ir`, `mwl-runtime` (+ `alloc`, MWL's own
> `#[global_allocator]` in an optimized build, `object`, `array`, `throwable`, `closure`,
> `identity`, `decimal`, and `ctx`'s capture stack and carrier roster), `mwl-stdlib` (`Arr` × 36,
> `Str` × 39, `Math` × 38, `format`, `granularity`, `ordering`, `cldr`, `instance`, `issue`,
> `Order`, `NormalForm`, `RoundMode`, `Unit` and `Weekday`, `Regex` × 6 plus `Regex\Match` × 4 over
> `regex`/`fancy-regex`, `Time` × 7 plus `Time\Instant` × 9, `Time\DateTime` × 14, `Time\Duration` ×
> 19 and `Time\Zone` × 4 (+ `UTC`) over `jiff`, `Json` × 4 over `serde_json`, `Path` × 9 (+
> `SEPARATOR`) over nothing at all, `Random` × 6 over `rand`, `Uuid` × 4 (+ `toString`) over `uuid`,
> `Uri` × 4 over nothing at all, `Encoding` × 2 over nothing at all, `Hash` × 4 plus `Hash\Stream` ×
> 2 over the RustCrypto family, `crc32fast` and `subtle`, `Csv` × 2 over `csv-core`, `Out` × 1
> answering the `Cli\Text` carrier, `ObjectMap` × 9, `ObjectSet` × 9 and `Heap` × 5 over
> `identity_store`, all three iterable through `cursor`, and the conformance-coverage gate),
> `mwl-codegen`, `mwl-cli` (`ast`, `check`, `run`, `test`, `info`), `mwl-test` (+ `case`, `expect`,
> `run`), `tests/conformance` × 486 (in `array`, `class`, `core`, `enum`, `error`, `iter`, `lang`
> and `reject`) and `tests/differential` × 141, `fuzz/`, `tools/`, `benches/abi-probe`.
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
> **Open now:** **Stage 4's two counts are the frontier: conformance is 486 of the 600 the gate
> requires and differential is 141 of 150.** Both named guards
> (`every_part_one_member_has_a_conformance_case`, `every_part_one_spec_member_is_registered`) pass,
> so the gap is behavioural *depth* per member rather than an unregistered or uncovered one, and
> every section has now had its pass — a new case therefore reaches for one of the four shapes in
> conventions.md rather than for a section, and lands its claim as its own file, because the gate
> counts files (playbook, *Writing a test case*). **`python tools/gaps.py` is the worklist and no
> session should re-derive it**: `--differential` names every member whose spec **Replaces** column
> gives it a PHP twin and which no oracle case calls — 19 left — and `--errors` every `Fault::` site
> in `mwl-stdlib` whose message no case asserts. **Differential is the half that comes first** — the
> smaller of the two gaps, and an `--ORACLE--` case has no frozen output to derive at all, because
> PHP computes it. Over `Core\Arr` the twin's key rule decides which kind of case it is: PHP
> renumbers a result's integer keys and keeps its string ones, ADR 0069 § 3 refuses that, so a
> member matches its twin over a *list* and diverges over a map, and both shapes exist now for the
> window, the ends, the padding pair and `reverse`. **A member answering a value rather than an
> array has no key rule to diverge over at all**, which is why `min`/`max` and `reduce` match PHP
> over a map as readily as over a list; what parts them from their twins is PHP's loose comparison —
> `min([0, "a"])` is `0`, and two numeral strings compare numerically — and `array_reduce`'s
> callback taking exactly two arguments where MWL's takes the key third. **Four other twins sit
> outside the key rule**: `array_unique` renumbers nothing at all, so the divergence there is
> `SORT_STRING`'s comparison by *spelling*; `array_count_values` names a bucket through the same key
> normalization `countBy` uses, so that pair parts only where PHP warns-and-skips a value `countBy`
> refuses; `ksort` reads a numeral *key* as a number where ADR 0007 § 5 makes every stored key a
> `string` compared bytewise, so `sortByKey` parts from it over a numeral or mixed-key subject and a
> `comparator` is the way back; and `array_fill` takes a start index `fill` drops, so every non-zero
> start is `Core\Arr::fillKeys` over the keys the caller wanted. **`Core\Math` is down to the `gmp`
> pair alone, and `Core\Str`'s 9 are now half of the whole remainder.** `Core\Math`'s settled pairs
> say what shape the rest take: nothing there has a key rule, so a member either agrees with its
> twin outright or parts over a *tie*, a *conversion*, a *guard* or a *repair*. **PHP's two-argument
> `min` answers its second argument on a tie and its `max` answers its first**, where
> `mwl_stdlib::math::pick` answers the first to both — visible wherever two equal values are
> distinguishable, `min(1000000000000000000, 1.0e18)` being the sharpest — and `f64::total_cmp`
> separates `-0.0` from `0.0` where PHP's `<` calls them equal. **A `CoreTy::Var("T")` signature
> binds `T` to the first argument**, so a crossed pair reaches the runtime refusal only when the
> union is declared on the bindings; written as two literals `Core\Math::min(0, "a")` is `E0401`,
> and the same holds of `Core\Math::mod(7, 2)`, whose two `float` parameters do not widen an `int`
> the way `fmod` does — the integer remainder is the `%` operator. **`Core\Math::mod` throws on a
> zero divisor where `fmod` answers `NAN`**, spec § 3 making a division by zero a throw wherever it
> appears, while the IEEE *domain* rows — an infinite dividend, either operand a `NAN` — stay at
> IEEE's answer and agree; `intDiv` agrees with `intdiv` on every row including both refusals. **The
> transcendental pairs are closed and they agree with their twins outright**: `sqrt`, `exp` and
> `log` leave the *argument's* domain unguarded, so a negative root and a negative logarithm are
> `NaN`, a zero logarithm is `-INF`, `exp` overflows to `INF` and underflows to the smallest
> subnormal, and every one of those renders byte-identically on both sides; `hypot` and `atan2`
> agree on the four signed-zero quadrants, on the infinite ones, and on the magnitude where `sqrt($x
> * $x + $y * $y)` overflows and `hypot` does not. **The one guard this added is `log`'s *base***,
> which PHP has and MWL did not: a base not greater than zero is a `Fault::thrown` where PHP raises
> a `ValueError`, and base `1.0` is `NAN` on both sides rather than the infinity `ln($n) / ln(1.0)`
> would answer. **The base pair and the two predicates are closed too, and each parts from its twins
> over a repair rather than over arithmetic.** `toBase`/`fromBase` agree with
> `decbin`/`dechex`/`decoct`/`base_convert` and their from-halves on every non-negative number
> written in digits the base holds — the same lowercase alphabet out, either case in, and the same
> `ValueError`-versus-`Fault` refusal outside base 2 to 36 — and part at the four inputs PHP has no
> failure mode for: a negative `$n` is `-ff` where `dechex` writes the two's complement
> `ffffffffffffff01` and `base_convert` drops the sign as a character it has no digit for, a digit
> past the base is a throw where `hexdec("beefy")` is `48879`, an empty string is a throw where
> `bindec("")` is `0`, and an answer past `int` is a throw where `hexdec("ffffffffffffffff")` widens
> to a `float`. `isNan` and `isFinite` agree with `is_nan` and `is_finite` outright, and PHP's third
> predicate is the two of them folded, so `is_infinite` is neither — asserted as a *partition* over
> a table rather than row by row. **`toRadians` and `toDegrees` now compute PHP's own expression
> rather than the accurate one**, and that is the only place in `Core\Math` where a spelling was
> changed to match a twin: `($degrees / 180.0) * PI` replaces `f64::to_radians`'s multiply by the
> correctly rounded `PI / 180.0`, and `($radians / PI) * 180.0` replaces `f64::to_degrees`. The std
> forms are the more accurate — over the 3,600 tenths of a degree in a turn they are closer to the
> true value 851 times against 118 — and the difference is at most one ulp, invisible at both
> languages' precision-14 rendering. It is visible through `==`, which is exact over `float` (ADR
> 0090): under PHP's spelling a whole-degree round trip lands back on its angle for 19 of 22 sampled
> angles and under the std one for 13, so the ulp is what a ported program comparing a round trip
> actually sees. AGENTS.md's priority 2 — PHP-compatible *observable* behaviour — is what decides
> it, and the accuracy spent is stated in `mwl_stdlib::math`'s own doc comments at both members.
> **`Core\Math::gcd` and `::lcm` have no callable twin on either leg**: neither the Windows `php`
> nor WSL's has `gmp`, so `gmp_gcd`/`gmp_lcm` are undefined functions and an oracle case for those
> two has to compute its expectation with an explicit Euclidean loop in PHP or be left out of the
> count. **MWL's `float` rendering is PHP's**, precision 14 with trailing zeros trimmed —
> `sqrt(2.0)` prints `1.4142135623731`, `exp(-745.0)` prints `4.9406564584125E-324` and `0.1 + 0.2`
> prints `0.3` on both sides — so a `Core\Math` oracle case may echo a float directly and needs no
> formatting, but never a `NAN`, which PHP 8.4 and later warn about coercing to a string.
> **`Core\Str`'s remaining twins part from PHP over a *unit* before they part over anything else**,
> and `slice`/`replaceRange` is the worked pair: `mwl_stdlib::granularity::DEFAULT` is
> `Unit::Grapheme`, so a member's `int $offset` and `?int $length` count clusters where `substr`
> counts bytes and `mb_substr` counts code points. Inside ASCII with no carriage return all three
> units coincide, which is why the whole offset/length sign table — 10 offsets × 9 lengths, both
> members, plus the `null` length, the empty subject and the deletion spelling — agrees outright as
> one `--ORACLE--` file, and the three-way split is a second `--ORACLE-DIVERGES--` file with a
> frozen `--EXPECT--`, because the `php` on the Windows `PATH` has no `mbstring` and `mb_substr` is
> not callable at all. Both members read one `window` helper, and putting a window's own slice back
> into it reproduces the subject on all 90 ASCII cells and all 72 multibyte ones — **PHP's pair
> holds that identity too**, which the `replaceRange` doc comment used to deny and no longer does.
> **A frozen `--EXPECT--` must never render a decomposed cluster**: `e`+U+0301 and U+00E9 are
> indistinguishable in an editor and are different strings, so a divergence case echoes
> `Core\Encoding::toHex($s as bytes)` for those cells instead. **`Core\Str::before`/`::after` and
> `::compare` are closed, and each parts from a *twin* rather than from a rule.** Both cut members
> exclude the needle where `strstr` keeps it, so the port of a program that wanted PHP's shape is
> `$needle . Str::after(...)`, and an absent needle is `null` where `strstr`, `stristr` and
> `strrchr` all answer `false` — a difference in how "no answer" is spelled, folded to one sentinel
> on both sides rather than a divergence. **`{last: true}` is `strrchr` only at a one-character
> needle**: `strrchr` reads the needle's first character and nothing else, so `strrchr("a::b::c",
> "::")` is `":c"` where the member answers `"c"`, and `strrchr("banana", "an")` answers `"a"` for a
> needle that never occurs at all; at the empty needle `strrchr` fails outright where `strrpos`
> answers the subject's length, which is the member's answer too. The honest twin past one character
> is `strrpos` plus a `substr`, and both halves rejoin through the needle to the subject on every
> cell where it occurs — counted, on both sides, under each occurrence rule. **`stristr` has no
> twin**, because the pair's one option is `{last}`; the port is `Core\Str::indexOf($s, $n,
> {caseInsensitive: true})` plus a `slice`, needle-inclusive. **`compare` folds four twins into two
> independent options and the fold is exact over ASCII** — all 144 ordered pairs of a table, in all
> four corners — but only after a *sign* normalization, because PHP's four disagree with each other
> on magnitude: 8.2 narrowed `strcmp`, `strnatcmp`/`strnatcasecmp` were always -1/0/1, and
> `strcasecmp` on 8.5.9 still returns the byte difference, in 84 of those 144 cells. `compare` is
> always one of three literals, and it is the only ordering two strings have at all, since `<` over
> two `string`s does not lower. **Unbuilt in the library**, none of it a registration gap:
> `Core\Json::decodeAs<T>`'s decoder, which ADR 0071 leaves reading a scalar-fielded class only — no
> enum, `decimal`, `Instant`, `array` or nested-class field, and no optional key from a parameter
> default (`mwl_stdlib::json` gap 2); ADR 0088's qualifier classification, missing from every
> `mwl-stdlib` member row, which is why `Core\Str::format`'s template is not yet the sink that ADR
> makes it, why neither the fail-closed default for an unclassified `string`/`bytes` parameter nor
> the test refusing an unclassified member exists, and why `Core\Hash::hmac`'s `secret bytes $key`
> is a plain `CoreTy::Bytes` (`mwl_stdlib::hash`'s module doc); and ADR 0086 § 1's substitution
> table, which is what would make the terminal sink neutralize a control byte and `Cli\Text::plain`
> a constructor that cannot produce an injected escape (M8, `crates/mwl-stdlib/src/cli.rs` gap 1).
> **Decided and unbuilt, but not catch-up** — ADRs 0091 (the `development`/`production` run mode),
> 0092 (one diagnostic record rendered three ways by the sink in force), 0093 (`mwl service`), 0097
> (the server's scope and its `[server]` block) and 0100 § 3 (a file opening `#!` starts in code
> mode with no tag — one `mwl-syntax` branch at offset 0, `E0009` reserved in the registry, no
> parser or runtime change). None invalidates built behaviour or a written fixture; their work is
> M1, M4, M6, M7, M8 and M10. **Open beside the library** — a property's declared default runs and
> is type-checked (`E0472`), limited to a literal or `[]`; `do`/`while` is the one M4 control-flow
> statement that does not lower; a closure cannot be called through the variable holding it and
> `Class::method(...)` panics `mwl-ir` outright, so a case sweeps a table with `foreach` and reaches
> a helper through a `public static function` declared in the same file (`mwl-ir` gap 1); a `?bool`
> cannot be tested for truth at all, so a member answering one has no `yn` rendering; `bool as int`
> does not lower and `bool as string` renders `false` as nothing at all; an abandoned generator
> never runs the `finally` it is suspended inside (`mwl-ir` gap 18, a deliberate PHP divergence);
> ADR 0043's `by`-delegation is off path; and `docs/spec/02-php-migration.md` is 31% classified,
> reported by `python tools/check-migration.py`. **What has landed is not restated here** — `git
> log` holds the session-by-session history and each crate's own module doc holds its per-file gaps.
>
> **Blocking:** Nothing external, and nothing waiting on a decision — every design call this loop
> reaches is pre-authorized in `docs/agent/loop-goal.md` § *Standing decisions*, including the
> `?T`/`mixed` representation (settled, `mwl_ir::Ty::Tagged`), re-scoping a `catch` binding to its
> own handler block, widening `array<T>` to element-covariant-on-read (settled,
> `mwl_types::expr::is_assignable`), object identity (settled, `mwl_runtime::identity`), the sink's
> carrier being a `Core` instance rather than a `string` (settled under ADR 0088 § 5), and picking
> every dependency but the two the user named. Stages 0, 1, 2 and **3** pass whole on both legs,
> with no intermittently red test left: the uncaught-throw use-after-free is fixed and
> `mwl_codegen::Unit::install_in` owns the obligation it broke. What Stage 4 needs is not a
> decision: conformance stands at 486 of 600 and differential at 141 of 150, and the remainder is
> written a section at a time.

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
