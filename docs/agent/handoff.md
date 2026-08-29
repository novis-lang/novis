# Handoff

## State

**Goal 1 Stage 2's route table is closed at the acceptance check**: all nine tests the
`nvs-types (the route table)` check names are on disk and green in `crates/nvs-types/tests/routes.rs`,
including the two this session wrote. Item 4's `#[Query]` half is done; its `#[Access]` half is not, and
is the next group.

**`#[Query]` is `nvs_types::derive::QUERY` on `ATTRIBUTES`**, read by `routes::query_params` beside
`check_captures` — a second reading of the same parameter list, never a third reading of the path. The
keys land on `Route::query` and cross into `nvs-ir` on the row, because the only question asked about
them is ADR 0102 § 6's and it is asked after every file is walked. § 3's "the same type list as a path
capture" is `crate::commands::converts_from_string`, read and not copied, so all three passes over that
list ask one question.

**ADR 0102 § 6's refusal is `E0759` in `links::declared`** — a `$params` key naming neither a capture nor
a declared `#[Query]` parameter. It short-circuits behind `E0755`, so one misspelled key that is both
faults is reported once, at the half that says what to write.

**Two module-doc gaps moved.** `links.rs`' gap 1 is closed and deleted. `routes.rs`' gap 1 was stale — it
claimed nothing reverses the table, which `links.rs` has done for some time — and now names the real hole:
a `#[Query]` outside a `#[Route]` method is not refused, which is `commands.rs`' gap 2 exactly.

**Item 15 in `docs/agent/loop-goal.md` still owns M4's seventeen `nvs-ir` lowering refusals**, unchanged
and standing by design; the ratchet is `CEILING` at `crates/nvs-ir/tests/refusals.rs:66`.

**Manifest gap:** `orient.py` printed no ADR 0096 section, and the whole next group is that ADR. Add
`0096` §§ 1, 1a, 2 and 3 to `[context] adrs` in `docs/agent/loop-goal.toml`.

## Next group

**One file set: `crates/nvs-types/src/derive.rs`, `src/routes.rs`, `src/attributes.rs`,
`crates/nvs-diagnostics/src/lib.rs` and `tests/routes.rs`.** All three slices are item 4's remaining
half, `#[Access]`, and no acceptance check names them — the item does.

- [ ] **`#[Access]` joins the roster and its payload is checked.** ADR 0096 §§ 1 and 1a. The constant
      goes beside `QUERY` at `crates/nvs-types/src/derive.rs:150` and on `ATTRIBUTES` at
      `crates/nvs-types/src/derive.rs:81`; its option roster is a `const` beside
      `crates/nvs-types/src/routes.rs:105`'s `OPTIONS`, dispatched from the `else if` chain at
      `crates/nvs-types/src/attributes.rs:138`. § 1a is the one to read first: it says which field is
      required and which is deliberately *not* checked.
- [ ] **A `#[Route]` with no sibling `#[Access]` does not compile.** ADR 0096 §§ 1 and 3 — an omission is
      an error and never a default. The question is asked where the method's attributes are already in
      hand, `collect_route` at `crates/nvs-types/src/routes.rs:227`; the next free code is `E0760`, after
      `crates/nvs-diagnostics/src/lib.rs:2034`. Every fixture in `crates/nvs-types/tests/routes.rs` then
      needs a sibling attribute, and `route_src` at `crates/nvs-types/tests/routes.rs:32` is where most
      of them are built.
- [ ] **The decision lands on the `Route` row.** ADR 0102 § 8: the *dispatcher* enforces the access
      decision and the server never does, so the row is what carries it across — beside `Route::query` at
      `crates/nvs-types/src/routes.rs:142`, the same reasoning and the same channel.

## Backlog

- A `#[Query]` outside a `#[Route]` method is not refused — `nvs_types::routes`' gap 1, and
  `nvs_types::commands`' gap 2 is the identical hole for `#[Option]`.
- Only the first `#[Route]` on a method becomes a row — `nvs_types::routes`' gap 2, ADR 0110 § 1.
- Item 6, the command table: `a_command_table_is_built_from_the_program_enumeration` and
  `a_duplicate_command_name_is_a_diagnostic` — `nvs_types::commands`' gaps 1 and 3.
- A named argument to `Core\Router::url` is not folded — `nvs_types::links`' gap 1.
- `E0759` has no `.nvst` reject case; `tests/conformance/reject/a-route-link-refuses-an-unknown-name-and-an-uncovered-capture.nvst` is the file it belongs beside.
- Item 15's seventeen `nvs-ir` lowering refusals — `docs/agent/loop-goal.md`.
