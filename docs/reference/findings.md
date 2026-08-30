# Findings from writing the reference — every place the binary and the docs disagree

Collected on 2026-08-30 while `docs/novis.md`'s chapters were written against `target/debug/nvs.exe`.
Every item was **observed**, not inferred: the spelling that was run is given, and the probe files
named in italics are under `.agent-tmp/` (gitignored — rerun the spelling if they are gone). The
reference documents what the binary *does*; this file is the list of what to decide about, one by
one. Tick an item when it is fixed, or when the decision is "the binary is right" and the doc that
disagreed has been folded.

Categories: **P** — a panic or abort (exit 101/127) where a diagnostic or a clean error was owed;
**U** — a rule the docs state that the binary does not enforce; **D** — the binary and an ADR/spec/
registry card disagree on behaviour; **M** — a member, class or feature the docs name that does not
exist in the registry (or the reverse).

## Panics and aborts (P)

- [ ] **P1** First-class callable syntax `Class::method(...)` / `$obj->method(...)` type-checks and
      panics in `nvs-ir` (`lower/expr.rs:2870`, "records `ExprInfo::CallableRef` … no arm"). ADR 0027
      keeps the spelling. Works only as the argument of `Core\Attributes::get/all`, where it is
      folded at check time. *probes1 `fcc`, probes2 `fcc_case`*
- [ ] **P2** Calling an instance method statically — `class C { public function f() … } C::f();` —
      panics (`lower/expr.rs:2978`, "is not static but is reached from a frame with no `$this` —
      nvs_types is expected to have refused that"). *probes2 `static_call_instance`*
- [ ] **P3** `$this` inside a `static` method panics (`lower/expr.rs:138`, "undeclared local `$this`").
      *probes2 `this_in_static`*
- [ ] **P4** A method declared `: never` panics the lowerer even when never called ("nvs-ir only
      lowers a resolved call's … return type — got Never"). *types-probes; ref30 `never`*
- [ ] **P5** An `array<T>`-typed class constant panics at use: `public const array<int> XS = [1, 2];
      Core\Arr::count(K::XS)` (`lower/expr.rs:286`, "no value recorded"). *types-probes*
- [ ] **P6** An **untyped interface constant** panics at use: `interface I { public const LIMIT = 9; }
      echo I::LIMIT;` — the typed form works, an untyped *class* constant works. *probes q31*
- [ ] **P7** `throw "x";` / `throw 1;` panic ("nvs-ir lowers `throw` only for an exception object")
      instead of a diagnostic. *ref30 `throw_string`*
- [ ] **P8** `clone` of an array — `array<int> $b = clone $a;` — panics ("lowers `clone` only for an
      object"). *ref30*
- [ ] **P9** `new $className()` with a `string` variable panics ("`new` … has no resolved class");
      likewise `$className::f()`. *ref30; php-diff probes*
- [ ] **P10** Anonymous classes `new class { … }` pass the checker and panic in `nvs-ir`. *probes p03*
- [ ] **P11** An enum case as an array key — `$m[E::A] = "a"` — panics in lowering. *probes p25*
- [ ] **P12** Calling a member the registry does not hold on a `Core` instance — `$uuid->version()` —
      panics (`lower/expr.rs:2681`, "instance method call … has no resolved target") instead of the
      E0405 a static miss gets. *coretime-probes `u_version`*
- [ ] **P13** Reading a `catch` binding after its clause panics ("undeclared local `$e`") rather than
      being refused by the checker. *ref-errors `catch_scope`*
- [ ] **P14** `catch (LogicError | IOError $e)` parses and checks, then fails in codegen ("does not
      lower `instanceof LogicError | IOError`", exit 1); a property read through the union is E0495.
      *ref-errors; ref30*
- [ ] **P15** A memory-limit breach inside `try { … } finally { … }` aborts the process with a Rust
      panic in `nvs_array_release` ("attempt to subtract with overflow", exit 127) instead of the
      clean `FATAL`. Without the `finally` it is clean. *refp/fatal/main.nvs*
- [ ] **P16** `Core\Env::EOL` panics in `nvs-ir` (exit 101) — `Core\Env` resolves as a class but has
      no members; the `E0319` help text names `Core\Env::mode()`. *php-diff probes*

## Rules stated but not enforced (U)

- [ ] **U1** `readonly` is inert: a property (declared or promoted) is written after construction
      and reads back the new value. *probes2 `readonly_write`, `readonly_write_method`; r08*
- [ ] **U2** `final` is inert: `final class A {} class B extends A {}` and an override of a `final`
      method both compile and run. *probes p09, p10, r07*
- [ ] **U3** `new` on an `abstract` class compiles and runs; calling the bodiless method then dies
      with `FATAL: internal error: a method with no body was called`. `abstract` on a method of a
      non-abstract class is not refused either. *probes p11, p11b, q01*
- [ ] **U4** `secret` is refused only by a `Throwable` message (E0422), `Core\Debug` (E0724) and any
      unclassified `string`/`bytes` `Core` parameter (E0401). `echo $pw;`, `"{$pw}"` interpolation
      and `Core\Json::encode($pw)` all print the value — ADR 0033 § 4 lists output among the sinks.
      *probes2 `secret_echo`; types-probes*
- [ ] **U5** `Core\Secret::reveal` does not exist (E0405), yet the E0422/E0724 help texts tell the
      user to call it. No member returns `tainted` or `secret`; a value is qualified only where a
      declaration spells it. *types-probes*
- [ ] **U6** `#[Command]` does not force `tainted string` on a positional parameter (`string $name`
      compiles), accepts an instance method and an `int` return — ADR 0086 § 6 says static,
      `void`/`uint`, tainted. *ref-attr `command-*`*
- [ ] **U7** Under `nvs run` only `[limits] memory` is enforced: `wall_time = "1s"` with an infinite
      loop ran past 60 s, `cpu_time = "1s"` completed a 7 s loop, `max_output = "10"` let 28 bytes
      through. `Core\Fatal::onLimit`'s card lists all of them. *refp/cpu, refp/wall, refp/out*
- [ ] **U8** A memory-limit breach in a **child isolate** is not observed: `[limits] memory = "8M"`
      with a child holding 64 MiB answered `ok = true`. *refp/childmem*
- [ ] **U9** `nvs config check`/`nvs run` do not validate quantities or ceilings: `memory = "12
      bananas"` and `[limits] memory = "1G"` over `[limits.hard] memory = "512M"` both pass and run.
      `tree.rs` says values are refused where sizes are parsed. *php-diff/config probes*
- [ ] **U10** The config trust rule (a file writable by another account is refused) is applied by
      neither `nvs run` nor `nvs config check` — `crates/nvs-cli/src/config.rs` reserves it for
      `serve`/`ctl reload`, which do not exist. E0607/W1005 are unreachable in this binary.
- [ ] **U11** `password_file` is not materialized under `nvs run`: `Core\Config::get("db.main.password")`
      is `null`, and `config dump` shows the path rather than `<secret>` (secret.rs says `<secret>`).
      The both-set refusal E0608 does fire.
- [ ] **U12** A write to a get-only hooked property compiles and is silently unobservable. *q03*
- [ ] **U13** `$e->message = "b"` on a throwable is accepted and reads back — the properties are not
      read-only, though a conformance case calls `location` "readonly". *ref-errors `write_message`*
- [ ] **U14** `#[Access]` on a method with no `#[Route]` compiles (no stray-marker refusal, unlike
      `#[Query]`/`#[Option]`/`#[Api]`). *ref-attr*
- [ ] **U15** `nvs test --filter` does not select `#[Test]` methods — `--filter clock` ran all four
      tests; it filters `.nvst` paths only. *ref-probe/t1*
- [ ] **U16** `#[Test(db: …)]` and `#[Test(server: …)]` are accepted and have no reader in the runner.
- [ ] **U17** `array $a = [1, 2];` with no `<T>` is accepted (ground rules: every array declares its
      element type). *php-diff probes*
- [ ] **U18** `<>` parses as `!=` — ADR 0090 § 1 says `==`/`!=` are the whole set. *ref30*
- [ ] **U19** `try { … }` with neither `catch` nor `finally` is accepted (PHP refuses it). *ref30*
- [ ] **U20** `namespace A { class B {…} echo B::f(); }` (the braced form) parses and resolves. *php-diff*
- [ ] **U21** `Iterator::current()` outside the protocol does not throw on a generator (ADR 0053 § 1
      says it does): `0` before the first `advance()`, the last value after exhaustion. *p44*

## Binary and docs disagree on behaviour (D)

- [ ] **D1** The time types do **not** implement `Comparable`: `$i < $j` on two `Core\Time\Instant`,
      `Duration`, `Date` or `TimeOfDay` values is `E0411 … does not implement Comparable`, while the
      cards say `compareTo` is "as `Comparable` requires" and spec § 4 says `<`/`>` work directly.
      *coretime-probes `t_instant`, `t_duration_echo`, `t_date`, `t_tod_lt`*
- [ ] **D2** `Core\Time\Duration ==` is identity: `90m == 1h30m` is false; `compareTo` answers `0`.
      *types-probes*
- [ ] **D3** `Core\Weekday as int` is zero-based (`Friday` → `4`); the enum card says the cases are
      ordered "as `date("N")`" (Friday = 5). *coretime-probes `t_enum_int`*
- [ ] **D4** A literal pattern/duration argument is a **compile error** (E0769), not the
      `LogicError`/`ParseError` the cards name: `->format("yyyy-QQ")`, `Duration::parse("30 seconds")`.
      Only a computed argument throws. *coretime-probes `t_format_bad_literal`, `t_parse_errors`*
- [ ] **D5** `Core\File::read("missing.txt")` under a valid `fs.read` grant throws the **capability**
      `RuntimeError` ("needs the capability fs.read for missing.txt, which is not granted"), not the
      card's `IOError`; `"./missing.txt"` gets the `IOError`. Likewise `write("copy.txt", …)` with
      `write = ["."]` is refused while `"./copy.txt"` succeeds. Cause: `capability.rs:257`
      `resolved()` — `Path::new("copy.txt").parent()` is `""`, which never canonicalizes.
      *refp/fs; coretime-probes*
- [ ] **D6** `Core\Router::url` with a **computed** name throws for every name, declared or not:
      "no route is named 'u'. The compile-time route table is not built yet (ADR 0077 § 5)". The
      card says only an unknown computed name throws. *ref-attr `route-url-computed-name`*
- [ ] **D7** `Core\Json::decodeAs<T>` is a **FATAL** for an array, enum or nested-class field (encode
      handles all three). *ref-attr `derive-array-and-enum-field`, `derive-decode-nested-and-float`*
- [ ] **D8** A promoted constructor parameter is not a `#[Json\Derive]` field — a class with only
      promoted state is refused E0758; ADR 0071 § 1's own example uses promotion. *`derive-promoted-ctor`*
- [ ] **D9** Private properties are JSON fields under `#[Json\Derive]` (`{"name":"a","n":1}` for a
      `private int $n`). *`derive-private-property`*
- [ ] **D10** `#[Api]` fields `tags`, `security`, `errors`, `example` are checked but absent from the
      `nvs build --openapi` document. *ref-probe/api2*
- [ ] **D11** `spawn script Class::method(...)` as the operand is refused (`E0401: expected string,
      found callable`); spec § 2 and ADR 0006 describe it as available. *refp/spawn/method.nvs*
- [ ] **D12** `spawn script … with(args: …)` is accepted but there is no reader: `Core\Script::args()`
      is E0405, and the parser's own hint (`$_ARGS` → `Core\Script::args()`) names it. *refp/spawn/args.nvs*
- [ ] **D13** `spawn script` default `output` is `'capture'`, not inherit. *refp/spawn/main.nvs*
- [ ] **D14** A child with no top-level `return` answers `value = int(1)`, not `null`. *refp/spawn/noret2*
- [ ] **D15** `Core\Config::get("mode")` answers `null` and `set("mode", …)` returns `false`;
      `mode.default` works for both — ADR 0091 § 4 spells the bare `mode`.
- [ ] **D16** `Core\Arr::from` refuses the `Core` collections although they are `Iterable`:
      `from($objectSet)` → E0401 "expected `array<T>|Iterable<T>|Iterator<T>`, found
      `Core\ObjectSet<int>`"; on `ObjectMap` the message leaks the unsubstituted `array<K>`. `foreach`
      over them works. *q09b, r04b*
- [ ] **D17** `Core\ObjectSet::union`/`intersect`/`diff` lose the element type: the result is a bare
      `Core\ObjectSet` (`foreach … as Tag $t` → "expected Tag, found T"). *ref-core-probes2*
- [ ] **D18** `object` is not fully opaque: a method call through `object` is refused (E0477) but a
      property read compiles and resolves at run time ("`A` has no field `nope`" on a miss); the
      diagnostic cites ADR 0036 § 4, so this may be intended. *r02, s01*
- [ ] **D19** A comparison (`$e == E::A || $e == E::B`) does not narrow an enum value to the
      case-union type; only `as E::A|E::B` does. *q06, r03b*
- [ ] **D20** An empty shape `{}` is satisfied by every attached attribute literal, so a bare marker
      `#[Audited]` (`type Audited = {}`) is ambiguous (E0728) on a class with any other attribute.
- [ ] **D21** A payload holding a class constant or enum case can be declared but not retrieved
      (E0731) with `Attributes::get`/`all`.
- [ ] **D22** `inout` accepts only a local: `M::bump(inout $a["k"])` is E0439 "cannot be passed to
      an `inout` parameter **yet**". *ref30*
- [ ] **D23** ADR 0031 § 3's named-closure recursion (`fn fact(int $n): int => … fact($n - 1)`)
      parses, but the recursive call resolves as a free function (E0320). *ref30*
- [ ] **D24** `1.0 / 0` answers `INF` without throwing; ADR 0007 § 4's `/ 0` row is the integer one.
      *ref30*
- [ ] **D25** A property default of `null` on a `?T` property is refused (E0472, "must be a `int|null`
      constant … not a constant of the declared type") — `null` is exactly such a constant. *probes2
      `nullable_default_int`, `nullable_default_class`*
- [ ] **D26** `continue` (level 1) inside a `switch` inside a loop continues the enclosing loop — the
      documented Novis choice, but PHP acts as `break`; noted so the crosswalk row stays deliberate.
- [ ] **D27** A shebang `#!` first line is **not** recognized: the line is HTML-mode text and is
      copied to the output. ADR 0100 says a file opening `#!` starts in code mode. *php-diff probes*
- [ ] **D28** The on-disk compile cache is unwired: `cache.rs` and `[cache] dir` exist, nothing in
      `nvs-cli/src` outside `cache.rs` references it, and `nvs run` compiles fresh every time.
- [ ] **D29** `await`'s result renders as `Core\Script\Result#1` in `Core\Debug::render`, while
      `isolate.rs` types it as an anonymous shape.
- [ ] **D30** Diagnostics name classes that are empty or absent in this build: E0211's help says
      `Core\Request`, `Core\Server`, `Core\Cli`; each resolves but has no members and none is in
      `nvs meta --json`.
- [ ] **D31** `Core\Cli\Text` has no members and no constructor (module doc: deliberate), so
      `Core\Str::length($text)` is refused while `echo`, `.` and `as string` accept it; ADR 0086
      gives it `plain`/`styled`/`+`.
- [ ] **D32** `Core\Validate::isEmail` does not launder — `string $s = $in;` after a `true` answer is
      still E0401 (may be intended; noted because a reader expects a validator to launder).
- [ ] **D33** `assertSame($uintValue, 2)` is `E0401: expected uint, found int` — a literal beside a
      generic parameter does not adapt.
- [ ] **D34** `Core\Arr::sort` on an `array<decimal>` throws at run time: "no natural order for tag
      10 against tag 10: two numbers, two strings, two bools or two nulls have one" — `decimal` is a
      number and the checker accepted the call. Found by the blind proof's CSV task. *probes4 `sort_decimal`*
- [ ] **D35** There is no typed decode of a JSON **array**: `Core\Json::decodeAs<array<U>>` is E0465
      ("builds a class, and `array<U>` is not one"), so a list of derived objects is read as
      `array<mixed>` and converted element by element. Found by the blind proof's JSON task.
      *probes4 `decode_list`*

## Named in the docs, absent from the registry (M)

- [ ] **M1** `Core\Task::afterResponse` (spec § 19) — only `all` and `map` exist.
- [ ] **M2** `Core\Fatal::onUncaughtThrow` (ADR 0020 § 2) — only `onLimit` exists.
- [ ] **M3** ADR 0079's wider assertion roster (`assertStartsWith`, …) — `Core\Test` holds ten members.
- [ ] **M4** `Core\Test\Failure` and `RecursionError` appear in no member card's `errors` list, only
      in the exception tree (behaviour verified by probe).
- [ ] **M5** `Core\Uuid` has no `version()`; the cards' `errors` never name it. (Calling it is P12.)
- [ ] **M6** `Core\Command::run`/`help`/`completions` (ADR 0086 § 6) do not exist; the command table
      is built and checked, and `Core\Program::implementing` is the only reader.
- [ ] **M7** `nvs serve`, `nvs fmt`, `nvs convert`, `nvs lsp`, `nvs ctl` are unrecognized subcommands.
- [ ] **M8** `Core\Secret`, `Core\Taint`, `Core\Log`, `Core\Env`, `Core\Cli`, `Core\Request`,
      `Core\Server`, `Core\IO`, `Core\Html` resolve as names in diagnostics or the crosswalk but have
      no registry rows. Part D's *dropped* rows still cite `Core\Html::escape` and `Core\IO::within`.
- [ ] **M9** `Core\Env::mode()`, `$_ARGS`/`Core\Script::args()` are named by diagnostic help texts
      and do not exist.
- [ ] **M10** The registry cards cite ADR numbers inline in 33 places ("ADR 0056's two engines",
      "ADR 0009's default unit") — meaningless to the reference's readers. `tools/reference.py`
      strips the parenthesised form `(ADR 0013)`; the inline ones need rewording in the cards.

## Facts worth keeping (not bugs, but not written anywhere a user reads until now)

- `as` throws `ArithmeticError` for a numeric range failure (`3.9 as int`, `-1 as uint`) and
  `RuntimeError` for everything else (a non-numeric string, a wrong `mixed` tag, an enum miss).
- `string as int` refuses `" 42"`, `"42 "`, `""`, `"0x1A"`, `"12.0"`; `"1e3" as float` is `1000`.
- Two `catch` clauses in one function must bind different names (E0406); so must two
  `for (int $i …)` loops; two `foreach` loops may reuse a binding name.
- `1 << 64` answers `0`; a negative shift count throws.
- Escapes `\v`, `\e`, `\f` are not escapes (`"a\vb"` prints `a\vb`); literal `017` is decimal 17.
- A `foreach` over an `Iterator<T>`/generator has no key to bind (E0444).
- Both grant spellings work from an example's own directory: `[[app]] entry = "main.nvs"` +
  `[app.capabilities.fs]`, and the top-level `[capabilities.fs]` table.
