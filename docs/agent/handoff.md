# Handoff

## State

**Stage 4's document carries ADR 0085 § 1's two text rows and its response row.** `nvs build --openapi`
emits `summary`/`description` from the handler's doc comment and a `200` response whose schema is the
handler's declared return type; `crates/nvs-cli/tests/openapi.rs` is 12 tests, all green. § 2's `#[Api]`
is checked against the declaration but supplies nothing to the document yet — that is the next group.

**The stage-4 acceptance check is not failing.** The pack's failure is the previous run's last record
(playbook bullet above); the command passes by hand at this commit and `loop.py` has been fixed since
before this run started. Nothing in the tree is owed for it.

**A doc comment is read out of the source text, not off a token** — `nvs_syntax`'s lexer preserves no
trivia at all, and ADR 0099's trivia layer is M10's. `nvs_types::routes`' `doc_comment` looks at the bytes
in front of `ClassMember::span`, which is why it finds the comment above `#[Route]` and why `//` and
`/* */` above a handler are not doc comments. Its own comment owns that and names what replaces it.

**The response schema stops at `schema()`'s scalar list on purpose.** A class renders as the empty schema
because its wire fields are ADR 0071's codec roster and *not* `ClassSignature::properties`: that map holds
private properties too, and publishing those is exactly what `#[Json\Derive]` exists to decide. The
emitter's module doc gap 1 now says so, so the next session does not reach for the cheap map.

**Orientation gap, ninth session running:** `[context] adrs` still does not carry `0085 §§ 1-4`, and this
session sliced § 1 by hand again.

## Next group

**Everything `#[Api]` already checked but drops, § 2 — gap 3 in `crates/nvs-cli/src/openapi.rs`'s module
doc. Shared file set:** `crates/nvs-types/src/routes.rs`, `crates/nvs-cli/src/openapi.rs`,
`crates/nvs-cli/tests/openapi.rs`, `crates/nvs-cli/tests/fixtures/api/`.

- [ ] **`tags` and `security` reach the document, § 2** — the two fields that are arrays of plain strings,
      so one folding helper serves both. `API_OPTIONS` is `crates/nvs-types/src/routes.rs:659`, the walk
      that reads them is `check_api` at `:750`, the row is `Route` at `:321` (add beside `summary`), and
      the operation object is `operation` at `crates/nvs-cli/src/openapi.rs:147`. `tags` is an operation
      member; `security` is a list of `{scheme: []}` objects, and `check_api`'s doc comment already owns
      why no scheme roster exists to check a name against.
- [ ] **`errors` and `example`, § 2** — the two structured ones. Each `errors` entry is a status and a
      class, so it becomes another key of `responses` (`crates/nvs-cli/src/openapi.rs:186`, which today
      writes `200` and says why it writes nothing else); `example` becomes the `200` content's `example`
      member. `check_api_errors` and `check_api_example` (`routes.rs:@check_api_errors`) already walk both
      payloads — carry what they read rather than walking a second time.
- [ ] **An object response schema, § 1** — gap 1. `DerivedCodec` (`crates/nvs-types/src/derive.rs:178`)
      holds `fields: Vec<DerivedField>` in declaration order with `key`, `ty` and `nullable`
      (`:195`) — everything a JSON Schema object needs and nothing private. It lives on
      `crate::ExprTypeTable`, which the emitter does not have, so the schema is built in `nvs-types` and
      rides on the row: `openapi.rs`'s module doc is explicit that a missing fact is a gap in the table.

## Backlog

- `info.version` (gap 4) waits on M6's `nvs.toml` reader; nothing in the language declares a version.
- Request body schemas (gap 2) also need *which parameter is the body*, which no row answers — ADR 0085
  § 1 names the `#[Json\Derive]` codec of "the body parameter" and nothing marks one.
- `docs/agent/loop-goal.toml`'s `[context] adrs` is missing `0085 §§ 1-4`.
- Stage 5's depth pass is the fallback whenever this group is blocked (`docs/agent/loop-goal.toml`).
- `Core\Router::match` and the server stay out of scope (goal § *Standing decisions*).
