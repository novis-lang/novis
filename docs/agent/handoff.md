# Handoff

## State

**M4 — language completeness.** The write-target rule has a fourth entry and no shape below it that
panics. An element write whose root is not a **place** — `$h->rows()["a"] = "y"`, `[1, 2]["0"] = "z"`,
a ternary — is `E0700` at `mwl_types::expr::assign::check_write_target`
(`crates/mwl-types/src/expr/assign.rs:432`), and `is_a_place` beside it is that rule's only home:
exactly the roots `write_back_array` can re-point. It is the one of the four PHP does *not* refuse
(8.5 writes into the temporary and discards it silently), so **ADR 0007 § 7 row 15** records the
divergence.

**The `E04xx` band is full and the decision is taken.** The types band continues at **`E07xx`**,
opening at `E0700`; `E0500` is never issued, its digits reading as IR-and-codegen.
`docs/adr/README.md` § *Decisions taken at project start* owns the reasoning, and `tools/brief.py`
now prints a filled band as `FULL at E0499` rather than handing out the number past its end. `E08xx`
is unallocated.

**Parentheses are transparent to what an expression *is*.**
`mwl_syntax::ast::Expr::unparenthesized` is that rule's one home, and both walks that ask it go
through it now — the write-back root, and `Lowering::aliasing_read`. That closed a live
double-release: `array<string> $b = ($a);` printed correctly and exited **127**, and `($a)["0"] = "y"`
panicked. Both run, `tools/leak-check.sh` clean over them, exit 0.

**An increment passes the parser's gate too**, so `$h->n()++` is `E0105` where it is written rather
than an assertion inside `lower_read_modify_write`, and `lower_store`'s catch-all
(`crates/mwl-ir/src/lower/stmt.rs:1171`) is proven dead — it is an `unreachable!` carrying the proof.
Measured, so it is not re-derived: `$b = $a++` and `$c = ++$a` over a local already lower and print
PHP's answers.

## Next group

**The two remaining assertions in the write path, both in one file.** The file set:
`crates/mwl-ir/src/lower/stmt.rs`, `crates/mwl-types/src/expr/assign.rs`, `tests/conformance/lang/`.

- [ ] **Prove `lower_read_modify_write`'s assertion dead, or find what reaches it** —
      `crates/mwl-ir/src/lower/stmt.rs:507` asserts `reevaluable_target`
      (`crates/mwl-ir/src/lower/stmt.rs:1516`). The kinds that can arrive are now exactly the four
      `mwl_syntax`'s `is_assignable` (`crates/mwl-syntax/src/parser/mod.rs:435`) admits, minus a
      chain whose root `E0700` refuses; `(new H())->n += 1;` already lowers, staged. The open
      question is a subscript chain over a property of a temporary.
- [ ] **`stmt.rs:909` — a property assignment target with no declaring class recorded** — find the
      spelling that reaches it or word it as the invariant it is (`python tools/holes.py --item 7`
      lists it).
- [ ] **`stmt.rs:1198` — an intermediate level of a nested element write with no element type
      recorded** — same shape of question, same file, same tool.

## Backlog

- `($a) = 5;` and `($a)++;` are `E0105`, and PHP refuses both as parse errors — settled, do not
  re-open (`crates/mwl-syntax/src/parser/expr.rs`'s `require_write_target`).
- `E08xx` is unallocated; the next band to fill takes it — `docs/adr/README.md`.
- ADR 0007 § 2's `array<T> as array<U>` still does not lower, which is what blocks several depth
  cases — `docs/agent/playbook.md`.
- `python tools/holes.py` is the live worklist; `--item N` prints one in full.
