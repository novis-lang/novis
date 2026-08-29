# Handoff

## State

**Stage 4's first half is on disk: `nvs build --openapi <file>` emits a 3.1 document**, and the acceptance
check that failed after session 0002 (`error: unrecognized subcommand 'build'`) now passes. The subcommand
is `crates/nvs-cli/src/main.rs:160`, its runner `main.rs:428`, and the emitter is the new module
`crates/nvs-cli/src/openapi.rs` — whose module doc owns the list of **six § 1 rows the table does not
supply yet** (responses, request bodies, summaries, everything `#[Api]`, `info.version`, and a type outside
the scalar map rendering as the empty schema). Nothing here infers: every member is read off a `Route` row.

**`Route` grew `params: Vec<RouteParam>`, replacing `query: Vec<String>`** (`crates/nvs-types/src/routes.rs:308`,
the type at `:346`). A row now carries § 2's path captures in path order then ADR 0102 § 3's `#[Query]`
parameters, each with its name, where it arrives from, whether it may be absent, and the declared type as
`TypeInterner::describe` renders it — which is what § 1's "by declared type" needs and what the diff slice
will compare. The old field had exactly one reader (`links.rs:243`) and it asks the same question through
`ParamIn::Query`.

**The emitter builds a `serde_json::Value`, not text.** `serde_json` is already a vetted workspace
dependency with its rationale at `Cargo.toml:114`, so `nvs-cli` only had to name it — no dependency ritual
was owed. **This closes the open question the next slice would otherwise have started with:** `nvs api diff`
reads two documents with the crate the emitter already writes through, and diffs `Value`s rather than text.

**Test home decided: `crates/nvs-cli/tests/openapi.rs`**, driving the built binary through
`env!("CARGO_BIN_EXE_nvs")`. `nvs-cli` is a bin crate with no lib target, so a test cannot call the emitter
directly — and the thing ADR 0085 § 3 promises is what the *command* writes. Four tests, all green, one of
them slice 2's named `an_openapi_document_is_byte_identical_across_two_emissions`.

**The stage-4 `cargo-named` check still fails**, on the two tests that do not exist yet:
`an_api_attribute_contradicting_its_own_signature_is_a_diagnostic` and the two `nvs api diff` ones. That is
an item still open, not a regression.

**Orientation gap, sixth session running:** the pack does not print the ADR of the stage it is narrowed to.
`[context] adrs` needs `0085 §§ 1-4` (this session sliced the whole decision half by hand for one call);
the previous session's request for `0057 §§ 1, 3, 4` is now behind the stage and can be dropped.

## Next group

**Stage 4's second half. Shared file set:** `crates/nvs-cli/src/main.rs`, `crates/nvs-cli/src/openapi.rs`,
`crates/nvs-cli/tests/openapi.rs`.

- [ ] **`nvs api diff <old.json> <new.json>`, ADR 0085 § 4.** Breaking / additive / cosmetic, non-zero exit
      on a breaking one. Read both with `serde_json::from_str::<Value>` — the dependency is already on
      `nvs-cli` — in a new `crates/nvs-cli/src/api_diff.rs`, and join `enum Command` at
      `crates/nvs-cli/src/main.rs:84` beside `Build` (`:160`). Breaking is a removed operation, a removed or
      newly required parameter or field, a narrowed type, a removed enum case, a changed status code;
      additive is a new optional parameter or field, a new operation, a new enum case. Tests
      `an_api_diff_classifies_a_removed_route_as_breaking` and
      `an_api_diff_classifies_an_added_optional_parameter_as_compatible` (`loop-goal.toml:1283`) go in
      `crates/nvs-cli/tests/openapi.rs` beside the four already there, building both documents from fixtures
      with the emitter rather than pasting JSON.
- [ ] **`#[Api]` and its four contradictions, § 2.** The name joins `nvs_types::derive::ATTRIBUTES`
      (`crates/nvs-types/src/derive.rs:82`) and is read where `#[Route]` is, in `collect_route`
      (`crates/nvs-types/src/routes.rs:574`); ADR 0102 § 8 already put `Core\Access` on ADR 0071 § 1's closed
      list, so this is the same move. The test the check names is
      `an_api_attribute_contradicting_its_own_signature_is_a_diagnostic`; the four contradictions are an
      `errors` type the handler cannot produce, an unknown `security` scheme, an `example` that fails the
      real decoder, and an `#[Api]` with no `#[Route]`. **This is the expensive one — take it alone**, and
      note the last of the four is the only one needing no other subsystem.
- [ ] **Response schemas, § 1** — gap 1 in `openapi.rs`'s module doc. Needs the handler's declared return
      type on the row and ADR 0071's codec to render a class; the scalar half (`string`, `int`) is reachable
      today with only the return type added beside `RouteParam`.

## Backlog

- Stage 4's `nvs build` grows a `-o <file>`; today the document only goes to standard output
  (`crates/nvs-cli/src/openapi.rs` module doc).
- `info.version` has no source in the language — it waits on M6's `nvs.toml` reader (ADR 0103).
- `Core\Uuid` and enum-typed captures render as the empty schema; the mapping is `openapi.rs`'s `schema`.
- Gaps 1-4 in `crates/nvs-types/src/intrinsics.rs`'s module doc (ADR 0057's fold) are unchanged.
- `Core\Router::match` and the dispatcher stay out of this goal (loop-goal § *Standing decisions*).
