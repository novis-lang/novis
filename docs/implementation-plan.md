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
> Stage 4's two counts**, conformance 496 of 600 and differential 151 of 150. **Nothing below them
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
> `run`), `tests/conformance` × 496 (in `array`, `class`, `core`, `enum`, `error`, `iter`, `lang`
> and `reject`) and `tests/differential` × 151, `fuzz/`, `tools/`, `benches/abi-probe`.
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
> **Open now:** **Stage 4's differential gate is met — 151 against the 150 it requires — so
> conformance is the only frontier left, at 502 of the 600.** Both named guards
> (`every_part_one_member_has_a_conformance_case`, `every_part_one_spec_member_is_registered`) pass,
> so the gap is behavioural *depth* per member rather than an unregistered or uncovered one, and
> every section has now had its pass — a new case therefore reaches for one of the four shapes in
> conventions.md rather than for a section, and lands its claim as its own file, because the gate
> counts files (playbook, *Writing a test case*). **`python tools/gaps.py` is the worklist and no
> session should re-derive it**: `--differential` names every member whose spec **Replaces** column
> gives it a PHP twin and which no oracle case calls — 8 left — and `--errors` every `Fault::` site
> in `mwl-stdlib` whose message no case asserts, 61 of them — of which **57 are `Fault::fatal`,
> unreachable by any handler and so by any case, leaving 4 a case can catch**. A site leaves the
> list when the literal run of its message *before the first format hole* appears anywhere in either
> suite, so a case echoing `Core\Time\DateTime::format():` also silences every message spelled
> `Core\Time\DateTime::{member}` — which is how closing seven sites once took twelve rows off the
> list at once. Three sites are hidden that way today and none is owed a case: `time.rs:1968`'s
> unknown zone, where `datetime_built` re-derives every stored zone id from the resolved zone so no
> program can reach it and the invariant is what a case asserts in its place, and `json.rs:767` and
> `json.rs:869`, both `fatal` and both hidden by the `Core\Json::decodeAs(): ` stem the codec case
> asserts. The list is a worklist rather than a ledger, and the playbook's *Tooling* section owns
> what to do when the drop is larger than the number of sites a session asserted. Over `Core\Arr`
> the twin's key rule decides which kind of case it is: PHP renumbers a result's integer keys and
> keeps its string ones, ADR 0069 § 3 refuses that, so a member matches its twin over a *list* and
> diverges over a map, and both shapes exist now for the window, the ends, the padding pair and
> `reverse`. **A member answering a value rather than an array has no key rule to diverge over at
> all**, which is why `min`/`max` and `reduce` match PHP over a map as readily as over a list; what
> parts them from their twins is PHP's loose comparison — `min([0, "a"])` is `0`, and two numeral
> strings compare numerically — and `array_reduce`'s callback taking exactly two arguments where
> MWL's takes the key third. **Four other twins sit outside the key rule**: `array_unique` renumbers
> nothing at all, so the divergence there is `SORT_STRING`'s comparison by *spelling*;
> `array_count_values` names a bucket through the same key normalization `countBy` uses, so that
> pair parts only where PHP warns-and-skips a value `countBy` refuses; `ksort` reads a numeral *key*
> as a number where ADR 0007 § 5 makes every stored key a `string` compared bytewise, so `sortByKey`
> parts from it over a numeral or mixed-key subject and a `comparator` is the way back; and
> `array_fill` takes a start index `fill` drops, so every non-zero start is `Core\Arr::fillKeys`
> over the keys the caller wanted. **`Core\Str` and `Core\Regex` are closed, `Core\Math` is down to
> the `gmp` pair alone, and `Core\Path`'s three are now the largest block of the 8.** `Core\Math`'s
> settled pairs say what shape the rest take: nothing there has a key rule, so a member either
> agrees with its twin outright or parts over a *tie*, a *conversion*, a *guard* or a *repair*.
> **PHP's two-argument `min` answers its second argument on a tie and its `max` answers its first**,
> where `mwl_stdlib::math::pick` answers the first to both — visible wherever two equal values are
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
> **`Core\Str`'s twins part from PHP over a *unit* before they part over anything else**, and
> `slice`/`replaceRange` is the worked pair: `mwl_stdlib::granularity::DEFAULT` is `Unit::Grapheme`,
> so a member's `int $offset` and `?int $length` count clusters where `substr` counts bytes and
> `mb_substr` counts code points. Inside ASCII with no carriage return all three units coincide,
> which is why the whole offset/length sign table — 10 offsets × 9 lengths, both members, plus the
> `null` length, the empty subject and the deletion spelling — agrees outright as one `--ORACLE--`
> file, and the three-way split is a second `--ORACLE-DIVERGES--` file with a frozen `--EXPECT--`,
> because the `php` on the Windows `PATH` has no `mbstring` and `mb_substr` is not callable at all.
> Both members read one `window` helper, and putting a window's own slice back into it reproduces
> the subject on all 90 ASCII cells and all 72 multibyte ones — **PHP's pair holds that identity
> too**, which the `replaceRange` doc comment used to deny and no longer does. **A frozen
> `--EXPECT--` must never render a decomposed cluster**: `e`+U+0301 and U+00E9 are indistinguishable
> in an editor and are different strings, so a divergence case echoes `Core\Encoding::toHex($s as
> bytes)` for those cells instead. **`Core\Str::before`/`::after` and `::compare` are closed, and
> each parts from a *twin* rather than from a rule.** Both cut members exclude the needle where
> `strstr` keeps it, so the port of a program that wanted PHP's shape is `$needle .
> Str::after(...)`, and an absent needle is `null` where `strstr`, `stristr` and `strrchr` all
> answer `false` — a difference in how "no answer" is spelled, folded to one sentinel on both sides
> rather than a divergence. **`{last: true}` is `strrchr` only at a one-character needle**:
> `strrchr` reads the needle's first character and nothing else, so `strrchr("a::b::c", "::")` is
> `":c"` where the member answers `"c"`, and `strrchr("banana", "an")` answers `"a"` for a needle
> that never occurs at all; at the empty needle `strrchr` fails outright where `strrpos` answers the
> subject's length, which is the member's answer too. The honest twin past one character is
> `strrpos` plus a `substr`, and both halves rejoin through the needle to the subject on every cell
> where it occurs — counted, on both sides, under each occurrence rule. **`stristr` has no twin**,
> because the pair's one option is `{last}`; the port is `Core\Str::indexOf($s, $n,
> {caseInsensitive: true})` plus a `slice`, needle-inclusive. **`compare` folds four twins into two
> independent options and the fold is exact over ASCII** — all 144 ordered pairs of a table, in all
> four corners — but only after a *sign* normalization, because PHP's four disagree with each other
> on magnitude: 8.2 narrowed `strcmp`, `strnatcmp`/`strnatcasecmp` were always -1/0/1, and
> `strcasecmp` on 8.5.9 still returns the byte difference, in 84 of those 144 cells. `compare` is
> always one of three literals, and it is the only ordering two strings have at all, since `<` over
> two `string`s does not lower. **`reverse` and `chunk` are closed, and on those two the unit is the
> *whole* disagreement**, since neither takes an offset: `strrev` and `str_split` walk bytes, so
> their answer for any subject outside ASCII is not well-formed UTF-8 and therefore not a value MWL
> can hold at all — which is why neither member is offered a compatibility spelling back to its
> twin, there being nothing there a port could want. Over ASCII both agree with their twins
> outright, and what each agreeing file pins is the *properties* rather than the rows: `reverse` is
> its own involution over a table, preserves length, is the identity on exactly the palindromes, and
> turns a concatenation inside out on all 100 ordered pairs — PHP holding every one of those
> byte-wise. `chunk` folds three twins, and `chunk_split` is not a fourth member but
> `Core\Str::join(Core\Str::chunk($s, $n), $end) . $end`, which reproduces it on all 30 cells
> *including the empty subject*, where `chunk` gives no chunks and `chunk_split("")` is still the
> separator alone; a zero size is refused on both sides, PHP's `ValueError` against a catchable
> `Fault::thrown`. **The counted claim that survives a unit change is not the discriminating one**:
> pieces rejoined with nothing between them reproduce the subject under all three units, so what
> parts the members from their twins is the piece *count* — `Core\Str::chunk($s, 1)` answers
> `Core\Str::length($s)` pieces where `str_split($s, 1)` answers `strlen($s)` of them.
> **`replaceAll` and `wrap` are closed too, and each is a fold whose two twins disagree with *each
> other* rather than with MWL.** `replaceAll` folds `str_replace`'s array form and `strtr`, which
> read one table two ways: `strtr` scans the subject once and takes the longest needle matching at
> each position, never rescanning what it produced, where `str_replace` runs each pair over the
> whole subject in turn and feeds every earlier replacement to every later pair. **The member is
> `strtr`**, ADR 0063 R20 being why, and 19 cells of a 48-cell table of cascades, swaps, prefix
> pairs and a growth the next pair re-matches are where the two twins part. The cascading reading is
> not lost and is owed no option: `Core\Str::replace` applied pair by pair in a `foreach` reproduces
> `str_replace`'s array form on all 48, and the property that decides between the two readings is
> *counted* rather than read off a line — the same table written back to front answers the same
> thing on 48 of 48 cells one-pass and on 29 in sequence. **`replaceAll` is also the one `Core\Str`
> member the unit question does not reach**: it matches a needle as a byte sequence, and a valid
> UTF-8 needle can never begin inside a character, so it splits a decomposed cluster exactly as
> `strtr` does — `"e"` over `"e\u{301}ta\u{301}t"`, a subject `Core\Str::length` counts as 4 — and
> there is no divergence file to write. Its one boundary left to the conformance case is the *empty
> needle*, which both skip but PHP warns while skipping, and a warning on stdout is not a difference
> in the result. **`wrap` runs `wordwrap`'s own algorithm** with PHP's third and fourth arguments as
> `{breakWith, cutLongWords}`, so 8 subjects × 8 widths under both cutting readings, a
> multi-character break and the default newline agree outright, the rule that a break already
> present in the subject *resets the line* included; both refusals agree as well — an empty break
> and a zero width that must cut, `ValueError` against a catchable `Fault::thrown` — while a zero
> width *without* cutting is an answer on both sides and breaks at every space. What the cutting
> option is worth is counted: 352 of 352 lines are within their width when long words may be cut,
> and 122 when they may not. **`wrap`'s width counts clusters where `wordwrap` counts bytes**, so
> its multibyte half is still owed as an `--ORACLE-DIVERGES--` file — the last `Core\Str` divergence
> file not written, and the one item of the closed section that is not counted. **`codePoints` and
> `fromCodePoint` close the section, and they are the one pair whose unit the *member* chose rather
> than one the class default imposed**: `codePoints` is `Unit::CodePoint` on purpose,
> `mwl_core_str_code_points`'s doc comment being the home of why, so it parts from
> `Core\Str::length` on exactly the subjects where a cluster holds more than one scalar value — four
> of seven sampled, `\r\n` among them, which is the one place inside ASCII the two units disagree.
> Inside ASCII the byte-wise half of PHP's pair answers the same question, so `str_split` plus
> `array_map("ord", …)` is `codePoints` and `chr` is `fromCodePoint` over the whole 0–127 table: the
> 128 encodings render as bytes sixteen to a line, the round trip holds on all 128, each answer is
> exactly one byte, and both members are injective there — 128 code points giving 128 distinct
> strings. **Past 127 the pair parts in both directions and the reason is the same one read from
> either end.** `ord` reads a byte and `str_split` cuts between them, so `array_map("ord",
> str_split("é"))` is `[195, 169]` where `codePoints("é")` is `[233]`; and `chr` constrains its
> argument with `% 256`, which makes `chr(55296)`, `chr(1114112)` and `chr(0)` one byte and the
> function not injective past 255 — with no diagnostic. `fromCodePoint` refuses instead: a surrogate
> in `55296..=57343` and anything past `1114111` are a catchable `Fault::thrown`
> (`mwl_stdlib::str::scalar_value`), never a substituted U+FFFD, ADR 0009 § 3's checked-not-repaired
> rule reaching a code point exactly as it reaches a buffer. **PHP is retreating from the wrap from
> its own end**: on 8.5.9 `chr()` *deprecates* an argument outside `0..255`, and the notice it
> prints to stdout is itself why that half cannot be an oracle leg. What survives the unit change is
> the round trip, which holds on every multibyte subject too, `fromCodePoints` being the fused
> spelling of `fromCodePoint` applied element by element. **`Core\Regex`'s two twins are closed, and
> each parts from PHP over a *shape* rather than over a result.** `quote` and `preg_quote` escape
> different sets outright — 18 characters here, `#$&()*+-.?[\]^{|}~`, against 22 there,
> `!#$()*+-./:<=>?[\]^{|}` — because `&` and `~` are meta to the Rust engine's character-class set
> operators and not to PCRE, `!:<=>` are meta to neither engine and PCRE's launderer escapes them
> anyway, and `/` is escaped only because a delimiter was handed in, which ADR 0056 § 5 removed. The
> outputs are therefore not comparable at all, and what is counted instead is the property both
> launderers exist for: over the whole 95 × 95 printable-ASCII grid a quoted character matches
> itself and nothing else, the same holds over a 20 × 39 grid of metacharacter-carrying literals
> against the strings their unlaundered reading would have reached, every quoted literal is still
> found *inside* a larger subject, and the one row-level agreement left is that neither launderer
> touches a word character. **`Core\Regex\Match::groups` is `preg_match`'s `$matches` under
> `PREG_UNMATCHED_AS_NULL` and not under PHP's default**: ADR 0063 R11 removed the `PREG_*`
> constants, so one of the two readings has to be the only one, and the default's trimming of
> *trailing* unmatched groups plus its `""` for the ones in the middle conflates "not declared",
> "declared and did not participate" and "participated and captured nothing" — the first two being
> exactly what `group`'s throw-versus-`null` split is built on. Over twelve rows the two readings
> part on six of them, six entries short in total, and the flagged one agrees with `groups()` on
> every key, every value and the order they arrive in, a name before its number.
> **`Core\Bytes::pack`'s refusals are closed, and they are the first block `--errors` has emptied**:
> all six argument sites and all three range sites now have a case asserting the *message* each
> names rather than only that something threw, which is what the older `pack` case reported. What
> the two cases add past their rows is counted. Every code accepts exactly one class of argument, so
> a 15 × 7 grid of code against argument kind partitions the roster into 3 buffer codes, 7 integer,
> 4 float and `x`, which reads no argument at all and therefore accepts none — a code that grew a
> conversion of its own, an integer field taking a numeral string or a float field taking an `int`,
> still looks right on its own line and fails there. An integer field's accepted range is the
> **union** of its width's signed and unsigned ranges, so `-1` and the unsigned top write the same
> octets and the signed bottom writes what its own negation writes, at every width, while the value
> one past either end is refused on both sides. **The 64-bit codes are the one width whose range
> refusal is unreachable**, and that is not a gap: `int` and `uint` together are exactly what `J`
> and `P` accept, so a value past the bound is `E0429` at the call and never becomes an argument —
> `-9223372036854775808` is not a writable literal at all, and the field's low end is spelled
> `-9223372036854775807 - 1`. **A 32-bit float field is the one place `pack` accepts a lossy
> write**: precision is what a caller chose when they wrote four octets, so `0.1` rounds silently,
> while a finite value that would round to an infinity throws and the same argument is an ordinary
> field under the 8-byte code. **`Z` differs from `a` by exactly the octet it reserves**, asserted
> as an agreement over five widths rather than as a row, and `a0` over the empty argument is the
> sharpest accepted cell against `Z0`, which has nowhere to put its NUL. **`bytes.rs`'s reachable
> list is empty now, and so is `Core\Encoding`'s decoder half.** `unpack`'s two bounds,
> `Core\Bytes::at` and `Core\Bytes::fill` each name the last octet they read beside the first they
> cannot, and each of `fromBase64`, `fromBase64Url`, `fromBase32` and `fromHex` says which of its
> reasons refused and at what offset — four reasons for base64 and for base32, two for hex, and
> which one a text gets is decided by the earliest thing wrong with it, so a space at offset 4 of
> eight base32 symbols is the alphabet's refusal while a space that also makes the text nine long is
> the group's. What the two cases add past their rows is counted: a code reads its whole width or
> none of it at every width in the table, an index and its negative name one octet, all 256 values a
> `fill` byte admits are one octet read back, and the offset a base64 refusal names is the offending
> position at each of eight in turn rather than a constant. Two boundaries are the compiler's rather
> than the member's — `fill`'s low end is `E0401` on a `uint` parameter, as `pack`'s 64-bit range is
> `E0429` at the call — and the operand a decoder quotes back is bounded at 32 characters, asserted
> on both sides of that bound. **`--errors` names no assertable site in `str.rs` or `encoding.rs` at
> all now.** Spec § 7's `encodeText` says which character the charset has no spelling for, its code
> point and the byte offset it sits at, and `decodeText` the offset the first unreadable sequence
> *starts* at — there being no character to quote when the whole point is that these octets spell
> none. Spec § 1's five say what they stopped at: `at`'s index against the length it is outside of,
> `chunk`'s size, `countOf`'s needle, and which of `wrap`'s two options, whose width pair is the
> sharper because the same width of 0 is an answer or a throw depending only on `cutLongWords`.
> **`Core\Str::at`'s message was the one in `mwl-stdlib` missing the `()` every sibling writes** and
> now has it. What the two cases add past their rows is counted. `Latin1` is the one charset with no
> refusal at all, so it reads every one of the 256 octets and each returns as the octet it went in
> as, while `Ascii` reads exactly half — the same bound stated as a partition rather than as two
> rows — and `isValidText` answers `false` at exactly the octets `decodeText` throws for on all 256,
> which is what makes one the other's question with somewhere to put the answer. **Both encoding
> members count their reported offset in bytes**, asserted over eight positions in turn on each
> side, which is the one thing the two messages have to agree about since a caller uses one to index
> what it handed the other; and `encodeText`'s quoted operand is bounded on both sides of 32
> characters, ADR 0088's register being why it is bounded at all. `at`'s accepted set is exactly the
> 2n indices from -n to n-1 and nothing else, over sixteen spanning both bounds, and the length it
> reports agrees with `Core\Str::length` on six subjects whose byte and cluster counts diverge in
> different places. **Two spellings a case reaching for these cannot use**: a source-declared
> `Core\Charset` parameter does not unify with the registry's own enum type and fails `E0401` naming
> the same type twice, so the enum is written at each call site; and a message quoting a C1 control
> back cannot be a frozen `--EXPECT--`, so U+0080's row asserts the printable half. Both are
> playbook bullets. **`path.rs` holds no assertable site at all now, and neither does `Core\Time`'s
> rendering half.** Spec § 8's `withExtension` refuses four ways, and because two of a call's
> arguments can be wrong at once, what the case pins past the four sentences is the *order* they run
> in: the extension is checked before the path, and inside the extension the empty one beats the dot
> beats the separator — which is why `withExtension('/', './a')` reports a dot whose suggested
> replacement the next check would itself refuse. What it adds past its rows is counted once per
> argument. Twelve extensions against one path leave four accepted, the dot being refused only at
> position 0 — so `tar.gz` and `txt.` are usable extensions and `.txt` and `.` are not, the second
> of those being the sharpest cell because the extension it names instead is the empty one the
> *first* check refuses — and every refusal but the empty one quotes what it was handed. Twelve
> paths against one extension leave the nameless ones exactly the six roots, a *trailing* separator
> not being one, each naming the path back. **Spec § 4's three `format` members read one pattern
> compiler** (`mwl_stdlib::cldr`) and each wraps its refusal in its own name, which is the half of
> the sentence the neighbouring § 4 cases never asserted: an unknown letter, an unterminated quote
> and `V` at any count but two are one grammar spoken in three names. What the two zone-free halves
> do not share is the narrowing, which refuses a field rather than filling one in because every
> value they could invent — midnight, the process's own zone — is a wrong answer stated confidently,
> and it is asserted as a **partition**: over the 16 fields the subset carries, no field is accepted
> by both `Date` and `TimeOfDay`, and the three neither takes are exactly the zonal ones. Over the
> 52 ASCII letters exactly 15 are fields and 36 of the 37 refusals quote the letter they were
> handed, the odd one out being `V`, a pattern letter at the wrong count and saying so. The compile
> step runs before either narrowing, so an unreadable letter beats a field the value cannot carry.
> **The unknown-zone sentence is spelled three times, each behind the name of the member that would
> speak it, and only `Zone::of`'s is reachable** — the other two read an id this crate wrote itself.
> What that case adds past the sentence is counted twice. Over fourteen ids the six accepted are
> exactly the region names: a sign at position 0 is refused whichever way it leans, because that is
> `fixed`'s spelling and R20 leaves one way to build one value, while `Etc/GMT+5` is an ordinary
> name whose sign means the *opposite* of what a sign means in an offset — and every refusal equals
> the sentence rebuilt from the id it was **handed** rather than the canonical one, the lookup being
> case-insensitive, so `europe/berlin` is accepted and stored as `Europe/Berlin`. The second count
> is the premise the two unreachable siblings rest on: all seventeen `DateTime` members that read
> the zone slot answer for every zone, and each fixed offset rebuilt out of the `DateTime` it was
> stored in is worth what it went in as — which is where the round trip could actually break, a
> fixed zone's id being rendered rather than looked up. **`Core\Time::parse` throws two of § 10's
> classes and which one is decided by which argument was wrong**: a pattern is written by the call
> site, so a bad one is a `LogicError`, while text arrives from elsewhere, so text that does not
> match a well-formed pattern is a `ParseError`. The two tables are asserted as a *partition* — not
> one of seven bad patterns lands in `ParseError`, not one of six bad texts in `LogicError`, and
> every sentence on both sides is wrapped in the member's own name. **One refusal crosses that
> split, and it is the pattern that names a zone**: `format` accepts every zonal field and shares
> the compiler, so only the reader can refuse it, and a *pattern* mistake therefore arrives as a
> `ParseError`. The compile step runs first, so a call with both arguments wrong reports the
> pattern. A calendar refusal — an impossible date, an hour past 23 — is `jiff`'s own wording and is
> asserted by class and by the name in front of it rather than frozen. **`Core\Time::fromIso`
> refuses in `ParseError` too now, and not in the bare `RuntimeError` a plain `Fault::thrown`
> gives** — the one behaviour a session changed here rather than only asserting. That member's
> single argument is text that arrived from somewhere else, so the rule `parse` splits its two
> classes by decides this one outright, and § 10's own gloss on the class is "input did not match a
> format this code declared". The sentence past the member's name is `jiff`'s, because it says which
> component it stopped at, so it is asserted by its class and by the name in front of it rather than
> frozen — counted over fourteen texts, of which exactly the five carrying their own offset are
> accepted, all five at one epoch second, and all nine refusals wrapped in the member's own name.
> The bound is one character wide, the same civil time being refused without an offset and accepted
> with one; and the five accepted spellings share a *second* rather than a point, since a subsecond
> is read and kept and `toEpochSeconds` is what truncates it. **`--errors` now names no assertable
> site in `math.rs` or `time.rs` either, and `Core\Math`'s only guarded arguments are `log`'s base
> and `format`'s decimal count.** What the two cases add past those two sentences is counted. Over
> the eighteen members whose argument and whose answer are both a `float`, one `NaN` is answered by
> all eighteen and refused by none — the *argument's* domain is IEEE's throughout, which is what
> leaves `isNan` and `isFinite` something to ask about — while that same value is refused by `sign`,
> which would otherwise hand back a side of zero the caller goes on to branch on, and by `format`,
> which has no digit string for it. `log` is one of the eighteen and answers the `NaN` with the
> rest; its base arrives through an *option* rather than a position, which is why the sweep cannot
> see it, and the bound is named on both sides: zero is refused whichever sign it carries, the least
> positive `float` — one subnormal step above it — is accepted and answers a ratio of logs, and base
> `1.0` is inside the guard and is `NAN` rather than an infinity. `format`'s cap is stated the same
> way, a hundred decimals accepted against the hundred-and-first refused, and its sentence quotes
> the count it was *handed* rather than its own constant twice — asserted at 101, 200 and 1000 in
> turn — while the accepted count really is the fraction's width, one more decimal being exactly one
> more character over six subjects whose exact binary value terminates before either count.
> **`Core\Json`'s two `thrown_as` and `Core\Regex\Match::group` are closed, and two messages changed
> rather than only being asserted.** What `Core\Json::encode` refuses is a partition over the *tags*
> rather than a list: eleven values with a JSON spelling render — both array shapes among them, a
> list as an array and a string-keyed map as an object, and a `decimal` written as ADR 0054's exact
> number rather than through a `float` — while the five without one all throw in `LogicError`, the
> class saying the value was built by the program rather than handed to it by the world, and each is
> wrapped in the member's own name. The three sentences are frozen and two of them are new here: a
> non-finite `float` is quoted the way `echo` spells it, `NAN` and `INF` through
> `mwl_runtime::php_float_to_string` rather than Rust's `inf`, and a value whose tag has no spelling
> is named by `Tag::describe` — ``a `bytes` value has no JSON encoding`` — where it reported a tag
> *number* only this crate can read, that arm being where a `bytes` argument lands as
> `Encodable::text`'s own doc comment already said. **`decodeAs<T>`'s codec refusal is the same rule
> read from the other end, and it is asserted as an agreement**: ADR 0071 § 5 makes one attribute
> decide both directions of the wire, so over three classes declaring the same two fields the two
> members answer alike on all three and exactly one participates. Two claims sit past that — the
> codec is read *before* the document, so one unreadable text is a `LogicError` for a class with no
> codec and a `ParseError` for one with, and each refusal quotes back the class it was handed rather
> than a constant. **One wart is left there**: a class carrying the attribute and declaring *no*
> field has an empty codec, so both members refuse it with the sentence that says it does not carry
> the attribute at all — the message is wrong about why, and whether a fieldless class should encode
> as `{}` instead is the question behind it. **`Core\Regex\Match::group` splits three ways over a
> pattern's whole group set**, where PHP's absent array entry folds the refusal and two of the
> answers together: of the eight keys `groups()` reports, four are reached, two participate and
> capture nothing and two are declared but never reached, and `group` agrees with `groups()` on
> every one of them. Everything outside that set is a `Fault::thrown` and so a `RuntimeError` —
> where the call-site-argument rule that made `Core\Time::parse`'s bad pattern a `LogicError` argues
> for `Logic`, which the case asserts as it stands rather than as that rule implies — and the
> sentence is rebuilt from the key it was handed at three keys in turn. The bound is one number
> wide, group 4 being the last the pattern declares and group 5 the first it does not, and a name
> and its number reach one group at all three named ones, as do the `int` and `string` spellings of
> a number. **What a case can still catch is four sites**, one each in `arr.rs`, `csv.rs`, `out.rs`
> and `random.rs`, and no file holds more than one. **Unbuilt in the library**, none of it a
> registration gap: `Core\Json::decodeAs<T>`'s decoder, which ADR 0071 leaves reading a
> scalar-fielded class only — no enum, `decimal`, `Instant`, `array` or nested-class field, and no
> optional key from a parameter default (`mwl_stdlib::json` gap 2); ADR 0088's qualifier
> classification, missing from every `mwl-stdlib` member row, which is why `Core\Str::format`'s
> template is not yet the sink that ADR makes it, why neither the fail-closed default for an
> unclassified `string`/`bytes` parameter nor the test refusing an unclassified member exists, and
> why `Core\Hash::hmac`'s `secret bytes $key` is a plain `CoreTy::Bytes` (`mwl_stdlib::hash`'s
> module doc); and ADR 0086 § 1's substitution table, which is what would make the terminal sink
> neutralize a control byte and `Cli\Text::plain` a constructor that cannot produce an injected
> escape (M8, `crates/mwl-stdlib/src/cli.rs` gap 1). **Decided and unbuilt, but not catch-up** —
> ADRs 0091 (the `development`/`production` run mode), 0092 (one diagnostic record rendered three
> ways by the sink in force), 0093 (`mwl service`), 0097 (the server's scope and its `[server]`
> block) and 0100 § 3 (a file opening `#!` starts in code mode with no tag — one `mwl-syntax` branch
> at offset 0, `E0009` reserved in the registry, no parser or runtime change). None invalidates
> built behaviour or a written fixture; their work is M1, M4, M6, M7, M8 and M10. **Open beside the
> library** — a property's declared default runs and is type-checked (`E0472`), limited to a literal
> or `[]`; `do`/`while` is the one M4 control-flow statement that does not lower; a closure cannot
> be called through the variable holding it and `Class::method(...)` panics `mwl-ir` outright, so a
> case sweeps a table with `foreach` and reaches a helper through a `public static function`
> declared in the same file (`mwl-ir` gap 1); a `?bool` cannot be tested for truth at all, so a
> member answering one has no `yn` rendering; `bool as int` does not lower and `bool as string`
> renders `false` as nothing at all; an abandoned generator never runs the `finally` it is suspended
> inside (`mwl-ir` gap 18, a deliberate PHP divergence); ADR 0043's `by`-delegation is off path; and
> `docs/spec/02-php-migration.md` is 31% classified, reported by `python tools/check-migration.py`.
> **What has landed is not restated here** — `git log` holds the session-by-session history and each
> crate's own module doc holds its per-file gaps.
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
> decision: the differential gate is met at 151 of 150, conformance stands at 502 of 600, and the
> remainder is written a section at a time.

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
