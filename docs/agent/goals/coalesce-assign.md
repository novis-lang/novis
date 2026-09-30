---
milestone: post-parity
position: last
---
# Loop goal 181 — `??=` works as PHP's does, and `??+=`, `??-=` and `??.=` start from a default

Two repairs to `??=`, and three new assignment operators built on the mechanism the second repair
needs.

```nvs
// 1. The value of `??=` has the type of what was written. Today this is `E0403`.
public function posts(): array<Post> {
    return $this->posts ??= Post::findActive();      // $this->posts is ?array<Post>
}

// 2. `??=` writes an absent key. Today this throws `undefined array key`.
array<string> $options = ["lang" => "de"];
$options["theme"] ??= "light";

// 3. Three operators that start from a default when the target is `null` or absent.
$counts[$word] ??+= 1;          // $counts[$word] = ($counts[$word] ?? 0) + 1
$stock[$sku] ??-= $sold;        // $stock[$sku]   = ($stock[$sku] ?? 0) - $sold
$lines[$day] ??.= $entry;       // $lines[$day]   = ($lines[$day] ?? "") . $entry
```

**The result type.** An assignment written with `??=` is an expression whose value is the value the
target has afterwards. Its type is the target's type with `null` removed when the right side's type
has no `null`, and the target's type otherwise. Nothing else about `??=` moves: the right side still
runs only when the target is `null` or absent, and the target's declared type is unchanged.

**The absent key.** The read a `??=` makes of its own target is the guarded read `??` makes of its
left operand: an absent key at any level of the target gives `null` and does not throw, so the right
side is written there. `rule:php-migration/absent-storage-is-never-a-zero-value` already says `??`
is the explicit way to read a key that may be absent, and `??=` is that same `??`.

**The three operators.** `$a ??+= $v`, `$a ??-= $v` and `$a ??.= $v` mean `$a = ($a ?? d) op $v`,
with the target evaluated once and `d` the zero of the target's own type: `0` for `int` and `uint`,
`0.0` for `float`, the zero `decimal`, and `""` for `string`. Every other question about them —
which operand types are accepted, what overflows, what a qualifier does, what the expression's type
is — has the answer `($a ?? d) op $v` has when it is written out. There is no way to write another
default: `$a = ($a ?? 100) - 1` is that program, and it already works.

## Why here

A user met `return $this->x ??= compute();` in PHP code being ported, and the two shapes PHP code
uses `??=` for both fail: the memoising getter does not compile against a non-null return type
(`crates/nvs-types/src/expr/assign.rs:@check_compound_assign` returns the target's type), and the
array default throws, which is the recorded gap `nvs-ir/throws-on-an-array-key-that`.
`docs/reference/tools/30-php-differences.md` says `??=` works as in PHP, so the reference is ahead
of the code on both.

The operators are here because they are the same mechanism. The gap's record says what closes it is
a read-side mark on a compound target that the write side does not read. Once a compound target can
be read guarded, `??+=` is `+=` with that read and a default, and `$counts[$w] += 1` throwing on the
first new word is the most common reason a Novis program writes a target twice.

It needs nothing that is not built: `Lowering::lower_read_modify_write` already stages a target's
address once, and already emits a constant of the target's representation for `$x++`.

It carries `position: last` because it lands behind goal `program-enumeration`, and sits in front of
goal `ci-green` because that goal proves the tree the run ends on and this one still changes it.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong or incomplete, each rewritten whole by the session that
lands the behaviour and not before:

- `docs/rules/php-migration/absent-storage-is-never-a-zero-value.md` — "`$a["k"] ?? $d` is the one
  exception". The target of `??=` and of the three operators is the same guarded read, and the
  fragment says so in the same paragraph.
- `docs/reference/lang/30-expressions.md` — the `??=` bullet says "assigns only when `$a` is `null`";
  it gains the absent key and the type of the expression. The precedence table's assignment row gains
  the three operators. § *Assignment* says "every compound form `$x op= e` is `$x = $x op e`"; the
  three operators are stated beside it with their defaults.
- `docs/reference/tools/30-php-differences.md` — "`??=` … all work as in PHP" becomes true at Stage 2
  and stays as written. The page gains the PHP shapes the operators replace: `$a[$k]++` and
  `$a[$k] .= $s` on a key that is not there yet.
- `crates/nvs-ir/src/lower/stmt.rs`'s doc comments on `row_ty_of` ("`??=` marks nothing") and
  `lower_compound_assignment`, `crates/nvs-types/src/expr/presence.rs`'s on `mark_guarded_places`
  ("shared with `??`'s own left operand"), `Env::coalesce_guarded`'s in `crates/nvs-types/src/lib.rs`,
  and `AssignOp::binary_op`'s in `crates/nvs-syntax/src/ast.rs` ("for every variant here", and its
  count of rows).
- `data/gaps/nvs-ir/throws-on-an-array-key-that.json` — built by this goal, and deleted by the
  session that closes it. `docs/examples/lang/expressions/and-the-ternary/03-settings-with-their-gaps-filled-in.nvs`
  loses its `// proof: gap` line in the same commit, and its `.out` is then what the program prints.
- `docs/examples/lang/expressions/and-the-ternary/about.md` and
  `docs/examples/lang/expressions/assignment/about.md`.

The same search closes the stage as it opened it:
`grep -rn "??=\|marks nothing\|the one exception\|evaluated once" docs/rules docs/reference docs/examples crates data/gaps`,
read line by line. Every hit is either true as it stands, rewritten, or under `docs/decisions/`.
`docs/novis.md`, `docs/ground-rules.md`, the `docs/rules/*.md` chapters and `website/` are generated
and are regenerated, never edited.

## Stage 1 — the floor

Goal `program-enumeration`'s whole acceptance list, carried in by the goal switch. Never traded.
Every existing `??` and `??=` case keeps its output: a target that is present and not `null` is read
and left alone exactly as before, and the right side still does not run.

## Stage 2 — `??=` as PHP's, the keystone

**Does:** Makes `??=` write an absent key, and types the expression as the value it wrote.

One file set: `crates/nvs-types/src/expr/assign.rs`, `crates/nvs-types/src/expr/presence.rs`,
`crates/nvs-types/src/lib.rs`, `crates/nvs-ir/src/lower/stmt.rs`.

- **The decision record**, written first, from § *Standing decisions*, for the whole goal. It
  `modifies` `php-migration/absent-storage-is-never-a-zero-value` and `adds` one rule under
  `docs/rules/expressions/` that states the three operators, and the fragments are written with it.
- **The guarded read of a compound target.** `crates/nvs-types/src/expr/assign.rs:@check_compound_assign`
  marks the target's levels for the read when the operator is `??=`, with the walk
  `crates/nvs-types/src/expr/presence.rs:@mark_guarded_places` already is. The mark is one the
  **read** consults and the **write** does not: the gap's record explains why marking
  `Env::coalesce_guarded` alone is wrong — the read and the write share one `ExprInfo` entry per
  span, so a guarded intermediate level drops the `null` that
  `crates/nvs-ir/src/lower/stmt.rs:@row_ty_of` asserts is absent. Where the two entries live is the
  session's call; what is fixed is that `array<?array<int>> $g; $g["0"]["1"] ??= 5;` is still
  `E0482` and never a panic.
- **An absent key at any level is written.** `$g["k"]["j"] ??= 5` with no `"k"` creates the row, as
  the plain `$g["k"]["j"] = 5` does today. An absent key and a stored `null` in an `array<?T>` both
  take the right side; a present value that is not `null` is left alone and the right side does not
  run.
- **The result type.** `check_compound_assign` returns, for `??=`, the target's type with `null`
  removed when the value's type has no `null`. It returns the target's type, as now, when the value
  can be `null`. Every other compound operator and the plain `=` keep returning the target's type.
- **The value is the value written, not a second read.** A test on a hooked property pins that
  `$x = ($o->hooked ??= "v")` runs the `get` hook once, which is the read `??` makes, and the `set`
  hook once, and never reads the property again for the expression's value.
- **Pinned by** the Stage 2 checks, and
  `tests/conformance/lang/coalesce-assign-writes-an-absent-key-and-is-typed-as-the-value-written.nvst`
  (a memoising getter with a non-null return type called twice, an absent key, a nested absent key,
  a stored `null`, a present value whose right side must not run; prints each) and
  `tests/conformance/reject/coalesce-assign-through-a-nullable-row-does-not-compile.nvst`.

## Stage 3 — the three operators

**Does:** Adds `??+=`, `??-=` and `??.=`, which update a target and start from a default when it is
`null` or absent.

Two file sets, in this order. The syntax: `crates/nvs-syntax/src/token.rs`,
`crates/nvs-syntax/src/lexer.rs`, `crates/nvs-syntax/src/ast.rs`,
`crates/nvs-syntax/src/parser/expr.rs`, `crates/nvs-syntax/src/walk.rs`. The meaning:
`crates/nvs-types/src/expr/assign.rs`, `crates/nvs-ir/src/lower/stmt.rs`,
`crates/nvs-lsp/src/completion.rs`.

- **Three tokens.** `crates/nvs-syntax/src/lexer.rs:1008` matches `??=` before `??`; the three
  four-character operators are matched before both. `$a ??-1` and `$a ??+1` lex as they do today, a
  `??` and a signed literal, because a token needs its `=`. A lexer test holds both halves.
- **Three `AssignOp` variants**, parsed in `crates/nvs-syntax/src/parser/expr.rs:256`'s table, right
  associative on the precedence row every assignment operator has, and named in
  `crates/nvs-syntax/src/walk.rs:878`. `AssignOp::binary_op` answers `Add`, `Sub` and `Concat` for
  them, and one more method says whether the operator defaults its read, so the checker and the
  lowering read the pairing from one place as they do today.
- **The checker.** `check_compound_assign` reads the target through Stage 2's guarded read, takes
  `null` off its type, and from there is the `+=`, `-=` or `.=` it already is: the value is inferred
  with the target's type as its hint, the operator's result must be assignable back to the target,
  and every refusal is the operator's own (`??+=` on a `string` is what `+=` on a `string` is). The
  expression's type is the operator's result. A target whose type with `null` removed has no zero —
  an object, an array, a `bool` — reaches that same refusal and needs no code of its own.
- **The lowering.** `crates/nvs-ir/src/lower/stmt.rs:@lower_compound_assignment` goes through
  `lower_read_modify_write`, which stages the target's address once. The read is the guarded one,
  and a `null` or absent read is replaced by a constant of the target's representation, the way
  `lower_incdec` emits its `1`. `??.=` takes the rewrite and never `lower_string_append`: a target
  that can be `null` is not the plain `string` local that path is for.
- **`nvs-lsp` and `nvs-fmt`.** `crates/nvs-lsp/src/completion.rs:1081`'s list of tokens an
  expression follows gains the three. The formatter prints each with one space either side, as it
  prints `??=`; the test pins it, and nothing is built if it already does.
- **Pinned by** the Stage 3 checks, and
  `tests/conformance/lang/defaulting-assignment-counts-subtracts-and-appends-from-nothing.nvst` (a
  word count, a stock count that goes negative, lines grouped by key, a `?int` local and a `?string`
  property that start `null`, a `float` and a `decimal` target, a nested target with an absent row,
  a key expression with a side effect that must run once; prints each) and
  `tests/conformance/reject/defaulting-assignment-keeps-every-refusal-of-its-operator.nvst` (`??+=`
  on a `string`, `??.=` into an `int`, `$a[] ??+= 1`, a nullsafe target, `($a ?? 100) -= 1`; one
  diagnostic each).

## Stage 4 — the feature proofs and the reference

**Does:** Adds the tests, examples, attack and reference text for the repaired `??=` and the three
operators.

No new reference heading, so no new feature on the roster: `??=` is part of
`lang:expressions/and-the-ternary`, the three operators are part of `lang:expressions/assignment`,
and those two features' proofs grow.

- **`lang:expressions/assignment`.** One new example under
  `docs/examples/lang/expressions/assignment/` that counts words with `??+=` and groups lines with
  `??.=`, and prints both. One new numbered step in the attack under
  `tests/hostile/lang/expressions/assignment/`: a `tainted` string appended with `??.=` is still
  `tainted` afterwards, and `??+=` at the largest `int` throws `ArithmeticError`, so neither the
  default nor the operator drops a check. Its bench, `benches/members/lang/expressions/assignment.nvs`,
  is left as it is: Stage 3's IR test is the performance claim.
- **`lang:expressions/and-the-ternary`.** Example 03 runs with no gap line. One example gains a
  memoising getter that returns `$this->x ??= …` from a method with a non-null return type.
- The `covers:` markers on Stage 2's and Stage 3's cases name the feature each belongs to.
- Every document Stage 0 lists, each rewritten whole. The reference chapter's fenced programs run,
  so § *Assignment* gains one that uses the three operators and the `??` section's program gains an
  absent key under `??=`.
- The `about.md` of both features, each inside its own word band.
- `bun nv reference` regenerates `docs/novis.md`.

## Standing decisions

- **The user's calls.** The three operators are `??+=`, `??-=` and `??.=`, and no others: no
  `??*=`, no `??/=`, none for the bitwise operators. The defaults are fixed at the zero of the
  target's type. **No syntax overrides a default**: `($a ?? 100) -= 1` stays `E0105`, because
  `($primary ?? $backup) += 1` reads as "add to whichever exists" and would mean something else.
  The bare `??+` and `??-` are not operators, because `$a ??-1` already means `$a ?? -1`.
- **One sentence answers every question a session meets:** `$a ??op= $v` is `$a = ($a ?? d) op $v`
  with `$a` evaluated once. A case not listed here is decided by writing that expression out and
  doing what it does. If the written-out form is refused, the operator is refused with the same
  code.
- **The default has the target's type, never `int` by default.** `?uint $n = null; $n ??+= 1;`
  gives a `uint`. For a `mixed` or union target the default is the `int` `0` or the `string` `""`,
  and the operator is answered from the runtime tag as `rule:types/arithmetic` says.
- **A target that can never be `null` or absent compiles**, and the operator is then the plain
  `+=`, `-=` or `.=`. That is what `??` and `??=` do on such a value today, and no warning is added
  here.
- **No narrowing after the statement.** `?int $n = null; $n ??+= 1;` leaves `$n` a `?int` on the
  next line, as every write does (`crates/nvs-types/src/expr/assign.rs:@check_assign`'s comment on
  `overwrite`). Only the expression's own value is typed without `null`. A session that wants a
  write to narrow puts it in the handoff's `## Backlog` for the user.
- **The plain `=` is not in this goal.** `$a ?? ($a = f())` is still typed as the target. Same
  backlog.
- **Qualifiers follow the operator.** `tainted` and `secret` do through `??.=` what they do through
  `.=`; the default carries neither.
- **No new diagnostic code**, unless a refusal cannot be made true under the operator's existing
  one. No TextMate change: `editors/vscode/syntaxes/nvs.tmLanguage.json` does not scope symbolic
  operators.
- **One ADR slot**: one new record and no other number, checked right before it is written. It
  states the tradeoffs. Performance: one `null`-or-absent test more than `+=`, and the same one read
  and one write of the target; `??.=` copies the accumulated string on each append, as `.=` on an
  element or a property does today. Memory: nothing. Usability: a count, a running total and a
  grouped string are one line, and ported `??=` code works. Simplicity: three operators PHP does not
  have, whose default is not visible where they are written.
- **The example sweep is bounded.** The two features' examples change. The programs elsewhere that
  write `$a[$k] = ($a[$k] ?? 0) + 1` keep it, and neither form is preferred in new programs.
- **Every name in a test, an example and the record is neutral** — `Post`, `Blog`, `Shop`. The
  report that started this goal named a class of somebody's program, and it is written nowhere.
- **Every comment in a new `.nvs` and every changed `about.md` follows `AGENTS.md` § *Text an end
  user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over them before
  the wrap.
