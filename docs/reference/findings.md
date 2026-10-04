# Findings from writing the reference — every place the binary and the docs disagree

Collected while `docs/novis.md`'s chapters were written against `target/debug/nvs.exe`.
Every item was **observed**, not inferred: the spelling that was run is given, and the probe files
named in italics are under `.agent-tmp/` (gitignored — rerun the spelling if they are gone). The
reference documents what the binary *does*; this file is the list of what to decide about, one by
one. Tick an item when it is fixed, or when the decision is "the binary is right" and the doc that
disagreed has been folded.

Categories: **P** — a panic or abort (exit 101/127) where a diagnostic or a clean error was owed;
**U** — a rule the docs state that the binary does not enforce; **D** — the binary and an ADR/spec/
registry card disagree on behaviour; **M** — a member, class or feature the docs name that does not
exist in the registry (or the reverse).

## Triage

Every item was decided against the ADR that owns it; the decisions that were open were put to the user
and are folded into their ADRs (0006, 0007, 0033, 0047, 0071, 0090, 0091, 0094, 0103, 0107,
`README.md` § *Decisions taken at project start*, `divergences.md`). The code and card work is the five
rows whose owner is an item number, 31 to 35. An item's owner is the row it sits in.

| Verdict | Items | Owner |
|---|---|---|
| Code — the abort and the wrong answers a working program hits | P15, D34, D5, D25, U15, D24, D14, U11, D15 | item 31 |
| Code — a panic or an acceptance where a refusal was owed | P2, P3, P6, P7, P8, P9, P10, P11, P12, P13, P14, P16, U18, U19, U20, M5, M8, M9, D30 | item 32 |
| Code — modifiers and attributes parsed and not enforced | U1, U2, U3, U4, U5, U6, U12, U14, P5 | item 33 |
| Code — lowering and library gaps | P1, P4, D1, D7, D8, D10, D12, D16, D17, D21, D22, D23, D27, D33, D35, U21, M1 | item 34 |
| Docs in the tree — cards, help texts, module docs, reference chapters | M10, D2, D3, D4, D6, D9, D20, D29, U13, M2, M3, M4, M6, M7, and § *Facts worth keeping* | item 35 |
| Planned, and unchanged by this pass | U7, U8, U9, U16, D31, D11 and D28 | each finding's own entry below |
| Closed — the binary is right and the doc now says so | U10 (`rule:config/ownership-is-the-trust-boundary` states the `run`/`check`/`dump` exemption), U17 (`rule:types/grammar`: a bare `array` is `array<mixed>`), D13 (`rule:security/isolate-shares-nothing`: capture is the default), D18 (`rule:types/erased-member-access`), D19 (`rule:types/enum-case-type`: `as` is the only narrowing), D26 (`divergences.md`), D32 (a validator does not launder) | — |

## Panics and aborts (P)

- [x] **P1** First-class callable syntax `Class::method(...)` / `$obj->method(...)` type-checks and
      panics in `nvs-ir` (`lower/expr.rs:2870`, "records `ExprInfo::CallableRef` … no arm"). `rule:types/callable-values`
      keeps the spelling. Works only as the argument of `Core\Attributes::get/all`, where it is
      folded at check time. *probes1 `fcc`, probes2 `fcc_case`* — it lowers to the same closure
      object a `fn` literal builds, over a forwarding thunk (`nvs_ir::lower::anon_fn`'s
      `lower_callable`), so a `Core` member handed one cannot tell it from a written closure. Two
      parameter lists a `callable` cannot forward — `inout` and a variadic tail — are `E0793` where
      the `(...)` is written, since either reaches the callee as a type confusion.
- [x] **P2** Calling an instance method statically — `class C { public function f() … } C::f();` —
      panics (`lower/expr.rs:2978`, "is not static but is reached from a frame with no `$this` —
      nvs_types is expected to have refused that"). *probes2 `static_call_instance`* — `E0778` where
      the call is written, pinned by `tests/conformance/reject/an-instance-method-is-not-called-statically.nvst`.
- [x] **P3** `$this` inside a `static` method panics (`lower/expr.rs:138`, "undeclared local `$this`").
      *probes2 `this_in_static`* — `E0779` at the `$this`, pinned by
      `tests/conformance/reject/this-is-not-read-in-a-static-method.nvst`.
- [x] **P4** A method declared `: never` panics the lowerer even when never called ("nvs-ir only
      lowers a resolved call's … return type — got Never"). *types-probes; ref30 `never`* — it erases
      to `Ty::Void`, the representation of a caller that receives nothing, and the terminator stays
      the callee's own `throw` rather than becoming a mark on the call site;
      `nvs_ir::lower::erase_checked_ty`'s arm owns that choice and names the two refusals it leaves
      the checker. Pinned by `tests/conformance/core/a-never-method-is-a-terminator.nvst`.
- [x] **P5** An `array<T>`-typed class constant panics at use: `public const array<int> XS = [1, 2];
      Core\Arr::count(K::XS)` (`lower/expr.rs:286`, "no value recorded"). *types-probes* — it folds:
      `nvs_types::defaults::eval_const_value` places each element in the declared element type and
      records the array as the same `ConstArg` a folded retrieval already lowers, pinned by
      `tests/conformance/core/an-array-constant-folds.nvst`. What still has no constant form — a
      named constant, an enum case or `Foo::class`, on its own or nested — is `E0792` **at the read**
      rather than a panic, pinned by
      `tests/conformance/core/a-class-constant-with-no-constant-form-is-refused-at-the-read.nvst`.
- [x] **P6** An **untyped interface constant** panics at use: `interface I { public const LIMIT = 9; }
      echo I::LIMIT;` — the typed form works, an untyped *class* constant works. *probes q31* — the
      untyped form is now `E0246` where it is written, for a class as much as an interface, so neither
      reaches the lowerer; the class form's inference is retired with it.
- [x] **P7** `throw "x";` / `throw 1;` panic ("nvs-ir lowers `throw` only for an exception object")
      instead of a diagnostic. *ref30 `throw_string`* — `E0780` names the operand's type against the
      § 10 tree, pinned by `tests/conformance/reject/throw-takes-a-throwable.nvst`.
- [x] **P8** `clone` of an array — `array<int> $b = clone $a;` — panics ("lowers `clone` only for an
      object"). *ref30* — `E0781`, whose help says an `array<T>` is already copied on assignment
      (`rule:classes/clone-is-shallow`); pinned by `tests/conformance/reject/clone-takes-an-object.nvst`.
- [x] **P9** `new $className()` with a `string` variable panics ("`new` … has no resolved class");
      likewise `$className::f()`. *ref30; php-diff probes* — both are `E0496` where they are written,
      the code the third spelling `$x is $className` already had: one mistake under one report
      (`nvs_types::expr::members::reject_dynamic_class_name`), so neither reaches the lowerer. All
      three now accept a `class<T>` value instead (`rule:types/class-reference-sites`), and the refusal's help names the
      `as` that produces one — a `string` stays refused at every one of them.
- [x] **P10** Anonymous classes `new class { … }` pass the checker and panic in `nvs-ir`. *probes p03*
      `E0244` at `new class` stops the pipeline before `nvs-ir`; the declaration is still parsed whole,
      so `nvs_syntax::casing` reaches its members as it reaches a named class's.
- [x] **P11** An enum case as an array key — `$m[E::A] = "a"` — panics in lowering. *probes p25* — the
      key type check refuses it as `E0434`, beside the `float`/`bool`/`null` keys and at all three sites
      that write a key: `rule:enums/closed-integer-type` makes a case a named integer, so the normalization would key the array
      by a backing value two enums can share. `$case as int` is the spelling for the number.
- [x] **P12** Calling a member the registry does not hold on a `Core` instance — `$uuid->version()` —
      panics (`lower/expr.rs:2681`, "instance method call … has no resolved target") instead of the
      E0405 a static miss gets. *coretime-probes `u_version`* — the instance path now asks the
      registry exactly as the static one does (`nvs_types::expr::calls::infer_method_call`).
- [x] **P13** Reading a `catch` binding after its clause panics ("undeclared local `$e`") rather than
      being refused by the checker. *ref-errors `catch_scope`* — the clause's binding no longer joins
      what is live after the `try`, so the read is `E0301`; a name assigned before the `try` and reused
      by the clause is untouched.
- [x] **P14** `catch (LogicError | IOError $e)` parses and checks, then fails in codegen with a
      lowering panic on the union clause (exit 1); a property read through the union is E0495.
      *ref-errors; ref30* — `E0245` refuses the clause at parse time, where the type grammar's union
      shows up; the clause is still built from its first class, so the block behind it is checked.
- [ ] **P15** A memory-limit breach inside `try { … } finally { … }` aborts the process with a Rust
      panic in `nvs_array_release` ("attempt to subtract with overflow", exit 127) instead of the
      clean `FATAL`. Without the `finally` it is clean. *refp/fatal/main.nvs*
- [x] **P16** `Core\Env::EOL` panics in `nvs-ir` (exit 101) — `Core\Env` resolves as a class but has
      no members; the `E0319` help text names `Core\Env::mode()`. *php-diff probes* — the registry is
      the whole roster of `Core`, so an unregistered class's member is E0405 and no help text names a
      member that does not ship (`nvs_types::core_lib`'s module doc is the rule).

## Rules stated but not enforced (U)

- [x] **U1** `readonly` is inert: a property (declared or promoted) is written after construction
      and reads back the new value. *probes2 `readonly_write`, `readonly_write_method`; r08* — a write
      from anywhere but the declaring class's own constructor is `E0782` (`rule:classes/lateinit-restrictions`'s contract),
      at all four write spellings; PHP's second half, which admits an initializing write from any
      method of that class, is the divergence the differences page carries.
- [x] **U2** `final` is inert: `final class A {} class B extends A {}` and an override of a `final`
      method both compile and run. *probes p09, p10, r07* — `E0783` refuses the `extends` clause and
      `E0784` the redeclared method, each where the offending declaration is written, as PHP does.
- [x] **U3** `new` on an `abstract` class compiles and runs; calling the bodiless method then dies
      with `FATAL: internal error: a method with no body was called`. `abstract` on a method of a
      non-abstract class is not refused either. *probes p11, p11b, q01* — `E0785` refuses the `new`
      (an interface with it, since neither has instances) while `new static()` stays legal, and
      `E0786` refuses the bodiless method where it is declared.
- [x] **U4** `secret` is refused only by a `Throwable` message (E0422), `Core\Debug` (E0724) and any
      unclassified `string`/`bytes` `Core` parameter (E0401). `echo $pw;`, `"{$pw}"` interpolation
      and `Core\Json::encode($pw)` all print the value — `rule:security/secret-sinks-refuse` lists output among the sinks.
      *probes2 `secret_echo`; types-probes* `E0790` refuses the operand at `echo` and `print`, which
      covers the interpolation because the qualifier spreads to the composed literal, and `E0791`
      refuses `Core\Json::encode`, walking the argument's type so a declared `array<secret string>`
      is refused with the bare value. A written `["token" => $pw]` still reaches neither: an array
      literal with no expectation infers `array<mixed>`, which is `rule:security/secret-qualifier`'s unmodelled container
      axis and is fixed at the literal rather than at either sink.
- [x] **U5** `Core\Secret::reveal` does not exist (E0405), yet the E0422/E0724 help texts tell the
      user to call it. No member returns `tainted` or `secret`; a value is qualified only where a
      declaration spells it. *types-probes* `Core\Secret` is registered —
      `crates/nvs-stdlib/src/secret.rs`'s `reveal` and `revealBytes`, the only rows that write
      `Qual::Reveal`, brought forward because every `rule:security/secret-sinks-refuse` refusal's help text
      already named the call. The mark admits a `secret` argument and the answer drops the
      qualifier by not declaring it; `tainted` still crosses, so `reveal` launders one axis only.
- [x] **U6** `#[Command]` accepts an instance method and an `int` return — `rule:tooling/commands-are-compiled` says static and
      `void`/`uint`. (A positional parameter is *not* required to be `tainted`; that half of the original
      finding was wrong.) *ref-attr `command-*`* `E0789` now refuses both from
      `commands.rs`'s `check_command_shape`, worded from what the declaration did as the `#[Test]` and
      `#[Fixture]` shape refusals are. A method writing no return type at all is left to the diagnostic
      that already refuses that, rather than told it returns the `mixed` it never wrote.
- [ ] **U7** Under `nvs run` `[limits] wall_time` and `max_output` are not enforced: `wall_time = "1s"`
      with an infinite loop ran past 60 s, `max_output = "10"` let 28 bytes through.
      `Core\Fatal::onLimit`'s card lists both. `cpu_time` is enforced, pinned by
      `tests/conformance/error/a-loop-that-allocates-nothing-is-stopped-by-the-cpu-ceiling.nvst`.
      *refp/wall, refp/out*
- [ ] **U8** A memory-limit breach in a **child isolate** is not observed: `[limits] memory = "8M"`
      with a child holding 64 MiB answered `ok = true`. *refp/childmem*
- [ ] **U9** `nvs config check`/`nvs run` do not validate quantities or ceilings: `memory = "12
      bananas"` and `[limits] memory = "1G"` over `[limits.hard] memory = "512M"` both pass and run.
      `tree.rs` says values are refused where sizes are parsed. *php-diff/config probes*
- [x] **U10** The config trust rule (a file writable by another account is refused) is applied by
      neither `nvs run` nor `nvs config check` — `crates/nvs-cli/src/config.rs` reserves it for
      `serve`/`ctl reload`, which do not exist. E0607/W1005 are unreachable in this binary.
- [x] **U11** `password_file` is not materialized under `nvs run`: `Core\Config::get("db.main.password")`
      is `null`, and `config dump` shows the path rather than `<secret>` (secret.rs says `<secret>`).
      The both-set refusal E0608 does fire. The file *was* read — `Snapshot::retype` then rebuilt the
      typed tree out of the merged table, which never held the content, so the value was dropped
      between the resolver and every reader. It is carried beside the table now
      (`nvs_config::secret::Secret`), put back by `secret::apply` on each retype, and read by name:
      `Core\Config::get`/`all` answer it and the dump prints `rule:config/check-and-dump-audit-the-tree-offline`'s `<secret>` row naming the
      file. `dump --toml` still cannot leak it, because the table is still where it never is.
- [x] **U12** A write to a get-only hooked property compiles and is silently unobservable. *q03* —
      `E0787` refuses it from outside the declaring class; inside, it is the backing slot the `get`
      hook reads, which is how such a property holds a value. The differences page carries the row.
- [ ] **U13** `$e->message = "b"` on a throwable is accepted and reads back — the properties are not
      read-only, though a conformance case calls `location` "readonly". *ref-errors `write_message`*
- [x] **U14** `#[Access]` on a method with no `#[Route]` compiles (no stray-marker refusal, unlike
      `#[Query]`/`#[Option]`/`#[Api]`). *ref-attr* `E0788` now refuses it from the same per-method
      walk those three are asked in — `rule:attributes/access-is-a-required-sibling`'s sibling rule read from the other side, and a
      refusal rather than a silence because § 2 keeps the compiler from interpreting `allow`, so the
      route table is the decision's only reader.
- [x] **U15** `nvs test --filter` does not select `#[Test]` methods — `--filter clock` ran all four
      tests; it filters `.nvst` paths only. *ref-probe/t1* The flag now selects on both sides by one
      containment rule (`nvs-cli`'s `runner::selected`), over `Class::method` for a program's tests;
      it is case-sensitive there as it already was over a path, so `clock` selects nothing a class
      or method spells `Clock`.
- [ ] **U16** `#[Test(db: …)]` and `#[Test(server: …)]` are accepted and have no reader in the runner.
- [x] **U17** `array $a = [1, 2];` with no `<T>` is accepted (ground rules: every array declares its
      element type). *php-diff probes*
- [x] **U18** `<>` parses as `!=` — `rule:expressions/one-equality-operator` says `==`/`!=` are the whole set. *ref30* The lexer now
      reports `E0241` at the two characters and still pushes the token `<>` means, so a file that writes
      it reports the rest of its own problems in the same run — the recovery `===` and `!==` already had.
- [x] **U19** `try { … }` with neither `catch` nor `finally` is accepted (PHP refuses it). *ref30* The
      parser reports `E0126` at the `try` keyword, where the missing clause would be written, and still
      builds the statement.
- [x] **U20** `namespace A { class B {…} echo B::f(); }` (the braced form) parses and resolves. *php-diff*
      The parser reports `E0243` at the `{` and parses the block anyway, so the declarations inside it
      still report their own problems in the same run.
- [x] **U21** `Iterator::current()` outside the protocol does not throw on a generator (`rule:iteration/two-interfaces`
      says it does): `0` before the first `advance()`, the last value after exhaustion. *p44*
      It throws `LogicError` now, guarded on `gen#state >= 1` —
      `nvs_ir::lower::generator`'s module doc owns which § 10 class and why.

## Binary and docs disagree on behaviour (D)

- [x] **D1** The time types do **not** implement `Comparable`: `$i < $j` on two `Core\Time\Instant`,
      `Duration`, `Date` or `TimeOfDay` values is `E0411 … does not implement Comparable`, while the
      cards say `compareTo` is "as `Comparable` requires" and spec § 4 says `<`/`>` work directly.
      *coretime-probes `t_instant`, `t_duration_echo`, `t_date`, `t_tod_lt`*
      They order now, and the conformance is read off the `compareTo` row rather than a roster —
      `nvs_stdlib::registry::implements_comparable`, seeded by `nvs_types::core_lib`. `Core\Uri`
      gains it by the same rule (spec § 12). `crates/nvs-stdlib/src/time.rs`'s module doc owns why a
      `Core` class satisfies an interface by member at all.
- [x] **D2** `Core\Time\Duration ==` is identity: `90m == 1h30m` is false; `compareTo` answers `0`.
      Closed: the binary is `rule:expressions/equality-semantics`'s class row, and `Duration::compareTo`'s card now says so,
      naming `$a->compareTo($b) == 0` as the content comparison. *types-probes*
- [x] **D3** `Core\Weekday as int` is zero-based (`Friday` → `4`); the enum card says the cases are
      ordered "as `date("N")`" (Friday = 5). Closed: the card keeps `date("N")` for the **order** and
      states the numbering is not its — `Monday as int` is `0`. *coretime-probes `t_enum_int`*
- [x] **D4** A literal pattern/duration argument is a **compile error** (E0769), not the
      `LogicError`/`ParseError` the cards name: `->format("yyyy-QQ")`, `Duration::parse("30 seconds")`.
      Only a computed argument throws. Closed: the three cards on `rule:expressions/intrinsic-list-is-closed`'s roster —
      `Duration::parse`, `DateTime::format` and `Core\Time::parse` — name `E0769` and say only a
      computed argument reaches the throw, and the `DateTime::format` card names `Date::format` and
      `TimeOfDay::format` as the off-roster half `nvs_types::intrinsics` owns.
      *coretime-probes `t_format_bad_literal`, `t_parse_errors`*
- [ ] **D5** `Core\IO::read("missing.txt")` under a valid `fs.read` grant throws the **capability**
      `RuntimeError` ("needs the capability fs.read for missing.txt, which is not granted"), not the
      card's `IOError`; `"./missing.txt"` gets the `IOError`. Likewise `write("copy.txt", …)` with
      `write = ["."]` is refused while `"./copy.txt"` succeeds. Cause: `capability.rs:257`
      `resolved()` — `Path::new("copy.txt").parent()` is `""`, which never canonicalizes.
      *refp/fs; coretime-probes*
- [ ] **D6** `Core\Router::url` with a **computed** name throws for every name, declared or not:
      "no route is named 'u'. The compile-time route table is not built yet (`rule:routing/table-is-opt-in`)". The
      card says only an unknown computed name throws. *ref-attr `route-url-computed-name`*
- [x] **D7** `Core\Json::decodeAs<T>` was a **FATAL** for an **enum** field (encode always handled
      one). All three halves are closed: a class field erases to `CodecTy::Class` carrying its
      label, an `array<T>` to `CodecTy::List` carrying its element's wire type beside that label,
      and an enum to `CodecTy::Enum` carrying its declared backing values on `CodecField::cases`.
      The enum's *name* never travels — `rule:enums/representation` leaves a case indistinguishable from the
      integer behind it — so the decode is a membership test producing that integer, and a case
      name on the wire is refused exactly as any other non-case is. `nvs-codegen` resolves a class
      label to a descriptor once every class of the unit is defined, and `nvs_stdlib::json`'s
      `decode_nested`/`decode_list`/`scalar` report under `rule:core-classes/derive-reports-every-field`'s dotted path —
      `address.city`, `tags.3`, `authors.0.name`, `seen.2`.
      *ref-attr `derive-array-and-enum-field`, `derive-decode-nested-and-float`*
- [x] **D8** A promoted constructor parameter is not a `#[Json\Derive]` field — a class with only
      promoted state is refused E0758; `rule:core-classes/derive-attribute`'s own example uses promotion. `nvs_types::derive`
      now reads both spellings of a declaration through one view, in the members' own order;
      `crate::layout` had given a promoted parameter a slot for some time and only its module doc
      still said otherwise. *`derive-promoted-ctor`*
- [x] **D9** Private properties are JSON fields under `#[Json\Derive]` (`{"name":"a","n":1}` for a
      `private int $n`). *`derive-private-property`*
      Correct, and decided in writing: `rule:core-classes/derive-field-list`'s first bullet says the field list is the declared
      property list, "private ones included", because visibility answers who may *reach* a value and a
      wire format is not that question. The refusal this finding wondered about is the failure it would
      cause, not prevent — a codec that dropped a field the day it gained a `private` changes a document
      every consumer already parses, with nothing at the declaration to say so, and that silent break is
      what § 1's written opt-in exists to remove. `nvs_types::derive::codec_field` reads a declaration's
      modifiers for `lateinit` alone and never for a visibility, so encode and decode are blind to it by
      construction rather than by an omission; decode needs no separate answer, since § 2 makes it an
      ordinary `new` and a constructor assigns a private property like any other. The docs say it in
      one place already — `docs/reference/lang/90-attributes.md`'s `#[Core\Json\Derive]` section, whose
      field bullet reads "whatever their visibility" — and `docs/reference/core/Json.md` now names it on
      the `decodeAs<T>` sentence a caller actually reads first, where the same page had also outlived
      `rule:core-classes/derive-attribute`'s `array<U>` top level. *`json-derive-encodes-declared-fields`*
- [x] **D10** `#[Api]` fields `tags`, `security`, `errors`, `example` are checked but absent from the
      `nvs build --openapi` document. *ref-probe/api2*
      A `nvs_types::Route` carries all four: `check_api` hands back what each of its four walks
      accepted, so a value § 2 refused reaches no row and no document. The emitter writes `tags` as
      written, `security` as one 3.1 security requirement per name with no scopes, an `errors` entry as
      a response of its own described by its class, and `example` — folded to its `rule:attributes/retrieval-folds-while-checking` constant
      while the imports are still in reach — beside the schema it is an example of. § 1's `200` outranks
      an `errors` entry naming it. What is left is `crates/nvs-cli/src/openapi.rs`'s gap 3, which is not
      this finding: nothing in the tree declares what a named scheme *is*, so the document names schemes
      it does not define.
- [ ] **D11** `spawn script Class::method(...)` as the operand is refused (`E0401: expected string,
      found callable`); spec § 2 and `rule:security/isolate-shares-nothing` describe it as available. *refp/spawn/method.nvs*
- [x] **D12** `spawn script … with(args: …)` is accepted but there is no reader: `Core\Script::args()`
      is E0405, and the parser's own hint (`$_ARGS` → `Core\Script::args()`) names it. *refp/spawn/args.nvs*
      `Core\Script` is a registered class now and `args()` is its one row. It answers `mixed` rather
      than `rule:core-classes/script-args`'s original `array<mixed>` — `args:` narrows nothing at the call site — and
      `null` rather than an empty array for a child spawned without the option; § 6 states both, and
      `nvs_stdlib::script`'s module doc owns why.
- [x] **D13** `spawn script` default `output` is `'capture'`, not inherit. *refp/spawn/main.nvs*
- [x] **D14** A child with no top-level `return` answers `value = int(1)`, not `null`. *refp/spawn/noret2*
      The finding is right and ADR 0006 § *Decision* already said so; `1` is `require`'s, and the
      entry frame is not a `require` — `nvs_ir::lower::ScriptRole` is where the two now part.
- [x] **D15** `Core\Config::get("mode")` answers `null` and `set("mode", …)` returns `false`;
      `mode.default` works for both — `rule:config/a-program-may-read-and-flip-its-mode` spells the bare `mode`. The spelling half is not a
      bug: § 4 says `mode.default`, and a bare `mode` is a limit's name and nothing else
      (`nvs_config::request`'s module doc). What was missing is what the flip *does* — § 4's last
      bullet re-derives § 3's five defaults and § 5's ceiling bounds it, neither of which existed.
      `nvs_config::mode` is now the one home of that table and of the two-value order, and
      `Request::flip_mode` applies both.
- [x] **D16** `Core\Arr::from` refuses the `Core` collections although they are `Iterable`. Two
      halves, both in `nvs-types`: the nominal check gated on `nvs_hir`'s class graph, which holds
      what a *program* declared and never a `Core` class's seeded `implements`
      (`crate::expr::assign::class_satisfied` owns the two-roster rule); and what a class fixed for
      an interface is written in its own type variables, which is the `array<K>` leak.
      `crate::generics::with_class_args` is now that substitution's one home and its three callers
      say so. *q09b, r04b*
- [x] **D17** `Core\ObjectSet::union`/`intersect`/`diff` lose the element type. `CoreTy::Instance` of
      a *generic* class now interns at that class's own type variables, so the receiver's arguments
      reach the answer through the substitution every other member's `T` already went through
      (`nvs_types::core_lib`'s `lower`). The parameter tightens with it: `union` on an
      `ObjectSet<Pin>` receiver refuses an `ObjectSet<Tag>`, as § 9's signature reads.
      *ref-core-probes2*
- [x] **D18** `object` is not fully opaque: a method call through `object` is refused (E0477) but a
      property read compiles and resolves at run time ("`A` has no field `nope`" on a miss); the
      diagnostic cites `rule:types/erased-member-access`, so this may be intended. *r02, s01*
- [x] **D19** A comparison (`$e == E::A || $e == E::B`) does not narrow an enum value to the
      case-union type; only `as E::A|E::B` does. *q06, r03b*
- [x] **D20** An empty shape `{}` is satisfied by every attached attribute literal, so a bare marker
      `#[Audited]` (`type Audited = {}`) is ambiguous (E0728) on a class with any other attribute.
      Correct, and it is the price `rule:attributes/structural-retrieval` chose knowingly: retrieval is structural so that there
      is no second namespace of attribute-kind names for unrelated frameworks to collide in, and once
      the ask is a shape, `{}` asks for *any* attached literal — width subtyping admits no narrower
      reading of a shape with no fields. E0728 is then the honest answer rather than a gap: `get`
      promises at most one, two literals satisfy the ask, and the attached list is static, so the
      question is settled at the call site instead of by whichever attribute a test run happened to see
      first. A marker meant to be retrieved earns a field of its own. Both `Core\Attributes` cards now
      say so, beside the chapter bullet that already did.
- [x] **D21** A payload holding a class constant or enum case can be declared but not retrieved
      (E0731) with `Attributes::get`/`all`. Closed: the payload is folded through the scope it was
      *written* in, so a class constant, an enum case and `Foo::class` each fold to what a read of the
      same name inlines (`nvs_types::retrieval` owns why the attach site's scope and not the
      retrieval's). `E0731` is left for a constant whose own declaration folds to nothing.
- [x] **D22** `inout` accepts only a local: `M::bump(inout $a["k"])` is E0439, and the refusal is
      permanent — the `yet` is gone (`nvs_types::expr::args::check_inout_arg` owns why). *ref30*
- [x] **D23** `rule:types/anonymous-function-self-name`'s named-closure recursion (`fn fact(int $n): int => … fact($n - 1)`)
      parses, but the recursive call resolves as a free function (E0320). *ref30*
- [x] **D24** `1.0 / 0` answers `INF` without throwing; `rule:types/arithmetic`'s `/ 0` row is the integer one.
      *ref30*
- [ ] **D25** A property default of `null` on a `?T` property is refused (E0472, "must be a `int|null`
      constant … not a constant of the declared type") — `null` is exactly such a constant. *probes2
      `nullable_default_int`, `nullable_default_class`*
- [x] **D26** `continue` (level 1) inside a `switch` inside a loop continues the enclosing loop — the
      documented Novis choice, but PHP acts as `break`; noted so the crosswalk row stays deliberate.
- [x] **D27** A shebang `#!` first line opens code mode, per `rule:tooling/shebang-opens-code-mode`, and is trivia rather than
      output. `nvs_syntax::lexer`'s `Lexer::new` owns why it is lexed as the `#` comment it already is
      instead of skipped before lexing, and `E0009` names an `<?nvs` in such a file. *php-diff probes*
- [ ] **D28** The on-disk compile cache is unwired: `cache.rs` and `[cache] dir` exist, nothing in
      `nvs-cli/src` outside `cache.rs` references it, and `nvs run` compiles fresh every time.
- [ ] **D29** `await`'s result renders as `Core\Script\Result#1` in `Core\Debug::render`, while
      `isolate.rs` types it as an anonymous shape.
- [x] **D30** Diagnostics name classes that are empty or absent in this build: E0211's help says
      `Core\Request`, `Core\Server`, `Core\Cli`; each resolves but has no members and none is in
      `nvs meta --json`. A help text now names a `Core` member only where the registry holds one
      (`nvs_syntax::parser`'s `superglobal_replacement` owns why, and it is the rule for any help).
- [ ] **D31** `Core\Cli\Text` has no members and no constructor (module doc: deliberate), so
      `Core\Str::length($text)` is refused while `echo`, `.` and `as string` accept it; `rule:tooling/terminal-output-is-a-sink`
      gives it `plain`/`styled`/`+`.
- [x] **D32** `Core\Validate::isEmail` does not launder — `string $s = $in;` after a `true` answer is
      still E0401 (may be intended; noted because a reader expects a validator to launder).
- [x] **D33** `assertSame($uintValue, 2)` is `E0401: expected uint, found int` — a literal beside a
      generic parameter does not adapt. Closed: it is placed against the substituted parameter type
      (`nvs_types::expr::args::check_generic_args` owns the rule).
- [ ] **D34** `Core\Arr::sort` on an `array<decimal>` throws at run time: "no natural order for tag
      10 against tag 10: two numbers, two strings, two bools or two nulls have one" — `decimal` is a
      number and the checker accepted the call. Found by the blind proof's CSV task. *probes4 `sort_decimal`*
- [x] **D35** There is no typed decode of a JSON **array**: `Core\Json::decodeAs<array<U>>` is E0465
      ("builds a class, and `array<U>` is not one"), so a list of derived objects is read as
      `array<mixed>` and converted element by element. Found by the blind proof's JSON task.
      Closed: `array<C>` is the list form — `nvs_types::expr::args::written_class_of` records the
      element class and a flag, and `nvs_stdlib::json::decode_each` runs the § 5 decode once per
      element. *probes4 `decode_list`*

## Named in the docs, absent from the registry (M)

- [x] **M1** `Core\Task::afterResponse` (spec § 19) — only `all` and `map` exist. Closed:
      `nvs_stdlib::task`'s third row registers the closure and `nvs_runtime::deferred` runs it once
      the request's own frame has returned, which is `rule:concurrency/after-response-outlives-the-connection`'s "after the response" on a host
      that has no response. Only the request's own task may register — a child's queue would be
      drained by nobody — and that module's known gaps are § 7's `max_concurrent` and an isolate's
      own drain, both of which need a host that holds more than one tree.
- [ ] **M2** `Core\Fatal::onUncaughtThrow` (`rule:errors/on-uncaught-throw`) — only `onLimit` exists.
- [x] **M3** `rule:testing/test-attribute`'s wider assertion roster (`assertStartsWith`, …). Closed:
      the roster's one home is [docs/spec/01-core-library.md](../spec/01-core-library.md)'s Part II
      class table, and `assertStartsWith` is on it nowhere — `rule:testing/assertions-are-typed` names
      it once as an example of what `nvs-lsp` ranks by subject type. Every member that table gives
      `Core\Test` is registered in `crates/nvs-stdlib/src/test.rs`: `assertContains`,
      `assertMatchesInline`, `request`, `double<T>`/`partial<T>`, `assertCalled`/`assertNeverCalled`
      and `assertCompletes` among them.
- [x] **M4** `Core\Test\Failure` and `RecursionError` appear in no member card's `errors` list, only
      in the exception tree (behaviour verified by probe). Closed, one half stale and the other *no
      card owes it*: nine of `Core\Test`'s cards name `Core\Test\Failure` in `errors`
      (`crates/nvs-stdlib/src/test.rs:314` is the first), so that half was true only before the
      assertion roster carried cards at all. `RecursionError` has exactly one raise site —
      `nvs_runtime::ctx::nvs_stack_check`, the depth guard every non-leaf function entry runs — and no
      member body reaches it. A card's `errors` is what that body throws, so a class raised *by the
      call* rather than by a member belongs to every card equally, which is to say to none of them; the
      exception tree is its one home.
- [x] **M5** `Core\Uuid` has no `version()`; the cards' `errors` never name it. (Calling it is P12.)
      Closed as *the member is not in the spec's roster*: calling it is now E0405 rather than a panic,
      and adding a member is a spec question rather than a finding.
- [ ] **M6** `Core\Command::run`/`help`/`completions` (`rule:tooling/commands-are-compiled`) do not exist; the command table
      is built and checked, and `Core\Program::implementing` is the only reader.
- [ ] **M7** `nvs serve`, `nvs fmt`, `nvs convert`, `nvs lsp`, `nvs ctl` are unrecognized subcommands.
- [x] **M8** `Core\Secret`, `Core\Taint`, `Core\Log`, `Core\Env`, `Core\Cli`, `Core\Request`,
      `Core\Server`, `Core\IO`, `Core\Html` resolve as names in diagnostics or the crosswalk but have
      no registry rows. Part D's *dropped* rows still cite `Core\Html::escape` and `Core\IO::within`.
      The name still resolves — nothing under `Core\` needs a declaration — but every member of one is
      E0405, so a reader is told at the reference rather than at a panic. The crosswalk's own rows are
      item 35's.
- [x] **M9** `Core\Env::mode()`, `$_ARGS`/`Core\Script::args()` are named by diagnostic help texts
      and do not exist. E0211's table now cites `rule:statements/no-host-populated-variables`'s map rather than restating a row of it,
      and E0319 names `Core\Math::PI`, which ships.
- [ ] **M10** The registry cards cite ADR numbers inline in 33 places ("`rule:core-classes/regex-two-tiers`'s two engines",
      "`rule:types/bytes`'s default unit") — meaningless to the reference's readers. `bun nv reference`
      strips the parenthesised form `(`rule:classes/comparable`)`; the inline ones need rewording in the cards.

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
