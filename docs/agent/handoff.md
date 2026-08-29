# Handoff

## State

**ADR 0096 is landed except for § 4's `csrf` opt-out.** The decision now rides on the row:
`Route::access` is `Option<String>` (`crates/nvs-types/src/routes.rs:264`), filled from the sibling
`#[Access]` that `check_access_declared` hands back, and resolved by `access_name`
(`routes.rs:869`) to `Class::MEMBER` with the class side through `crate::expr::resolve_class_expr`
— so `Audience::Public` under a `use Core\Audience;` is `Core\Audience::Public` on the row. ADR 0102
§ 8 is why: the dispatcher enforces, so what the compiler guarantees was written has to cross.
`None` means the declaration was already refused, never a name that resolved to nothing.

**A second `#[Access]` on one method is refused**, `E0763` from `check_one_access`
(`routes.rs:211`), called from `attributes::check_declaration`'s method arm — the walk that holds a
method's whole attribute list, since a repeat is a fact about the list rather than about a payload.
It is asked of every method, not only of a route's, and the group form `#[A, B]` is covered because
the walk flattens groups.

**The one part of ADR 0096 still open is § 4**: `csrf: false` on a route whose every verb is safe
does not yet refuse. It does *not* have to wait on the module doc's gap 2 — `check_class_routes`
(`routes.rs:340`) holds `m.attributes`, so "every verb this method declares" is a walk over the
method's `#[Route]`s even while only the first of them becomes a row.

**The acceptance check that fails is item 6's**, `a_command_table_is_built_from_the_program_enumeration`
— ADR 0086 § 6's command table, untouched by this group and open by design, not a regression.

**Item 15 in `docs/agent/loop-goal.md` still owns M4's seventeen `nvs-ir` lowering refusals**,
unchanged and standing by design; the ratchet is `CEILING` at `crates/nvs-ir/tests/refusals.rs:66`.

## Next group

**One file set: `crates/nvs-types/src/routes.rs`, `crates/nvs-types/tests/routes.rs` and
`crates/nvs-diagnostics/src/lib.rs`** — the same set the last group used. No acceptance check names
these; the first slice closes ADR 0096, the second closes the module doc's own gap 2.

- [ ] **`csrf: false` on a route whose every verb is safe does not compile.** ADR 0096 §§ 1a and 4:
      the four unsafe verbs are `Post`, `Put`, `Patch` and `Delete`, so a `csrf: false` beside a
      method whose every `#[Route]` names one of the others has nothing to opt out of. The verbs are
      the method's, not the row's, so ask it in `check_class_routes` (`routes.rs:340`) where
      `m.attributes` is in hand rather than in `collect_route` (`routes.rs:416`); `CSRF` is already
      `routes.rs:110` and on `ACCESS_OPTIONS` (`routes.rs:137`), and `verb_of` (`routes.rs:836`)
      reads a verb out of one `#[Route]`. Next free code is `E0764`, after
      `crates/nvs-diagnostics/src/lib.rs:2079`.
- [ ] **Every `#[Route]` on a method becomes a row.** The module doc's gap 2 (`routes.rs:81`):
      `check_class_routes` takes `crate::testing::attribute_named` (`crates/nvs-types/src/testing.rs:859`),
      which answers with the *first* match, so ADR 0046 § 3's repetition contributes one row instead
      of two and
      [ADR 0110](../adr/0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md) § 1's
      "repetitions share a `name` when they share a `path`" has nothing to except. Walk all matches,
      and § 1's exception is then a question `check_table` (`routes.rs:911`) can ask.
- [ ] **A `#[Query]` outside a `#[Route]` method is refused.** The module doc's gap 1
      (`routes.rs:75`), ADR 0102 § 3 — a recognized marker that binds nothing where it is written is
      the mistake the closed roster exists to prevent. `query_params` (`routes.rs:764`) is only
      reached from a route method; the refusal belongs to the walk that sees every method's params.

## Backlog

- ADR 0086 § 6's command table — item 6's acceptance check, `docs/agent/loop-goal.toml`.
- M4's seventeen `nvs-ir` lowering refusals — item 15, `docs/agent/loop-goal.md`.
- `Core\Router::match` and `Core\Command::run` stay out of scope — `loop-goal.md` § *Standing decisions*.
