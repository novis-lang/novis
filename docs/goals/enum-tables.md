---
looper: 1
title: "an enum is a full key: a match covers every case, a constant holds cases, and an array is keyed by an enum"
summary: "Reached when a match over an enum that misses a case and has no default does not compile, a class constant and a property default hold enum cases, as int of a case, other constants and Foo::class inside arrays, array<E, T> types an array whose keys are the cases of the enum E, Core\\Arr keeps and returns those keys, the editor offers the cases where an enum is expected and a quick fix adds the arms a match is missing, and the rules, the reference, the examples, the feature proofs and the website say so."
context: [docs/rules/enums/no-class-machinery.md, docs/rules/types/arrays.md, crates/nvs-types/src/ty.rs]
stages:
  - id: 1
    title: a match over an enum covers every case
    status: done
    context: [crates/nvs-types/src/expr/mod.rs, crates/nvs-types/src/expr/operators.rs, crates/nvs-ir/src/lower/expr.rs, docs/rules/expressions/a-match-has-at-least-one-arm.md]
  - id: 2
    title: "a class constant and a property default hold enum cases, as int of a case, other constants and Foo::class inside arrays"
    status: done
    context: [crates/nvs-types/src/defaults.rs, crates/nvs-types/src/expr/members.rs, crates/nvs-ir/src/lower/call.rs, crates/nvs-diagnostics/src/code/types_e07.rs]
  - id: 3
    title: "array<E, T> types an array keyed by the cases of an enum, and the checker holds every key to it"
    status: done
    context: [crates/nvs-syntax/src/parser/ty.rs, crates/nvs-types/src/expr/literals.rs, crates/nvs-types/src/expr/assign.rs, crates/nvs-types/src/expr/iteration.rs, crates/nvs-diagnostics/src/code/types.rs]
  - id: 4
    title: "Core\\Arr keeps and returns enum keys, and as array<E, T> checks each key"
    status: done
    context: [crates/nvs-stdlib/src/arr.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-ir/src/lower/convert.rs, crates/nvs-stdlib/src/json.rs]
  - id: 5
    title: "the editor offers the cases where an enum is expected, and a quick fix adds the arms a match is missing"
    status: open
    context: [crates/nvs-lsp/src/completion.rs, crates/nvs-lsp/src/actions.rs, docs/rules/ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows.md, docs/rules/ide/completion-is-asked-where-a-spelling-ends.md]
  - id: 6
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [docs/examples/lang/enums, website/src/data/changes.json]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: covers
    stage: 1
    name: "a match over an enum or a nullable enum with no default arm does not compile while a case is missing, and a match over every case lowers no fallback throw"
    run: [cargo, test, --test, enum_tables]
    tests: [a_match_over_an_enum_that_misses_a_case_does_not_compile, a_match_with_a_default_arm_needs_no_other_case, a_match_over_a_nullable_enum_needs_a_null_arm, an_arm_that_is_not_a_case_covers_nothing, a_class_constant_that_folds_to_a_case_covers_it, a_match_over_mixed_or_a_union_is_unchanged]
    timeout: 1800
  - id: no-fallback
    stage: 1
    name: "a match over every case of a statically typed enum lowers no fallback throw"
    run: [cargo, test, --lib, a_match_over_every_case_lowers_no_fallback_throw]
    tests: [a_match_over_every_case_lowers_no_fallback_throw]
    timeout: 1800
  - id: folds
    stage: 2
    name: "a class constant and a property default fold an enum case, as int of a case, another class's constant and Foo::class, alone and inside arrays"
    run: [cargo, test, --test, enum_tables]
    tests: [a_constant_array_holds_enum_cases, a_constant_array_is_keyed_by_a_case_as_int, a_constant_array_holds_another_class_constant_and_a_class_name, a_property_default_array_holds_enum_cases, a_constant_that_still_cannot_fold_says_what_to_write]
    timeout: 1800
  - id: keyed
    stage: 3
    name: "array<E, T> takes only cases of E as keys, refuses an append, gives E to a foreach key, reads as array<T>, and refuses a key type that is not an enum"
    run: [cargo, test, --test, enum_tables]
    tests: [an_array_keyed_by_an_enum_takes_only_its_cases, an_int_a_string_or_another_enum_key_is_refused, an_append_to_an_enum_keyed_array_is_refused, a_foreach_key_over_an_enum_keyed_array_is_the_enum, an_enum_keyed_array_is_read_as_a_string_keyed_one, the_key_type_of_an_array_must_be_an_enum, a_literal_with_case_keys_and_no_target_names_the_keyed_type, a_constant_of_an_enum_keyed_array_folds]
    timeout: 1800
  - id: arr
    stage: 4
    name: "Core\\Arr members keep an enum key, return it as the enum, group by a case, and as array<E, T> checks every key"
    run: [cargo, test, --test, enum_tables]
    tests: [a_member_that_keeps_keys_keeps_the_enum_key, keys_and_first_key_return_the_enum, flip_turns_enum_keys_into_enum_values, group_by_a_case_returns_an_enum_keyed_array, a_string_keyed_array_converts_with_as_and_checks_each_key]
    timeout: 1800
  - id: cases
    stage: 4
    name: "the conformance cases for the covering match, the folded constants and the enum-keyed array are green"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/a-match-over-an-enum-that-misses-a-case-does-not-compile.nvst, --case, tests/conformance/enum/a-match-over-every-case-needs-no-default.nvst, --case, tests/conformance/enum/a-constant-holds-enum-cases-inside-an-array.nvst, --case, tests/conformance/enum/an-array-keyed-by-an-enum-gives-its-keys-back-as-cases.nvst, --case, tests/conformance/reject/an-enum-keyed-array-takes-only-its-own-cases.nvst]
    timeout: 1800
  - id: editor
    stage: 5
    name: "the editor offers an enum's cases at a match arm, a keyed subscript, a keyed literal's key and an enum parameter, a quick fix adds a match's missing arms, and rename, hover and highlighting reach the new forms"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/completion/a-match-arm-over-an-enum-offers-the-cases-not-yet-handled.lspt, --case, tests/lsp/completion/a-subscript-of-an-enum-keyed-array-offers-its-cases.lspt, --case, tests/lsp/completion/a-key-of-an-enum-keyed-literal-offers-the-cases-not-yet-written.lspt, --case, tests/lsp/completion/an-argument-at-an-enum-parameter-offers-its-cases.lspt, --case, tests/lsp/completion/the-key-of-an-array-type-offers-the-enums.lspt, --case, tests/lsp/actions/a-match-missing-a-case-is-offered-its-arms.lspt, --case, tests/lsp/rename/renaming-a-case-renames-it-inside-a-constant-table-and-a-keyed-literal.lspt, --case, tests/lsp/hover/an-enum-keyed-array-shows-its-key-type.lspt, --case, tests/lsp/highlight/an-enum-keyed-array-type-colours-both-arguments.lspt]
    timeout: 1800
  - id: proofs
    stage: 6
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, enum-tables]
    timeout: 1800
  - id: close
    stage: 7
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, enum-tables]
    timeout: 900
---

# an enum is a full key: a match covers every case, a constant holds cases, and an array is keyed by an enum

## Why

A user reported that a label for each enum case can only be written as a `match` in a static method.
Every table form is refused:

| Written | Today |
|---|---|
| `[Priority::Low => 'low', …]` | `E0434`: an enum case is not an array key (`crates/nvs-types/src/expr/literals.rs:1633`) |
| `const array<string> LABELS = [Priority::Low as int => 'low', …]` | `E0792` at the read: the constant folder has no arm for `as` or an enum case inside an array (`crates/nvs-types/src/defaults.rs:953`, `crates/nvs-types/src/expr/members.rs:432`) |
| `private static array<string> $labels = [ … as int => … ]` | `E0472`: a property default takes a case alone, never inside an array (`crates/nvs-types/src/defaults.rs:469`) |

And the `match` the rule recommends is not checked either. A `match` over an enum that misses a case
compiles with no diagnostic, and throws `LogicError` ("no `match` arm matched the subject") when the
missing case arrives (`crates/nvs-ir/src/lower/expr.rs:2275`). The package diff already reports a new
case as breaking "a `match` over every case" (`crates/nvs-cli/src/package/diff.rs:613`), a check the
checker never makes.

The user compared other languages. Every statically typed language whose enum is a plain value takes
it as a typed map key: C# `Dictionary<E, V>`, Go `map[E]V`, Swift `[E: V]`, Rust `HashMap<E, V>`. The
map's key type is what stops a case of another enum, so the key stays a plain integer and costs
nothing. Only the untyped forms have the trap `E0434` exists for: Python's `IntEnum` finds a row of
another enum with the same number, and TypeScript gives the key back as a string. So the user
decided, on 2026-11-24, to take all three in one goal: the covering `match`, the folded constants, and
an array typed by its enum key. Grouping rows by a state, a setting per case and a permission per role
are the shapes it serves.

## Standing decisions

**A `match` over an enum covers every case (stage 1).** When the subject's static type is an enum `E`,
or `?E`, and the `match` has no `default` arm, every case of `E` must be named by an arm, and `null`
too for `?E`. A missing case is a new error at the `match` keyword, listing every missing case. It is
the next free code after `E1008`, checked when added, `E_MATCH_MISSES_A_CASE`. Draft text, held to
AGENTS.md § *Text an end user reads*:
- message: "this `match` does not handle `Priority::High`" (each missing case, joined with "and")
- help: "Add an arm for each missing case, or add a `default` arm."

An arm condition covers a case when it is `ExprInfo::EnumCase` of `E` (`crates/nvs-types/src/defaults.rs:672`),
or a class constant that folds to one (stage 2 makes more of them fold). Any other condition covers
nothing, so the `match` needs a `default`. A subject of `mixed`, of a union with anything but `null`,
or of `bool` (`match (true)`) is unchanged. The check sits in the `ExprKind::Match` arm of `infer_kind`
(`crates/nvs-types/src/expr/mod.rs:928`).

**The fallback throw goes where it cannot run.** A statically typed `E` is always one of its cases,
because every write is checked (`rule:enums/representation`). A `match` over every case therefore
lowers no no-match branch (`crates/nvs-ir/src/lower/expr.rs:2268`). The IR test counts the blocks
(`rule:programs/less-work-is-taken`). Default if the lowering cannot see the checker's verdict: an
`ExprInfo` flag on the `match` expression, set by the checker.

**Every `.nvs` and `.nvst` in the tree the new error reaches** gets the missing arms or a `default`,
in the stage that adds it. A `.nvst` whose expected output is the `LogicError` is a test of the run-time
throw: it changes its subject to `mixed`, so the throw is still tested.

**What stage 1 built.** The code is `E1016`, `E_MATCH_MISSES_A_CASE`, the first free one after
`E1008`. The check is its own module, `crates/nvs-types/src/expr/cover.rs`, and
the verdict is `ExprInfo::MatchCoversEveryCase`, recorded at the `match` keyword's span alone
(`cover::match_keyword_span`), which no other expression has. The lowering does more than drop the
throw: it does not compare the last label either, and jumps to its arm, since it is the only value
left. Missing cases are listed by value, because the enum table keeps no declaration order. A
`Core` enum case is named the way the message's reader writes it after a `use`, such as
`ExitReport::Reason::Finish`.

**Declined for stage 1.**
- **A `switch` over an enum.** It is a statement, and a case it does not name runs nothing, which is a
  valid program.
- **A warning on a `default` arm when every case is named.** A `default` that catches a future case is
  a choice the author makes.
- **A warning instead of an error.** A closed type is checked, and a missing case is the same kind of
  mistake as a missing return.

**Constants hold enum cases (stage 2).** `eval_const_value` (`crates/nvs-types/src/defaults.rs:921`)
and `fold_const_array` (`:953`) fold, alone and inside an array at any depth:
- an enum case, kept as a case of its enum, so an element of `array<E>` is a case;
- `$case as int` and `as uint` where `$case` folds to a case of an enum with that backing, as its
  integer;
- another class's constant that folds, and `Foo::class`.

`ConstArg` (`crates/nvs-types/src/defaults.rs:101`) gains what it lacks for these, and
`emit_const_arg` (`crates/nvs-ir/src/lower/call.rs:699`) emits each into an array element. A case
inside an array is written with its enum tag (`rule:enums/representation`). `eval_property_default`
(`crates/nvs-types/src/defaults.rs:444`) takes the same arrays, so a static or instance property
default is a table too. `E0792`'s help (`crates/nvs-types/src/expr/members.rs:463`) and card
(`crates/nvs-diagnostics/src/code/types_e07.rs:1546`) drop "cannot yet" and name only what is still
refused. If nothing is left that reaches it, the code is retired and never reused. A cycle between two
constants stays an error. The session reads how the folder finds one today and keeps it.

**`array<E, T>` (stage 3).** An array type takes a second, optional parameter in front: `array<E, T>`,
where `E` must be an enum type, file-scope or class-scoped. Every other key type, `array<string, T>`
too, is a new error after stage 1's, `E_ARRAY_KEY_TYPE_NOT_ENUM`. Its help says that keys are strings
already, so `array<T>` is the way to write that.
- **Representation (decided in stage 3: the key field, not a variant).** `Ty::Array(elem, key)`,
  where `key` is `None` for string keys and the `Ty::Enum` id of `E` for `array<E, T>`. The AST is
  `TypeAtom::Array(elem, key)` in the same order. Why not `Ty::EnumArray`: a new variant is missed
  by every non-exhaustive reader (`if let Ty::Array(..)`, `matches!`, `_ =>` arms), and the qualifier
  walks in `quals.rs` are such readers. A `secret` or `tainted` element inside an `array<E, T>` would
  go unseen there (security). With a field, a reader that ignores the key reads `array<E, T>` as the
  `array<T>` it is assignable to, which is sound. Only the sites that read, write or compare keys
  read the field. Every walk that rebuilds an array type passes the key on with
  `TypeInterner::array_keyed`. `array_element` and `array_enum_key` are the helpers.
- **Storage does not change.** A key is stored as today: a position in a packed list, a decimal
  string otherwise. No enum identity is kept on the array at run time. **Memory:** nothing per array
  and nothing per entry.
- **A key read back as `E`** is the stored key as an integer, made a case with no check, because
  every write was checked. A packed position costs nothing. A decimal key is parsed, with no
  allocation.
- **What stage 3 built for the `foreach` key.** `lower_enum_key_at` (`crates/nvs-ir/src/lower/convert.rs`)
  reads the key with `ArrayKeyAt`, parses it with the existing `StrToInt`/`StrToUint` helper, releases
  the string and `Reinterpret`s the integer to `Ty::Enum`. The parse carries an error edge no run
  takes, as the slot add in `lower_foreach` does, since codegen requires one on every helper call.
  A new runtime entry that reads a slot's key as an integer, so a packed position skips the string,
  is deferred: it needs an IR instruction, codegen and runtime, and only a measured cost would earn it.
- **Writes.** A key must be a value whose static type is `E`. An `int`, a `string`, a case of another
  enum and a `mixed` key are `E0434`, with a message naming `E`, and the help says to convert with
  `as E`. `$a[] = $v` is a new error: an enum-keyed array has no next key.
- **Reads.** `$a[$k]` holds `$k` to `E` in the same way. An absent key throws, as today.
- **A literal.** `[Priority::Low => 'low']` at a target typed `array<Priority, string>` is checked
  against it. With no target type, `E0434` stays, and its help names `array<Priority, string>`. A
  literal is never inferred (`rule:types/arrays`).
- **`foreach`.** `foreach_key_ty` (`crates/nvs-types/src/expr/iteration.rs:439`) gives `E` for an
  enum-keyed array, and `E0723` (`:535`, and `crates/nvs-ir/src/lower/control.rs:994`) takes a key
  declared as `E` there.
- **Subtyping.** `array<E, T>` is read wherever `array<T>` is expected, with its keys as their decimal
  strings, so every `Core\Arr` member that takes `array<T>` takes it. The array is a copy-on-write
  value, so this is sound for the reason element covariance is (`assignable`,
  `crates/nvs-types/src/expr/assign.rs:262`). The element type stays covariant and the key type is
  invariant. The other direction is only `as array<E, T>` (stage 4).
- **Constants.** `const array<Priority, string> LABELS = [Priority::Low => 'low', …]` folds, since
  stage 2 folds a case as a key.

**`Core\Arr` and conversions (stage 4).**
- A member whose result keeps the subject's keys (`filter`, `map`, a sort with `preserveKeys`, and
  every other such member) returns `array<E, U>` for an `array<E, T>` subject. The signature
  mechanism, `CoreTy::Var` (`crates/nvs-stdlib/src/registry.rs:589`), gains a key variable bound to
  the string key or to `E`.
- **What stage 4 built for keeping keys.** Not a `CoreTy` variant: about forty walks over the
  registry's types match `CoreTy::Array` without being exhaustive, the taint walks among them, and
  each would miss a second array form (the stage-3 reason). Instead the roster
  `nvs_stdlib::arr::KEEPS_KEYS` names the members by symbol, and `nvs_types::core_lib::keep_keys`
  keys their subject and result by the variable `ARRAY_KEY_VAR`. An `array<E, T>` subject binds it,
  and `generics::substitute` keeps a key only when it substituted to an enum, so a string-keyed
  subject gives the plain `array<T>`. `slice`, `reverse` and `sort` keep keys only under
  `preserveKeys`, so they stay `array<T>`: a type cannot follow an option's value, and
  `as array<E, T>` converts the result back.
- **`as array<E, T>` (built in stage 4).** `nvs_to_enum_keyed` runs after the element walk, with a
  list of the enum's cases the IR builds per conversion. A list's key is its position, and a map's
  key must be the case's decimal exactly, so `"08"` is not the case `8`. A miss throws
  `RuntimeError`, or gives `null` under `as ?`.
- `keys`, `firstKey`, `lastKey`, `keyOf` and `findKey` return `E` or `array<E>`, and a callback's key
  parameter is `E`. **Soundness:** the runtime gives every key as a string, so the key variable
  `KEY_VAR` must not bind `E` until both halves exist. A returned key is converted in the IR after
  the call (`lower_enum_key_at` for `?E`, and a walk for `keys`'s `array<E>`). A callback's key is
  built in `arr.rs` as the integer when the callable's parameter check at the key position is an
  enum's `SlotCheck::Values`, so an unproven callable's case check still runs, and a proven one was
  proven against `E`.
- **What stage 4 built for returned and callback keys.** The roster `nvs_stdlib::arr::ENUM_KEYS`
  names the members whose key follows the subject. `core_lib::keep_keys` keys their subject alone by
  `ARRAY_KEY_VAR`, and `check_generic_args` binds `KEY_VAR` to `E` through
  `core_lib::enum_key_of_subject` when the argument at parameter 0 is an `array<E, T>`. An enum key
  is never tainted. A returned key (`GIVES_KEYS_BACK`) is converted after the `Core` call in
  `lower_core_call` by one new helper, `Helper::KeyAsCase` (`nvs_key_as_case`), and not by an IR
  parse: the result of `?E` is a tagged value, and the helper takes `null`, one key and a list of keys
  alike. `keys` builds a second list of scalars. A callback's key goes through `KeyArg` in `arr.rs`,
  read once per walk with `nvs_runtime::callable_param_enum`, and `SlotKey::as_case` makes the case.
  A key that is not the exact decimal is handed on as its `string`, and the parameter's check throws.
  A callback whose key is `mixed` still gets the `string`. `Core\Task::map` is not on the roster, so
  its key stays `string`.
- `flip` of `array<E, string>` returns `array<E>`: the strings become the keys and the cases the
  values. `flip` of an `array<E>` returns `array<E, string>`.
- **What stage 4 built for `flip` and `groupBy`.** The roster `nvs_stdlib::arr::CASE_KEYS`.
  `core_lib::case_keys` writes the value that becomes a key (`groupBy`'s callback result, `flip`'s
  subject element) as the variable `CASE_KEY_VAR`, and keys the result by it. `flip`'s result
  element is `ARRAY_KEY_VAR`, the subject's key. Both are `type_bounds` of the signature, bounded by
  the type the row writes, and `check_generic_args` keeps a binding only when it is exactly an
  enum (`core_lib::binds_only_an_enum`). So `E|string` keys by strings, and a `float` bucket is
  still refused. A deferred `fn` literal is placed against the bound. The run time needed nothing
  for a case bucket: `Value::tag` reads a case as its integer, so `key_bytes` writes its decimal.
  `flip` is on `GIVES_KEYS_BACK`, and `nvs_key_as_case` now keeps the keys of an array of keys.
- **Declined: `groupCounts` keyed by `E`.** Its `by` callback is in an options bag, and a bag binds
  no variable (`check_generic_args` checks it last, against the substituted type). Binding from it
  would check the bag twice, or need a typed `by` signature, which is a second change to the row.
  The run time counts a case under its decimal already, so `as array<E, uint>` converts the result.
- `groupBy` (`crates/nvs-stdlib/src/arr.rs:178`) and `groupCounts` (`:719`) with a callback that
  returns `E` return an array keyed by `E`.
- `as array<E, T>` and `as ?array<E, T>` check that every key is a case of `E`, O(n), beside the
  element walk (`crates/nvs-ir/src/lower/convert.rs:1651`). A key that is not a case throws, or gives
  `null`.
- **`Core\Json`.** An enum key is written as its number, as a string key `"0"` is today
  (`crates/nvs-stdlib/src/json.rs:1289`). A decoded object becomes enum-keyed with `as array<E, T>`.
  `Core\Serialize`, `Core\Cache` and sessions keep the stored keys, and the same `as` types them.

**The editor (stage 5).** The user asked for editor support as part of this goal.
- **Cases where an enum is expected.** Completion offers `E::Case` rows first wherever the expected
  type is `E`: a `match` arm over `E`, a subscript of `array<E, T>`, a key of a literal typed
  `array<E, T>`, and an argument at an `E` parameter. At an arm and a literal key, a case already
  written is left out, so the list is what is still missing. No new trigger character.
- **The quick fix for a missing case** is the new error's own suggestion, so the server computes
  nothing for it. Each inserted arm throws `LogicError` naming its case, so the file compiles and the
  user replaces each body. A `default` arm is never inserted: it would switch the check off.
- **Declined:** a code action that writes a constant table from an enum (generated code that no
  diagnostic asks for, which `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`
  does not allow); a new trigger character at `[` or `=>`.

**Declined, and written into the rules' Declined lists.**
- **A case as a key of a plain `array<T>`.** It would key by a number another enum shares, and give
  the key back as a string (`E0434` stays).
- **Any key type but an enum.** `array<int, T>` would bring back the integer key domain
  `rule:types/arrays` removed.
- **The case name as the JSON key.** The array keeps no enum identity at run time. Writing names would
  cost a tag per array, and decoding would need a second mapping.
- **A literal that must name every case.** The covering `match` of stage 1 is the total form, and a
  counting array starts empty.
- **An enum-keyed append.** There is no next case to append under.

**Cost.** Stage 1: check time only, and one branch less per `match` over every case. Stage 2: check
time only. A folded table is inlined like any constant. Stage 3 and 4: no memory, no change to storage.
A key read back as `E` from a map key parses a short decimal string. `as array<E, T>` costs one
check per key. Simplicity: the language gains one array form, and loses the `as int` on every write
and the conversion on every key a program reads back. Security: a key from a request has to pass
`as E` before it can be used.

**Feature proofs.** Three features: the covering `match`, enum cases in constants, and the enum-keyed
array. Each owes what `bun nv proofs --owed --goal enum-tables` names. At least, for the keyed array:
an attack that tries to use a case of another enum, an integer and a request value as a key, and a
bench of a lookup by case against the same lookup through `as int`.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on.

**Lanes.** The stages run in order. Stage 3 needs stage 2's folded case keys for its constants, and
stage 4 needs stage 3's type. Stage 5 needs stage 1's error and stage 3's type. Every file here is in
`nvs-types`, `nvs-ir`, `nvs-stdlib` or `nvs-lsp`, which the goals ahead also edit, so this goal runs
last on the chain.

## Stages

### 1. a match over an enum covers every case

- [x] `crates/nvs-types/src/expr/mod.rs:928`: for an `E` or `?E` subject with no `default`, collect the
  cases the arm conditions name and report the new code for the rest. The doc comment names the
  rule stage 6 writes, `docs/rules/expressions/a-match-over-an-enum-covers-every-case.md`.
- [x] `crates/nvs-diagnostics/src/code/types_e10.rs:1`: the new code after `E1008`, with its card,
  written to AGENTS.md § *Text an end user reads*.
- [x] `crates/nvs-ir/src/lower/expr.rs:2268`: no no-match branch for a `match` the checker marked as
  covering, and the test `a_match_over_every_case_lowers_no_fallback_throw` in
  `crates/nvs-ir/src/lower/tests.rs:945`.
- [x] `crates/nvs-types/tests/enum_tables.rs:1` (new): the tests the check `covers` names, then
  `cargo test --test enum_tables` once.
- [x] Every `.nvs` and `.nvst` the new error reaches, fixed as the standing decision says. Found with
  `bun nv suite tests/conformance/` once and the examples' own run.
- [x] `tests/conformance/reject/a-match-over-an-enum-that-misses-a-case-does-not-compile.nvst:1` and
  `tests/conformance/enum/a-match-over-every-case-needs-no-default.nvst:1` (new).

### 2. a class constant and a property default hold enum cases, as int of a case, other constants and Foo::class inside arrays

- [x] `crates/nvs-types/src/defaults.rs:101`, `:921` and `:953`: `ConstArg` and the folder take the
  three shapes of the standing decision, alone and inside arrays.
- [x] `crates/nvs-ir/src/lower/call.rs:699`: `emit_const_arg` emits each, a case with its enum tag.
- [x] `crates/nvs-types/src/defaults.rs:444`: `eval_property_default` takes the same arrays, and
  `E0472`'s help (`:474`) says so.
- [x] `crates/nvs-types/src/expr/members.rs:463` and `crates/nvs-diagnostics/src/code/types_e07.rs:1546`:
  `E0792`'s help and card name only what is still refused, or the code is retired.
  `tests/conformance/core/a-class-constant-with-no-constant-form-is-refused-at-the-read.nvst:1` is
  rewritten to a shape that is still refused.
- [x] `crates/nvs-types/tests/enum_tables.rs`: the tests the check `folds` names, then `cargo test
  --test enum_tables` once.
- [x] `tests/conformance/enum/a-constant-holds-enum-cases-inside-an-array.nvst:1` (new): a class
  constant and a static property, each a table of cases and of `as int` keys, printed.

### 3. array<E, T> types an array keyed by the cases of an enum, and the checker holds every key to it

- [x] `crates/nvs-syntax/src/parser/ty.rs:597`: `array<K, T>` parses, with `TypeAtom::Array`
  (`crates/nvs-syntax/src/ast.rs:142`) holding the optional key.
  - [x] `TypeAtom::Array(elem, key)` and the parser's comma.
  - [x] Every AST walk (hir aliases and requires, lsp definition and index, signatures, walk.rs) visits the key.
- [x] `crates/nvs-types/src/ty.rs:73`: the key field on `Ty::Array` (see the standing decision), its
  interning, its display for hover and diagnostics, and the element-type helper. The resolver reports
  the new key-type code for a key that is not an enum.
  - [x] `Ty::Array(elem, key)`, `enum_array`, `array_keyed`, `array_element`, `array_enum_key`, `describe`.
  - [x] `lower_array_key` (`crates/nvs-types/src/lower.rs`) and `E1017` `E_ARRAY_KEY_TYPE_NOT_ENUM`.
  - [x] Every walk that rebuilds an array type keeps the key (quals, generics, ext_lib, signatures, narrow).
- [x] `crates/nvs-types/src/expr/literals.rs:1621`: `check_array_key_type` takes the expected key
  type. Its callers, `literals.rs:1243` and `:1415`, `crates/nvs-types/src/expr/mod.rs:809` and
  `crates/nvs-types/src/locals.rs:2223`, pass it. The append error goes beside it.
  - [x] `check_enum_key`, and the enum-key help names `array<E, T>`.
  - [x] `E1018` `E_ENUM_KEYED_ARRAY_APPEND` at `$a[] =`, a literal element with no key, and a spread.
- [x] `crates/nvs-types/src/expr/assign.rs:262`: `array<E, T>` is assignable to `array<T>`, and the
  key is invariant between two keyed arrays.
- [x] `crates/nvs-types/src/expr/iteration.rs:439` and `:535`, and `crates/nvs-ir/src/lower/control.rs:994`:
  the `foreach` key is `E`, and a key declared as `E` is taken.
  - [x] The checker: `ForeachSource::Array { value, key }`, `foreach_key_ty` and `check_foreach_key`.
  - [x] The IR: `lower_foreach`'s `guarded_by!(E_FOREACH_KEY_TY)` (`control.rs:994`) takes a key typed
    `Ty::Enum`, and the key at `:1147` is made the case (next item).
- [x] The IR's own array type (`crates/nvs-ir/src/ty.rs:45`) and the lowering of a key read back as
  `E`, with no check, as the standing decision says.
  - [x] A case as a subscript or literal key: `lower_array_key_as` (`crates/nvs-ir/src/lower/expr.rs`)
    relabels it to its backing integer with `reinterpret_enum_to_backing`.
  - [x] A key read back as `E`: the `ArrayKeyAt` string parsed to the backing integer, then
    `InstKind::Reinterpret` to `Ty::Enum`. No membership check.
  - [x] A test in `crates/nvs-ir/src/lower/tests.rs` that the foreach key lowers with no check.
- [x] `crates/nvs-types/tests/enum_tables.rs`: the tests the check `keyed` names, then `cargo test
  --test enum_tables` once.

### 4. Core\Arr keeps and returns enum keys, and as array<E, T> checks each key

- [x] `crates/nvs-stdlib/src/registry.rs:589`: a key variable for `CoreTy`, bound at the call to the
  string key or to `E`.
  - [x] `ARRAY_KEY_VAR` and the roster `nvs_stdlib::arr::KEEPS_KEYS`
  - [x] `nvs_types::core_lib::keep_keys` keys the subject and result, in the base and in `method_sig`
  - [x] `generics` mentions, binds and substitutes an array's key
- [x] `crates/nvs-stdlib/src/arr.rs:178`, `:223`, `:295`, `:315`, `:355`, `:365`, `:516`, `:719` and
  every member whose result keeps keys: the signatures of the standing decision.
  - [x] the keep-keys members: `filter`, `map`, `unique`, `withoutNull`, `sortByKey`
  - [x] `keys`, `firstKey`, `lastKey`, `keyOf`, `findKey` give `E`, converted in the IR
  - [x] a callback's key parameter declared `E` gets the case (`arr.rs` `call_with_key` and the
    other key builders)
  - [x] `flip`, `groupBy`, `groupCounts` (`groupCounts` with `by` declined, see the decision)
- [x] `crates/nvs-ir/src/lower/convert.rs:1651`: `as array<E, T>` and `as ?array<E, T>` check every
  key.
  - [x] runtime `nvs_to_enum_keyed` and `nvs_to_enum_keyed_or_null`, and their unit test
  - [x] `Helper::ToEnumKeyed`/`ToEnumKeyedOrNull`, codegen symbols, `lower_enum_key_check`
- [x] `crates/nvs-types/tests/enum_tables.rs`: the tests the check `arr` names, then `cargo test
  --test enum_tables` once.
  - [x] `a_member_that_keeps_keys_keeps_the_enum_key`
  - [x] `a_string_keyed_array_converts_with_as_and_checks_each_key`
  - [x] `keys_and_first_key_return_the_enum`
  - [x] `flip_turns_enum_keys_into_enum_values`,
    `group_by_a_case_returns_an_enum_keyed_array`
- [x] `tests/conformance/enum/an-array-keyed-by-an-enum-gives-its-keys-back-as-cases.nvst:1` (new):
  a table, a `foreach`, `groupBy` by a case, `keys`, a JSON round trip through `as`.
- [x] `tests/conformance/reject/an-enum-keyed-array-takes-only-its-own-cases.nvst:1` (new): an `int`,
  a `string`, another enum's case, an append and `array<string, int>`, each with its code.
  `tests/conformance/reject/an-enum-case-is-not-an-array-key.nvst:1` keeps its case for a plain
  `array<T>`, and its expected help changes.
- [x] The check `cases` run once.

### 5. the editor offers the cases where an enum is expected, and a quick fix adds the arms a match is missing

**Split:** the `.lspt` cases go to one writing subagent once completion and the fix land. Its brief
names the nine paths of the check `editor`, the neutral names `Priority`, `Low`, `Normal` and `High`,
and `docs/agent/conventions.md` § *An `.lspt` case*.

- [x] `crates/nvs-lsp/src/completion.rs:410`: a new reach for a position whose expected type is an
  enum `E`, offering `E::Case` rows before every other row. The positions: an arm condition of a
  `match` over `E` or `?E` (only the cases no arm handles yet, and `null` for `?E`), a subscript of an
  `array<E, T>`, a key of an array literal whose target is `array<E, T>` (only the cases not written
  yet), and an argument at a parameter of type `E`. Trigger characters do not change: the list opens
  while a word is typed or when the user asks for it
  (`rule:ide/completion-is-asked-where-a-spelling-ends`).
- [x] The same module: after `array<` in type position, the enums in scope are offered beside the
  types, as the existing type completion does.
- [x] Stage 1's new error carries one suggestion that inserts an arm per missing case before the
  closing brace, each `Priority::High => throw new LogicError("…")` with the case named in the
  message. `crates/nvs-lsp/src/actions.rs:287` offers it with no new code
  (`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`). The arm text is held to
  AGENTS.md § *Text an end user reads*.
- [ ] Rename of a case (`crates/nvs-lsp/src/rename.rs:28`) reaches a case inside a constant table and
  a key of a keyed literal. Hover and signature help print `array<E, T>` from stage 3's display. A
  failing case is fixed at its cause.
- [ ] `editors/vscode/syntaxes/nvs.tmLanguage.json:362` already colours a name before `,` in a type
  argument. The highlight case pins it, and the grammar changes only if the case is red.
- [ ] The nine `.lspt` cases the check `editor` names, then the check run once.
- [x] The completion rule gains one sentence on the expected-type rows, in
  `docs/rules/ide/completion-is-asked-where-a-spelling-ends.md:1` or the fragment `bun nv brief
  --where completion` names as the home of what a list offers.

### 6. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** the examples, the attack and the bench go to one writing subagent once the rules are
rewritten. The session writes the rules, the docs and the website entry.

- [ ] `docs/rules/expressions/a-match-over-an-enum-covers-every-case.md` (new), and one sentence in
  `docs/rules/expressions/a-match-has-at-least-one-arm.md:1` pointing to it, with the Declined list
  of stage 1.
- [ ] `docs/rules/enums/no-class-machinery.md:1`: a case is a key of `array<E, T>`, the job table's
  "a case's name" row names a constant table and the covering `match`, and the Declined list above.
- [ ] `docs/rules/types/arrays.md:1`: "one parameter, not two" becomes the enum key, with the storage,
  the read-back, the subtyping and § *Memory*. The Declined list gains `array<int, T>` and the plain
  case key.
- [ ] The rule that holds a class constant's and a property default's values: found with `bun nv brief
  --where constant`, and the folded shapes added.
- [ ] `docs/examples/lang/enums/`: one example each for the three features, with `about.md` updated,
  written to AGENTS.md § *Text an end user reads*.
- [ ] The attack and the bench `bun nv proofs --owed --goal enum-tables` names, at the paths it
  prints.
- [ ] The generated reference pages rebuilt by the command that writes them, never by hand.
- [ ] `website/src/data/changes.json:1`: one entry per feature. Each page above reread and stamped with
  `bun nv site --stamp`.
- [ ] `bun nv proofs --owed --goal enum-tables` names nothing.

### 7. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stage 6 touched.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 4 are done. Stage 5 is all but one item. The nightly CI failures the sweep names ran on
`origin/main` at `53e94a9181` (run `45609291264`). Their fixes, `1c3196b0e8` and `8ca2f09e1b`, are
on local `main` and not pushed yet, so the next push clears them.

What stage 5 landed:
- `543615e749`: `E1016` carries an unsafe fix that inserts one arm per missing case after the last
  arm, indented as it is, each `Priority::High => throw new LogicError('Priority::High is not
  handled yet.'),`. The case prefix is the one an arm already writes (`cover::written_prefix`).
- `4ebe427eca`: `crates/nvs-lsp/src/expected.rs` finds the enum expected at the cursor (a `match`
  arm, a subscript, a keyed literal's key, an argument), read off the tokens at the node's depth.
  `completion::expected_cases` offers the rows under the new first `Tier::Expected`. The checker
  records a keyed literal's enum (`ExprTypeTable::keyed_literal`, set in `check_array_literal`).
  `arguments::param_at` is the slot lookup `route_argument_at` now uses too.
- The nine `.lspt` cases of the check `editor` are written. The whole `tests/lsp` run: 733 green,
  one red, the rename case below. The completion rule has its sentence.

### Next

1. **Rename misses a case read inside a class constant or a property default.** Renaming `High` in
   `tests/lsp/rename/renaming-a-case-renames-it-inside-a-constant-table-and-a-keyed-literal.lspt`
   gives 2:30, 9:45 and 10:23 and misses 6:78, the key in `const array<Priority, string> LABELS`.
   `const Priority DEFAULT = Priority::High;` and a static property's default are missed too, so
   the reference index does not walk constant and property initializers. Fix it at the index
   (`crates/nvs-lsp/src/index.rs` or `references`), so references and highlight gain it too. Then
   tick the rename item and run the check `editor` once.
2. The grammar item: the case `tests/lsp/highlight/an-enum-keyed-array-type-colours-both-arguments.lspt`
   is a `documentHighlight` case, not a colour. Check whether a grammar case exists for
   `editors/vscode/syntaxes/nvs.tmLanguage.json:362` (`bun nv brief --where tmLanguage`) and pin
   `array<Priority, string>` there, or tick the item if the grammar has no case suite.
3. The `.lspt` renderer sorts rows by label (`crates/nvs-lsp/src/render.rs:466`), so no case shows
   that the case rows come first. A unit test in `completion.rs` on `sort_text` starting with `0`
   pins it. No case covers `null` at an arm over `?Priority` yet.
4. Then stage 6 from the backlog.

Expected red: the rename case above, and nothing else.

### Backlog

For stage 6:
- `rule:types/arrays` says which `Core\Arr` members keep an enum key (`KEEPS_KEYS`), that `slice`,
  `reverse` and `sort` do not, and what `as array<E, T>` checks. The reference cards of the five
  keep-keys members say that the result keeps an enum key.
- `docs/reference/lang/20-types.md:4756` and
  `docs/examples/lang/types/properties-and-constants/details.md:20` still say that an enum case and
  another class's constant in a constant do not compile. Both must say what stage 2 made true.
- `docs/examples/lang/enums/match-and-switch-over-an-enum/02-a-match-without-a-default-throws.nvs`
  now throws over a `mixed` value. Check that its name and `about.md` still fit.
- `site --check stale` names what the changed `E0792`/`E0472` help and the new case
  `a-constant-holds-enum-cases-inside-an-array.nvst` (`// covers: lang:types/properties-and-constants`)
  moved. That case may move to the new feature for enum cases in constants.
