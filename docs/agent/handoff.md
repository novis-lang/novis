# Handoff

## State

**ADR 0077 §§ 2-3 are checked, and § 5's table now crosses into `nvs-ir`.**
`collect_route` (`crates/nvs-types/src/routes.rs:@collect_route`) reads a path twice:
`parse_path` splits it into `Capture`s against § 2's grammar, and `check_captures` asks
§ 3 of the method the attribute is attached to. Four new codes — `E0750` a path § 2 does
not admit (one code for the whole grammar, the message naming which way), `E0751` a
capture naming no parameter, `E0752` a capture's parameter type with no conversion,
`E0753` a `{name?}` whose parameter has no default. The conversion roster is
`crate::commands::converts_from_string` read, never copied; the one row this pass adds is
that a `{name...}` arrives as the single `tainted string` § 3 says it does, so it binds a
`string` and no other type. `shape` now keeps the capture *form* (`{}`/`{?}`/`{...}`),
because § 2's precedence makes the three different trie nodes and erasing the form would
call a legal pair a duplicate.

**The rows reach lowering on `ExprTypeTable::routes`.** `check_program`
(`crates/nvs-types/src/check.rs:141`) records the finished table after `check_table` runs,
`Route`/`RouteTable` are `pub` and re-exported from `nvs_types`, and
`RouteTable::named(name)` is the one lookup § 4 reverses by — unambiguous because `E0749`
already refused two rows claiming one name.

**Nothing reads them yet, so the driver's acceptance check still fails at
`examples/routes.nvs`**: `Core\Router::url` throws `no such route`
(`crates/nvs-stdlib/src/router.rs:162`). That is item 1 of the next group and it is the
whole remaining distance to a green check. ADR 0057's fold has **no implementation site
anywhere yet** — the four `0057-intrinsic` mentions in the tree are all `nvs-stdlib`
module docs — so the next session is creating that site, not extending one.

## Next group

**One file set: `crates/nvs-types/src/expr/calls.rs`,
`crates/nvs-types/src/expr_table.rs`, `crates/nvs-ir/src/lower/`,
`crates/nvs-stdlib/src/router.rs`.** In this order; the first two are one slice's worth
and close the acceptance check between them.

- [ ] **`Core\Router::url`'s literal name folds to its path template.** The trap first:
      **this cannot run inside the per-file walk.** A `url` call in the entry file names a
      route § 5's scan finds in a later file, so the table is complete only after the loop
      in `check_program` — the fold is a second pass beside `check_table`
      (`crates/nvs-types/src/check.rs:141`), over call sites the walk *recorded*, not a
      lookup made where the call is checked. So: in the `ExprKind::StaticCall` arm
      (`crates/nvs-types/src/expr/calls.rs:171`) record the span, the folded argument 0 and
      the `$params` keys of every `Core\Router::url`/`urlAbsolute` call; then resolve each
      against `RouteTable::named` after the loop, refusing an unknown name and a `$params`
      that does not cover the path's captures. `parse_path` (`crates/nvs-types/src/routes.rs`)
      is what splits the path — do not write a second splitter. The resolved template rides
      to `nvs-ir` as a new `ExprInfo` variant (`crates/nvs-types/src/expr_table.rs:226`;
      "one new variant per question" is that module's stated pattern). ADR 0077 § 4,
      ADR 0057.
- [ ] **Lowering emits the template, and the runtime substitutes it.** `nvs-ir` reads the
      new `ExprInfo` at the call's span and passes the path template where the name was
      written; `nvs_core_router_url` (`crates/nvs-stdlib/src/router.rs:174`) then
      percent-encodes each substituted value and prepends the mount prefix, and
      `no_such_route` (`:162`) becomes unreachable from a compiled program. § 4 makes
      `url` the launderer for the URL-path sink, so the encoding is the member's job and
      not the caller's. This is what turns `examples/routes.nvs` green.
- [ ] **A `.nvst` case over the two refusals** — an unknown route name, and a `$params`
      that misses a capture — under `tests/conformance/reject/`, `--EXPECTF-ERROR--`.

## Backlog

- **Only the first `#[Route]` on a method becomes a row** (`routes.rs` known gap 2):
  `testing::attribute_named` answers with one attribute, so ADR 0046 § 3's repetition is
  one row, and ADR 0110 § 1's shared-`name` exception has nothing to except.
- `Core\Router::match` is off `nvs_hir::requires`' scan roster until the member exists —
  goal § *Standing decisions*.
- ADR 0096: a `#[Route]` without a sibling `#[Access]` does not compile. Nothing checks it.
- ADR 0085's OpenAPI document is generated from this table; nothing reads it yet.
- The 1000-case conformance corpus count, `docs/implementation-plan.md` *Open now*.
- `python tools/gaps.py` and `python tools/holes.py` are the standing worklists.
