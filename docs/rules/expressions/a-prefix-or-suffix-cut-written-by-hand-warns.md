A prefix or suffix cut is one call: `Core\Str::afterPrefix(S, P)` tests that `S` starts with `P` and
returns the rest, or `null`, and `Core\Str::beforeSuffix(S, P)` does the same at the end. Where a body
writes the test and the cut as two or three calls, the cut is `W1036`, and it names the member. The
reason is simplicity: the prefix is written once, and the length that cuts it cannot disagree with
the test. It is not a speed rule, so it is not held to the cheaper-forms 2x bound
(`rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named`).

**The shapes, and only these.**

| Written | Named |
|---|---|
| `Str::slice(S, N)` reached only where `Str::startsWith(S, P)` is `true` | `Str::afterPrefix(S, P)` |
| `Str::slice(S, 0, -N)` or `Str::slice(S, 0, Str::length(S) - N)` reached only where `Str::endsWith(S, P)` is `true` | `Str::beforeSuffix(S, P)` |

`N` is `Str::length(P)`, with or without `as int`, or an integer literal equal to the length of a
literal `P` that is ASCII with no escape and no `\r`, so its byte count is its grapheme count. `S` is a
plain variable, `P` is a literal, a constant or a plain variable, and neither is written between the
test and the cut. "Reached only where" is one of two places:

- the `then` block of an `if` whose condition is the test, or has it as one operand of a chain of `&&`;
- a later statement of the same block, after an `if` with no `else` whose condition is `!test`, or has
  it as one operand of a chain of `||`, and whose block ends in `return`, `throw`, `continue` or
  `break`.

The chains of `&&` and `||` are the one widening past the table: either still means the cut runs only
where the test held. Anything else gets no warning, and the match is conservative, as `W1035`'s is. The
walk stops at the first statement that writes `S` or `P`, so `$s = Str::slice($s, 1)` is not named. An
anonymous function between the test and the cut stops it too, and so does a statement it does not
know. `crates/nvs-ir/src/hand_cuts.rs` finds the cuts on the written tree with the checker's resolved
calls, beside `rereads.rs`, and `nvs check` and the editor raise them from the same pass.

**The fix is offered for one exact shape.** The `if` has no `else`, its condition is the test alone,
the first statement of its block declares `$v` from the cut, and `$v` is not declared before the `if`.
The fix is an `alternative` (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`): it writes
`?string $v = Str::afterPrefix(S, P);` before the `if`, the condition `$v != null`, and the block
without its first statement. Every other match warns with no fix.

**The tree has one `W1036`, on purpose:** `benches/members/core/Str/afterPrefix.twin.nvs`, the bench
twin that is the hand-written cut. Novis has no annotation that silences a warning
(`rule:core-api/casing-has-no-suppression`), nothing fails on a warning, and a twin exists to be the
form the warning names. A ternary `startsWith(S, P) ? slice(S, N) : S` is not named, and is written
`afterPrefix(S, P) ?? S`.

**Measured** with `nvs run --count` on the bench pair: per operation, the hand-written cut runs 9.00
statements and 2.00 calls, and `afterPrefix` runs 6.00 statements and 1.00 call. Both make 0.50
allocations and 24.75 bytes. The perf ledger's twin ratio is 1.29x.

**What it spends:** while compiling, one walk of each body, and for each test it finds, one walk of
the statements it guards. At run time, nothing.

**Declined:**

- **A fix for every shape.** A wider fix must pick a variable name, or move a declaration across code
  the author wrote.
- **A row of the cheaper-forms table.** That table compiles a faster form silently and is held to 2x.
  This warning is about one call instead of three and the prefix written once, and 1.29x is not 2x.
- **Naming a ternary.** `afterPrefix(S, P) ?? S` is no shorter to read, and the ternary already
  writes the prefix where the reader sees both halves.
