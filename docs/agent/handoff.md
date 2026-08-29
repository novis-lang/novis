# Handoff

## State

**ADR 0057's fold now reads all five of § 1's rows.** The `Grammar::Duration` arm is
`crates/nvs-types/src/intrinsics.rs:224` and reaches `nvs_syntax::duration::parse` *directly* — no
`validate` entry point beside it, unlike the other three grammars, because ADR 0070 § 5 already put that
parser in `nvs-syntax` for the lexer, `Core\Time\Duration::parse` and an `nvs.toml` directive to share, and
`DurationError::message()` is the sentence all of them print. A fourth caller of one function is the
standing decision's "one implementation" outright.

**Both of the stage's `.nvst` cases are on disk and green**, conformance 890.
`core/a-literal-intrinsic-is-validated-while-checking.nvst` is `examples/intrinsics.nvs` with `--EXPECT--`
around its four lines; `reject/a-malformed-literal-intrinsic-is-a-compile-error.nvst` writes every folded
row twice — malformed as a literal and the identical text behind a `string` — and pins the six messages plus
`aborting due to 6 errors`, which is what asserts that the variable-borne twins added nothing (§ 2).

**The example was demonstrating the regex row with a member that is not on the list.** It called
`Core\Regex::matches`, so nothing folded; both it and the case now go through `Core\Regex::compile`. The
playbook bullet under *Writing a test case* owns the trap.

**Six more conformance cases moved to the runtime path**, exactly as the playbook's two ADR 0057 bullets
predict for each new arm: the five `time-duration-*` cases and `error/a-core-member-throws-a-named-class`
each wrote a malformed duration inline. The grep those bullets prescribe found all six in one call before
the arm landed.

**Gaps 1–4 in `intrinsics.rs`'s module doc are unchanged** — whole-literal span, nothing prepared, a
member's own restriction on a well-formed pattern, no named/spread argument read.

**The failing acceptance check is stage 4 and is not a regression:** `nvs build` is not a subcommand at all
(`nvs --help` lists `ast`, `check`, `run`, `test`, `info`), so `nvs build --openapi` exits 2. That is
unimplemented work, and it is the next group.

**Orientation gap, fifth session running:** the pack still does not print ADR 0057, the goal stage it is
narrowed to. `[context] adrs` needs `0057 §§ 1, 3, 4`; this session sliced § 1 by hand for one call, and its
table is what settled the `matches`/`compile` question above.

## Next group

**Stage 4 — ADR 0085's emitter, `nvs build --openapi` then `nvs api diff`. Shared file set:**
`crates/nvs-cli/src/main.rs`, `crates/nvs-types/src/routes.rs`, `examples/routes.nvs`, and a new home for
`-p nvs-cli` tests. **`crates/nvs-cli` has no `tests/` directory today** — `src/` is `main.rs`, `info.rs`,
`runner.rs` — so the first slice decides where the four named tests live before writing them.

- [ ] **`nvs build --openapi <file>` emits a 3.1 document.** The subcommand joins `enum Command` at
      `crates/nvs-cli/src/main.rs:78` (`info.rs` is the shape a non-running subcommand takes, per
      `loop-goal.toml:82`); the routes come from `nvs_types::routes::RouteTable`
      (`crates/nvs-types/src/routes.rs:349`) and the fixture is `examples/routes.nvs`. Acceptance wants
      `"openapi": "3.1`, `"/users/{id}"` and `"operationId"` in stdout (`loop-goal.toml:1265`).
- [ ] **Determinism and `#[Api]`.** `an_openapi_document_is_byte_identical_across_two_emissions` (§ 3 — a
      hash map is the cheapest way to fail it) and `an_api_attribute_contradicting_its_own_signature_is_a_diagnostic`
      (§ 2's four contradictions). Named at `loop-goal.toml:1277-1281`.
- [ ] **`nvs api diff`**, plus `an_api_diff_classifies_a_removed_route_as_breaking` and
      `..._an_added_optional_parameter_as_compatible` (§ 4). The classification is mechanical over two
      emitted documents, which is why the goal makes it one slice with the emitter.

## Backlog

- `a_literal_duration_is_validated_while_checking` is green and not in `loop-goal.toml:1234`'s stage-3
  `tests` list; adding it tightens the floor — `docs/agent/loop-goal.toml`.
- Gap 2, nothing prepared: § 3's compiled pattern and parsed plan need a channel to `nvs-ir` —
  `crates/nvs-types/src/intrinsics.rs` module doc.
- Gap 3, a member's own restriction on a well-formed pattern (`cldr::civil_fields_only`) — same doc.
- Stage 5's depth pass is the standing fallback whenever a stage's group is blocked —
  `docs/agent/loop-goal.toml:1288`.
