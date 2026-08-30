# Handoff

## State

**Stage 8's corpus count is 1000 and the acceptance check's floor is met** — six cases, all over
ADR 0088 § 2's admission, which is now pinned end to end in the corpus rather than only in
`crates/nvs-types`. What they hold: a contagious answer carries the qualifier through a union's
member and an array's element (`?tainted string`, `array<tainted string>`) and the plain declared
type is refused; ADR 0063 R11's four grammar sinks **agree** across four classes; the three
`Qual::Launder` rows each hand their answer to the sink their doc comment names; `secret` is refused
at the same contagious, neutral and laundering parameters that admit `tainted`; and a tainted
argument is still refused wherever the answer cannot carry the bit (`Core\Uri::parse`,
`Core\Json::decode`, `Core\Bytes::unpack`, `Core\Regex::match`).

**The four marks are counted where the sweep they copy lives**, not where the item said. The item
asked for `crates/nvs-types/tests/tainted.rs`; `method_sig` and `qual_of` are module-private, so an
integration test cannot ask the question at all, and
`core_lib::tests::every_mark_a_row_can_write_reaches_a_signature` sits beside
`every_registered_parameter_carries_its_classification_into_its_signature` instead. It adds the two
things that one does not ask: `Qual::Launder` reaches a signature, and each of R11's four grammar
classes carries a sink somewhere in its rows.

**Four playbook bullets were stale and are corrected in place**, all of them saying that no `Core`
member accepts a tainted argument. That stopped being true two sessions ago; a session reading them
would have written its cases against a behaviour the tree no longer has.

**Five known gaps carry forward unchanged**, each recorded where its code is: item 18's
`Core\Secret::reveal()` is not in the registry (`nvs_types::expr::quals`); `Live::admit`'s same-class
check is asked of the answer, not the argument (`crates/nvs-runtime/src/graph.rs` § *Known gaps*);
item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`); a generic `Core`
member returns before the admission loop; and a `...` spread carries its qualifier on the array
(`crates/nvs-types/src/expr/args.rs:@check_args_typed`).

## Next group

**Close item 18: `Core\Secret::reveal` is the one named way out of `secret`, and nothing implements
it.** ADR 0033 § 3 is its whole specification — the spec's § 1 tree has no row for the class. One
file set: a new `crates/nvs-stdlib/src/secret.rs`, `crates/nvs-stdlib/src/registry.rs:945`
(`CLASSES`), `crates/nvs-types/src/expr/quals.rs:186` (`admits_tainted_argument`) and
`tests/conformance/{core,reject}/`.

- [ ] **Decide how a `secret` parameter is spelled on a row, and record it** (ADR 0033 § 3). There is
      no `CoreTy` that accepts a qualified argument today — `Qual` describes what a member does with
      one, and every mark refuses `secret`. Whether `reveal` gets a fifth mark, a `CoreTy::Secret`,
      or a hard-coded pair in `expr/quals.rs` is this session's call under the goal's standing
      decisions; the home for the reasoning is `registry.rs`'s `Qual` doc comment.
- [ ] **Write the class: `reveal(secret string $s, string $reason): string` and its `bytes`
      overload** (ADR 0033 § 3), as conventions.md's five edits — row, card, body, `address()` arm,
      `.nvst` case. The `$reason` is required and is not decoration: § 3 wants the call site to say
      why.
- [ ] **Two cases: the escape works and it is the only one** (ADR 0033 §§ 3-4). A revealed value is
      a plain `string` that `echo` accepts, and the sinks § 4 lists still refuse the `secret` value
      itself — the reject half beside
      `tests/conformance/reject/a-secret-argument-is-refused-where-a-tainted-one-is-admitted.nvst`.

## Backlog

- `Core\Str::format("%s", $tainted)` compiles and answers a plain `string`: a qualifier entering a
  `Variadic(Mixed)` is lost. ADR 0007's unchecked position, or a hole ADR 0088 should name — the
  question is ADR 0088's.
- Item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`'s module doc says
  what the class deliberately does *not* have).
- `Live::admit`'s same-class check is asked of the answer, not the argument
  (`crates/nvs-runtime/src/graph.rs` § *Known gaps*).
- A generic `Core` member (`Core\Json::decodeAs<T>`) returns from `check_generic_args` before the
  admission loop, so it still refuses a tainted argument.
- A `...` spread carries its qualifier on the array rather than on the entries
  (`crates/nvs-types/src/expr/args.rs:@check_args_typed`).
