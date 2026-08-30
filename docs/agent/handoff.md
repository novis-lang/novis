# Handoff

## State

**Stage 8's two acceptance counts are both met** — conformance 1000 and differential 206 against a
floor of 205. The differential half was the driver's outstanding failure and is closed by six cases
in `tests/differential/core/`, all of them ADR 0023 § 2's graph copy asked of the one other
implementation of the same operation: PHP's `serialize`/`unserialize`. The wire formats are
unrelated and neither side can read the other's bytes, so what each case compares is the shape of
the value that comes back — a cycle and its sharing, an array's keys and insertion order, one object
reached twice staying one object, deep-where-`clone`-is-shallow, no constructor on the way back, and
a second round-trip rendering what the first did.

**Item 18's spelling decision is landed and its home is `nvs_stdlib::registry`'s `Qual` doc comment**
(§ *How a `secret` parameter is spelled*). A `secret` parameter is spelled with a **fifth mark**,
`Qual::Reveal`, and not with a `CoreTy::Secret` and not with a pair hard-coded in the checker; the
doc comment carries all three arms of the reasoning, including why it may not reuse `Qual::Launder`.
`admits_tainted_argument` has its arm (`Reveal` admits `tainted` for `Contagious`'s reason, since
removing `secret` says nothing about the other axis). **No row writes the mark yet**, and
`core_lib::tests::every_mark_a_row_can_write_reaches_a_signature` says in a comment why it is not
counted there — so today the tree still refuses every `secret` argument, exactly as before.

**Five known gaps carry forward unchanged**, each recorded where its code is: `Core\Secret::reveal()`
itself is unwritten (the next group); `Live::admit`'s same-class check is asked of the answer, not
the argument (`crates/nvs-runtime/src/graph.rs` § *Known gaps*); item 22's `Core\Script` members are
unwritten (`crates/nvs-stdlib/src/script.rs`); a generic `Core` member returns before the admission
loop; and a `...` spread carries its qualifier on the array
(`crates/nvs-types/src/expr/args.rs:@check_args_typed`).

## Next group

**Finish item 18: the mark now exists and nothing writes it.** ADR 0033 § 3 is the whole
specification. One file set: a new `crates/nvs-stdlib/src/secret.rs`, `crates/nvs-stdlib/src/
registry.rs:984` (`CLASSES`) and its `Qual` at `registry.rs:141`, `crates/nvs-types/src/expr/
args.rs:480` (`check_arg_admitting_taint`), `crates/nvs-types/src/expr/quals.rs:196`
(`admits_tainted_argument`) and `tests/conformance/{core,reject}/`.

- [ ] **Write the class: `reveal(secret string $s, string $reason): string` and its `bytes` twin**
      (ADR 0033 § 3). Five edits per member — row, card, `nvs_helper!` body, the `address()` arm
      (`crates/nvs-stdlib/src/arr.rs:2126` is the shape) and a `.nvst` case, or
      `conformance_coverage.rs` fails. Both rows write `CoreTy::Text(Qual::Reveal)` /
      `CoreTy::Blob(Qual::Reveal)` in slot 0 and an unclassified `CoreTy::Str` for `$reason`, which
      is what makes the reason itself unable to carry a qualifier in.
- [ ] **Admit the argument on the `secret` axis** (`args.rs:480`, `quals.rs:196`).
      `check_arg_admitting_taint` is the `tainted` axis only and `admits_tainted_argument` is named
      for that axis on purpose; a `Reveal` parameter needs the same shape one axis over, and the
      answer must come back with `secret` removed and `tainted` still on it. This is the slice that
      makes `Qual::Reveal` mean anything, so it lands with the rows or not at all.
- [ ] **Two cases: the escape works and it is the only one** (ADR 0033 §§ 3-4). A revealed value is
      an ordinary `string` and may be echoed; every other route out of `secret` is still refused —
      `tests/conformance/reject/a-secret-value-does-not-cross-a-boundary.nvst` is the neighbour that
      already pins the refusals, so the new reject case asserts the *narrowness* rather than
      repeating them.

## Backlog

- Item 22's `Core\Script` members are unwritten — `crates/nvs-stdlib/src/script.rs`.
- `Live::admit` asks its same-class check of the answer, not the argument — `graph.rs` § *Known gaps*.
- A generic `Core` member returns before the admission loop — `crates/nvs-types/src/expr/args.rs`.
- A `...` spread carries its qualifier on the array rather than on the entries — `args.rs:@check_args_typed`.
- Whether a `Neutral` parameter launders `secret` is still ADR 0088's open answer — `quals.rs:196`.
- The Stage 8 `differential` check has no `cases` list, only a count — `docs/agent/loop-goal.toml:1660`.
