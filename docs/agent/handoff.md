# Handoff

## State

**ADR 0077's route table is collected, and § 5's opt-in now has both of its callers.**
`Core\Router::url`/`urlAbsolute` answer `is_program_scan`
(`crates/nvs-hir/src/requires.rs:606`, roster at `:595`), so a program whose only
discovery call is a link performs ADR 0061 § 3's scan and `examples/routes.nvs` loads
`App\Users`. `::match` is deliberately off that roster until the member exists.

**`nvs_types::routes` is now two passes, like `nvs_types::commands`.**
`check_class_routes` (`crates/nvs-types/src/routes.rs:135`) adds one row per `#[Route]`
from `crate::check`'s per-class walk into `Env::routes`, threaded across every file;
`check_table` (`:257`) then reports the two errors that are questions about the whole
enumeration — `E0748` a route declared twice (keyed by verb plus § 2's path *shape*, so
`{id}` and `{userId}` collide) and `E0749` a `name` claimed twice, both at the row that
arrives second in load order. `E0747` refuses a `#[Route]` that gives no `path` or no
`method`, which closes that module's old gap 2: a roster says what a field may hold and
*required* is a fact about the row.

**Nothing reverses the table yet**, so `examples/routes.nvs` still throws at the first
`url` call and the driver's acceptance check still fails there. The rows exist in
`check_program` and are dropped at the end of it; `crate::expr_table` is the channel they
should travel to `nvs-ir` by, rather than a second return value.

**`python tools/verify.py` is still red at `crates/nvs-ir/tests/refusals.rs:179`** — the
same 17 unattributed lowering refusals, untouched by this session and older than it. The
gate stops there and never reaches the `.nvst` trees or clippy, so both were run by hand:
`nvs test tests/conformance` is 883 green and `cargo clippy -p nvs-types -p nvs-hir -p
nvs-diagnostics --all-targets` is clean. The five `.nvst` cases that write a `#[Route(…)]`
are userland `type Route` aliases and are untouched by the new pass — the nominal match
doing its job. The scan-to-table path was checked end to end on a scratch program: a
`url` call, an autoload root, and a routed class nothing names, which reports `E0749` in
the scanned file.

## Next group

**One file set: `crates/nvs-types/src/routes.rs`, `crates/nvs-types/src/expr_table.rs`,
`crates/nvs-stdlib/src/router.rs`.** In this order — the last one is what closes the
driver's acceptance check.

- [ ] **§ 2's path grammar and § 3's `{param}`-to-parameter check** — in `collect_route`
      (`crates/nvs-types/src/routes.rs:163`), which already holds the attribute and can be
      handed the `MethodMember` `check_class_routes` (`:135`) walks. The parameter's type
      is `env.signatures`, and the conversion roster is
      `crate::commands::converts_from_string` (`crates/nvs-types/src/commands.rs:230`) —
      never a second copy. ADR 0077 §§ 2-3.
- [ ] **The rows reach `nvs-ir`** — `RouteTable` is dropped at the end of
      `check_program` (`crates/nvs-types/src/check.rs:141`). `record_property_types`
      (`crates/nvs-types/src/check.rs:152`) is the worked shape for handing a
      compile-time fact to lowering through `crate::expr_table`.
- [ ] **`Core\Router::url("Users::show", [...])` resolves its literal name** — the fold is
      ADR 0057's, and the runtime path is `nvs_core_router_url`
      (`crates/nvs-stdlib/src/router.rs:174`), whose refusal message
      (`crates/nvs-stdlib/src/router.rs:162`) is what the acceptance check currently
      prints. One implementation, per the goal's standing decisions: the fold and the
      runtime member share the substitution.

## Backlog

- `Core\Router::url` laundering nothing: percent-encoding and the mount prefix —
  `nvs_stdlib::router`'s gap 2, ADR 0077 § 4.
- `urlAbsolute`'s configured origin, per mount falling back to `[app] origin` —
  ADR 0102 § 6.
- `Core\Router::match`/`methodsFor` and `Core\Router\Match` — out of scope by
  `docs/agent/loop-goal.md` § *Standing decisions*.
- `Core\Request` and the verb parse that turns an unrecognized method into a 501 —
  `nvs_stdlib::router`'s gap 4.
- The spec's `array<string, mixed>` spellings name a type the grammar has no form for —
  `docs/spec/01-core-library.md:918` and `:1058`.
- The 17 unattributed lowering refusals at `crates/nvs-ir/tests/refusals.rs:179`, which
  keep `verify.py` red for every session.
