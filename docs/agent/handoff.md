# Handoff

## State

**Stage 4 is on disk except `#[Api]`.** `nvs build --openapi <file>` emits ADR 0085 § 3's document and
`nvs api diff <old.json> <new.json>` is § 4's gate. The acceptance check the driver reported failing after
session 0003 (`nvs build --openapi emits 3.1`) **passes at this commit** — run by hand against
`target/debug/nvs.exe`, all three `want` fragments present, exit 0. Nothing was changed to make it pass, so
the driver's failure was against a binary built before session 0003's commit landed.

**`crates/nvs-cli/src/api_diff.rs` is the gate.** Three classes over two `serde_json::Value`s, one report
line per change worst-first, `ExitCode::FAILURE` on any breaking one. Its module doc owns the
classification and the two places it deliberately errs towards breaking (a removed parameter whatever its
optionality, a changed `format`), which is ADR 0085's own "occasionally wrong at the margins, and no
suppression mechanism". The walk covers members no Novis document carries yet — `responses`,
`requestBody`, `components.schemas` — because the *old* side of a diff is whatever a team's last release
wrote, and a gate that skipped an unrecognised member would report "no change" over a document that
removed every response in it.

**Fixtures are one file each**, `crates/nvs-cli/tests/fixtures/api/{base,removed,optional}.nvs`: `base`
has two operations, `removed` deletes one, `optional` adds a `#[Query] $sort` with a default. Both
documents in every diff case are built with the emitter, never pasted JSON, so a change to the emitter's
shape fails here rather than leaving two frozen files agreeing with each other.

**`crates/nvs-cli/tests/openapi.rs` is 9 tests, all green**, including three of the four the stage-4
`cargo-named` check names. That check now fails on exactly one missing test —
`an_api_attribute_contradicting_its_own_signature_is_a_diagnostic` — which is the next slice, not a
regression.

**Orientation gap, seventh session running:** `[context] adrs` still does not carry `0085 §§ 1-4`, and
this session sliced §§ 3-4 and *Verification* by hand for two calls.

## Next group

**`#[Api]`, then what it and the return type let the document say. Shared file set:**
`crates/nvs-types/src/derive.rs`, `crates/nvs-types/src/routes.rs`, `crates/nvs-cli/src/openapi.rs`,
`crates/nvs-cli/tests/openapi.rs`.

- [ ] **`#[Api]` and its four contradictions, ADR 0085 § 2.** The name joins
      `nvs_types::derive::ATTRIBUTES` (`crates/nvs-types/src/derive.rs:82`) beside the `Core\Query`
      constant at `:151`, which is the shape to copy — matched nominally after `resolve_ref`, never
      structurally. The payload is read where the route row is built (`routes.rs:577`
      `collect_route`) and checked where the table is (`routes.rs:1086` `check_table`), which is already
      the pass that reports a duplicate route. § 2's four contradictions — an `errors` entry naming an
      unreachable type, an unknown `security` scheme, an `example` that fails the real decoder, an
      `#[Api]` with no `#[Route]` — are one new diagnostic each or one code with four messages; next free
      in the types band is **E0771**. The test the acceptance check names is
      `an_api_attribute_contradicting_its_own_signature_is_a_diagnostic`, in
      `crates/nvs-cli/tests/openapi.rs`.
- [ ] **Response schemas, § 1** — gap 1 in `crates/nvs-cli/src/openapi.rs`'s module doc. The handler's
      declared return type is not on the row: `struct Route` is `routes.rs:292` and is filled at
      `routes.rs:577`, and rendering a class as a schema is ADR 0071's codec, which is why this slice is
      the one that decides whether `openapi.rs` gains a `components.schemas` section — `api_diff.rs`
      already walks one.
- [ ] **Summary and description, § 1** — gap 3, and the cheapest of the six: doc comments are parsed
      already, so this is a field on the row and two lines in `operation` (`openapi.rs`, `fn operation`).

## Backlog

- Gaps 2 and 4-6 of `crates/nvs-cli/src/openapi.rs`'s module doc — request bodies, `info.version`, a type
  outside `schema`'s list.
- `info.version` needs a key `nvs.toml` does not have; M6's reader owns it (`main.rs`'s `origin` doc).
- `nvs api diff` has no suppression mechanism and must not grow one in v1 — ADR 0085 *Consequences*.
- ADR 0085 *Verification*'s qualifier row: a `secret` property in a response class is a compile error at
  the handler, and nothing asserts it yet.
- `[context] adrs` in `docs/agent/loop-goal.toml` wants `0085 §§ 1-4`.
