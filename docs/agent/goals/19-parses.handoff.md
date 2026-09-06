# Handoff

## State

**Goal 19 — a class a string names, at one contract — has just started; nothing of it has landed yet.**
Goal 18's whole list is this goal's Stage 1 floor. This is the chain's last entry.

The scope line matters here, and it is a *narrowing* the user chose deliberately: **this goal does not
give `as` a class-building meaning.** The proposal it came from did, and that half was rejected — a
session that finds itself editing the conversion table, `expr as T`'s lowering, or `rule:expressions/nullable-conversion-availability`'s class
row has left the goal. The three reasons are in *Standing decisions* and are not to be re-derived.

What the goal is buying is one sentence made sayable. Four binding surfaces — a route `{capture}`, a
`#[Query]`, a command argument, a command option — all ask `converts_from_string`
(`crates/nvs-types/src/commands.rs:694`) whether a type can be built from text, and its class arm is
`*name == QName::parse(r"Core\Uuid")`. The roster is a roster only because the language had no way to
describe what `Core\Uuid` is. `Parses` describes it; the arm becomes a predicate; `Core\Uuid` reaches the
door through the same contract as everyone else and **not one line of `crates/nvs-stdlib/src/uuid.rs`
changes** — its rows at `:139` and `:148` already are the contract.

## Next group

**Stage 2: the contract** — one file set: `crates/nvs-hir/src/interfaces.rs`,
`crates/nvs-types/src/conformance.rs`.

- [ ] **`Parses` joins the reserved roster** — `crates/nvs-hir/src/interfaces.rs:46`, the
      `("Comparable", &[]), ("Stringable", &[])` list, plus a name constant beside `COMPARABLE` (`:55`)
      and `STRINGABLE` (`:59`). No type parameters; those two take none either.
- [ ] **Conformance owes `parse`** — `crates/nvs-types/src/conformance.rs:41` is the walk, and its own
      test at `:570` (`class Money implements Comparable {}` naming `compareTo`) is the shape to copy. A
      class claiming `Parses` with no `parse` names the member and the interface.
- [ ] **`tryParse` is a default body on the interface, not a second required member** — `rule:expressions/try-parse`
      condition 2, whose reason is CVE-2024-5458: `tryParse` *is* `parse` plus a caught throw. Requiring
      both would hand that failure to every implementor. The signature is
      `parse(tainted string $s): static` and `tryParse(tainted string $s): ?static`; `static` is what
      makes an implementor's `parse` answer its own class.

## Backlog

- Stage 3 (`converts_from_string`'s class arm becomes the contract; `implements_parses` beside
  `implements_comparable` at `crates/nvs-stdlib/src/registry.rs:2311`; the seed two lines below the
  `Comparable` one at `crates/nvs-types/src/core_lib.rs:83`; the three roster diagnostics at
  `commands.rs:660`, `routes.rs:1713` and `routes.rs:1799` stop reciting a name) is the second group. It
  shares `nvs-types` with stage 2 and adds `nvs-stdlib`'s two files.
- Stage 4 (the runtime arm) is the third, and it closes two gaps that are already written down as gaps
  with the same diagnosis — `crates/nvs-runtime/src/routes.rs:57` gap 2 and
  `crates/nvs-runtime/src/commands.rs:41` gap 1, both "the conversion exists as a `Core` member and what
  is missing is the arm". `decimal` rides along because it is named in both and is one arm in each of the
  two matches already being edited. The enum and literal-union half of commands gap 1 is **not** in
  scope.
- Stage 5 is `examples/parses.nvs`, the OpenAPI schema arm (`crates/nvs-cli/src/openapi.rs:372` — a
  `Parses` class is `{"type": "string"}` and `Core\Uuid` keeps `format: uuid` as a documentation hint
  over it), the reference pages, and the diagnostic corpus.
- **One question settled in stage 2 rather than deferred**: whether a plain `string` argument assigns to
  a `tainted string` parameter. If it does not, `parse`'s parameter takes `rule:security/unclassified-parameter-refuses-tainted`'s
  `Qual::Contagious` admission instead, which is what every `Core` member in this position already uses.
  Decided-and-recorded in `rule:security/tainted-qualifier`'s body, never `BLOCKED`.
- **When this goal's last check goes green the driver takes goal 20** — `rule:config/cache-shared-is-the-grant-over-the-configured-store`'s Unix sockets — and
  then goal 50, the dossier, which appends everything after it. `docs/agent/goals/chain.toml` is the
  schedule and this does not restate it.
