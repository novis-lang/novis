A class constant's value, and a property default, static or instance, is folded to a value when the
program compiles, and the value is inlined at every read
(`rule:classes/no-free-functions-or-constants`). What folds, alone and as an array element or key at
any depth:

- a literal: a number, a string, `bool`, `null`, and an array of these;
- an enum case, kept as a case of its enum, so an element of `array<E>` is a case and a key of
  `array<E, T>` is one (`rule:types/arrays`);
- `$case as int` and `$case as uint`, where `$case` folds to a case of an enum with that backing, as
  its integer;
- another class's constant whose value is a number or a string;
- `Foo::class` and `self::class`.

So a lookup table keyed by an enum is a constant: `const array<Priority, string> LABELS =
[Priority::Low => 'low', …]`. A case inside an array is written with its enum tag
(`rule:enums/representation`). The folder is `eval_const_value` and `fold_const_array` in
`crates/nvs-types/src/defaults.rs`, and the lowering emits each folded element with
`emit_const_arg` (`crates/nvs-ir/src/lower/call.rs`).

What does not fold is **`E0792`** at the read, not at the declaration, because a constant nobody
reads costs nothing. Three shapes reach it: another class's constant that is itself an array or an
enum case, `static::class`, which has no class before a method runs, and a shape-typed constant. A
cycle between two constants is the first of these.

A case, a class name or a constant inside a table is a recorded use, so rename, references and the
editor's highlighting reach it, and a deprecated or restricted constant read inside another constant
warns there.

**Cost.** Check time only. A folded table is inlined like any other constant, and nothing is computed
when the program runs.

**Declined:**
- **Folding another class's array or enum constant.** The class constants are folded from literals
  first, in a pass before this one, and a second pass that follows a chain of constants is a fixed
  point the checker does not need for a table that can be written out.
