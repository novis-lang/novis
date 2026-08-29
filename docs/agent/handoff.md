# Handoff

## State

**ADR 0077 § 1's `#[Core\Route]` is a recognized name with its own payload check.** It is on
`nvs_types::derive::ATTRIBUTES` (`crates/nvs-types/src/derive.rs:78`) and its roster is the new
`crates/nvs-types/src/routes.rs`, whose module doc owns the three gaps. The roster *walk* moved
out of `commands.rs` into `nvs_types::attributes::check_roster` and is now the one walk behind
`#[Test]`, `#[Command]`, `#[Option]` and `#[Route]`; `OptionTy` (`crates/nvs-types/src/testing.rs:157`)
is `pub(crate)` and gained an `Enum(&'static str)` row, because § 1's payload is not all `string`.

**`Core\Http\Method` is declared nowhere**, so `method`'s roster row interns to nothing and a case
of the *wrong* enum is admitted — `nvs_types::routes` gap 1, and the playbook bullet this session
added says why nothing below the roster catches it either.

**ADR 0086 § 6's two local compile errors land**: `E0745` when two options of one `#[Command]`
claim one spelling (the long form defaults to the parameter's own name, so an explicit `long:`
over a sibling's name collides), and `E0746` when an `#[Option]`'s parameter has no conversion
from `string`. Both are `commands::check_class_commands`, hooked at
`crates/nvs-types/src/check.rs:230`; `commands::converts_from_string` is the one home for
ADR 0077 § 3's conversion roster and is what the route table's own pass should read.

**The driver's failing acceptance check is still `examples/routes.nvs`**, unchanged and with no new
errors: `Core\Router` has no `url`/`urlAbsolute` and there is no router module under `nvs-stdlib`.
`App\Users` is still never scanned. Note its frozen `want` needs `url=/users/7` and
`absolute=https://example.test/…`, so the member rows alone do not close it — the § 5 scan, the
table and ADR 0102 § 6's configured origin are all in front of that line.

**`python tools/verify.py` is still red at `crates/nvs-ir/tests/refusals.rs:179`** — the same 17
unattributed lowering refusals the goal switch orphaned, untouched by this session and unrelated
to it. It is the first failure, so the gate stops there and never reaches the `.nvst` trees or
clippy — `nvs test tests/conformance` was run by hand instead and is 881 green, and
`cargo test -p nvs-types` is green including the two new suites.

## Next group

**One file set: `crates/nvs-stdlib/src/registry.rs`, a new `crates/nvs-stdlib/src/router.rs`,
`crates/nvs-types/src/routes.rs`.** Take them in this order — the enum unblocks the roster row the
first slice of this session had to leave interning to nothing.

- [ ] **`Core\Http\Method` joins `ENUMS`** — the roster is `crates/nvs-stdlib/src/registry.rs:853`
      and `CoreEnum`'s shape is `crates/nvs-stdlib/src/registry.rs:824`. Declared beside the member
      that takes it, which is the rule that roster states, so it lands with the router module rather
      than on its own. It is what `Core\Request::method` answers with and what
      `crates/nvs-types/src/routes.rs:@OPTIONS` already names. ADR 0077 § 1.
- [ ] **`Core\Router::url`/`urlAbsolute` rows** — `CLASSES` is
      `crates/nvs-stdlib/src/registry.rs:709`, and the four edits a `Core` member owes are in
      `docs/agent/conventions.md`. ADR 0077 § 4 and ADR 0102 § 6 (the origin is configured per
      mount, never read from a header). The moment `url` exists the § 5 scan runs and `App\Users`
      is checked for the first time — its three `#[\Core\Route]`s are recognized now, so what it
      hits next is whatever else that file names, not `E0726`.
- [ ] **The route table itself** — ADR 0061 § 3's scan filtered by `#[Route]`, which is what
      `url("Users::show", ["id" => 7])` reverses into `/users/7`. This is the slice that closes the
      acceptance check, and it is the one that can report §§ 1-3's four remaining compile errors.

## Backlog

- 17 unattributed lowering refusals, `crates/nvs-ir/tests/refusals.rs:179` — verify's first failure.
- A ternary's arms are checked with `None`, so `$b ? 0 : 1` is `int` at a `uint` return —
  `crates/nvs-types/src/expr/mod.rs:365`.
- An `#[Option]` on a method carrying no `#[Command]` is not refused — `nvs_types::commands` gap 2.
- `#[Route]`'s `path`/`method` are not required — `nvs_types::routes` gap 3, the table pass's.
- `Core\Command::run` and `Core\Router::match` are later goals' — `docs/agent/loop-goal.md`.
