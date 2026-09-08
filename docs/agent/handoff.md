# Handoff

## State

**Goal 18. Stage 0's catch-up check is closed, and stage 5 items 1–3 are landed.**

`crates/nvs-diagnostics/src/lib.rs` carries `the_pipeline_codes_are_the_next_free_parser_band_numbers`
again. It was not unwritten work: commit `009eca413` renamed it to
`the_newest_parser_code_is_the_bands_highest_number` when `E0132` took the top of the parser band, and
that rename broke the `cargo-named` acceptance check that names the old test. The two invariants are now
two tests — where the pipeline's three were *allocated* (one past the highest code that predates them,
which holds whatever lands above them later), and which code is the band's highest *now*, which is what
`brief.py`'s next-free line reads.

**An unqualified inline shape at a `Core\Request` decode site is now refused.** `check_decode_sites`
walks `CodecFieldSite` rows for a deriving class, so a `Ty::Shape` written as a type argument reached it
never; `crates/nvs-types/src/derive.rs`'s new `check_shape_decode_site` answers the same question where
the shape is written, since a shape declares its own fields and has no declaration further down the file
to carry the qualifier. `written_class_of` calls it for the three `Core\Request` members and for no other
owner — `Core\Json::decodeAs` takes its document through a plain `string` parameter, `Core\Arr::shapeAs`
converts an array the program already holds, and a `Core\Db` row is not a taint source.

`unqualified_text` also walks a union arm and a nested shape now, not only an array element, so `?string`
and `{inner: {name: string}}` are reached on the class side as well.

Three landed cases under `tests/conformance/core/` were written before the diagnostic existed and now
write `tainted` at the call site. Nothing else about them changed, so the qualifier cost a word and no
behaviour.

**Not a defect, checked and ruled out:** a shape field typed `array<T>` parses fine in both a parameter
and a type-argument position. A probe that said otherwise had named its field `list`; the playbook owns
the tell.

## Next group

**Stage 5: the missing-required-key diagnostic** — one file set:
`crates/nvs-types/src/expr/assign.rs`, `crates/nvs-diagnostics/src/lib.rs`,
`tests/conformance/reject/`. loop-goal.md § *Stage 5*'s third clause is the specification, and it needs
the checker before it needs a case, exactly as the first clause did.

- [ ] **A shape literal missing a required key names the key** — `rule:types/shape-type` and
      loop-goal.md § *Stage 5*. Today it is the generic assignability mismatch: passing
      `({name: "x"})` where `{age: int, name: string}` is declared reports `E0401` as *expected
      `{age: int, name: string}`, found `{name: string}`*, which prints both shapes whole and leaves
      the reader to diff them. The relation is `assignable` at
      `crates/nvs-types/src/expr/assign.rs:1073` and the report site is
      `crates/nvs-types/src/expr/assign.rs:404`; decide there whether the key is named by widening
      `E0401`'s message on the shape-to-shape arm or by a code of its own, since `E04xx` is full and a
      new one is `E0811`.
- [ ] **One shape mismatch is reported twice through a parenthesized group** —
      `crates/nvs-types/src/expr/assign.rs:404`. `Door::a(({name: "x"}))` reports the same `E0401`
      at the group's span and again at the literal's, one column apart. Worth settling in the same
      slice, because the case below has to freeze whichever count is correct.
- [ ] **Its `--EXPECTF-ERROR--` case**, once the message is decided — written beside
      `tests/conformance/reject/an-unqualified-shape-at-a-request-decode-site-is-refused.nvst:28`,
      whose section shape it copies. The `-->` line's indentation widens with the line number, so a
      single-digit line takes two spaces and a two-digit line three.

## Backlog

- `written_class_of` pushes a `RowSite` on `method == "queryAs"` without testing the owner
  (`crates/nvs-types/src/expr/args.rs:1530`), so `Core\Request::queryAs<C>` over a declared class looks
  like a `Core\Db` row site. Not reproduced — the shape arm returns before it, so only the class form
  could reach it.
- Reference pages for `Core\Request::queryAs` and `postAs` — loop-goal.md § *Stage 5* names three
  members and `docs/reference/core/Arr.md` has only `shapeAs`.
