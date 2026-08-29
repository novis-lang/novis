# Handoff

## State

**ADR 0077 § 4's link half folds, and `Core\Router::url` answers.**
`examples/routes.nvs` prints `url=/users/7`: `crate::links` (`crates/nvs-types/src/links.rs`)
records every `url`/`urlAbsolute` site whose argument 0 folded to a literal, and
`links::resolve` (`links.rs:165`) looks each one up against the finished table from
`check_program` (`crates/nvs-types/src/check.rs:149`) — **after** the file loop, because a
`url` call in the entry file routinely names a route § 5's scan finds later. Two new codes:
`E0754` a literal name no `#[Route]` claims, `E0755` a `$params` literal covering less than
the path's captures. The resolved route rides across as `ExprInfo::RouteLink { pieces,
absolute }`, recorded **over** the `ExprInfo::Call` the walk already wrote at that span, so a
*computed* name keeps its `Call` and throws — § 4's own rule, with no second state.

**§ 2's grammar is still read exactly once.** `routes::link_pieces` (`routes.rs:419`) is the
only reader outside `parse_path`, and it answers `Vec<UrlPiece>`;
`UrlPiece::prepared` (`expr_table.rs:261`) writes that out in the format
`nvs_stdlib::router::link` defines, `nvs-ir` emits it as argument 0 of a `CoreCall`
(`lower_route_link`, `crates/nvs-ir/src/lower/expr.rs:2488`), and `substitute`
(`crates/nvs-stdlib/src/router.rs:236`) reads it back with a `split` and a byte test. No
runtime parser of a path exists to disagree with the compiler's.

**`python tools/verify.py` is red on something this session did not cause, and it blocks the
whole gate.** `cargo test -p nvs-ir --test refusals`'
`every_refusal_is_a_diagnostic_or_decided` reports **17 unattributed refusals**, all
long-standing `nvs-ir` lowering gaps in `call.rs`, `control.rs`, `convert.rs`, `exception.rs`,
`expr.rs`, `mod.rs` and `stmt.rs`. Confirmed pre-existing by stashing this session's work and
re-running at `7e39dd2`. `holes.py --unattributed` attributes a refusal to an **open item in
`docs/agent/loop-goal.md`**, and goal 1's item list names none of these where M4's did — so
this is the goal switch's residue, not a regression in the tree. The test forbids the
allowlist by name; the fix is an item or a standing decision in the goal file.

**The acceptance check is red on its last line only, and for one missing thing.**
`urlAbsolute` throws (`router.rs:300`): ADR 0102 § 6's origin is configured and never sniffed,
`[app] origin` lives in `nvs.toml`, and **`nvs.toml` has no reader at all** — it is M6's, per
`nvs_syntax`'s own module docs. Everything else in `examples/routes.nvs` matches
`docs/agent/loop-goal.toml`'s `want`.

## Next group

**Two file sets, and the first item stands alone: `docs/agent/loop-goal.md`, then
`crates/nvs-stdlib/src/router.rs`, `crates/nvs-cli/src/main.rs`, `tests/conformance/core/`.**

- [ ] **The 17 unattributed refusals get an owner, so `verify.py` can go green again.**
      `python tools/holes.py --unattributed` is the list and
      `crates/nvs-ir/tests/refusals.rs:144` is the gate. Read what M4's goal file said about
      them before writing anything: these are `nvs-ir`'s declared lowering gaps, so the answer
      is one open item in `docs/agent/loop-goal.md` naming the set, or a § *Standing decisions*
      paragraph — **not** an `ALLOWLIST` entry, which that test's own message forbids.
- [ ] **`[app] origin` reaches `urlAbsolute`.** ADR 0097 § 3 makes the origin per mount falling
      back to `[app] origin`, and ADR 0102 § 6 refuses every other source. There is no mount and
      no config reader off the command line, so this slice is: read `nvs.toml` beside the entry
      file for `[app] origin` only, hand it to the runtime the way `_ctx` already carries
      per-run state, and keep § 3's "a unit that resolves none is an error" as the throw at
      `crates/nvs-stdlib/src/router.rs:303`. Decide *where the value lives* first — the
      `nvs_helper!` bodies take `_ctx` and nothing else. `examples/` then needs the `nvs.toml`
      the check's `absolute=https://example.test/users/7` implies. ADR 0097 § 3, ADR 0102 § 6.
- [ ] **A `.nvst` case over the two refusals.** An unknown literal name (`E0754`) and a
      `$params` that covers no capture (`E0755`), both `--EXPECTF-ERROR--`; plus the positive
      twin — a `{page?}` left out of `$params` is *not* refused, because its whole segment is
      dropped. `crates/nvs-types/src/links.rs:165` is where both messages are written.
      ADR 0077 § 4.
- [ ] **A `$params` key naming no capture becomes a query string.** § 4's other half, and
      `crates/nvs-stdlib/src/router.rs:236` is where it lands — the pieces name every capture,
      so what is left over is the query. `crates/nvs-stdlib/src/uri.rs`'s query builder is the
      implementation; do not write a second one. `links.rs`' gap 1 owns why the *refusal* half
      waits on `#[Query]`.

## Backlog

- `urlAbsolute`'s mount prefix: `url` prepends one too (ADR 0097 § 3) and there is no mount.
- `nvs_types::links` gap 2 — a named argument (`url(name: "…")`) records no site and throws.
- `nvs_types::routes` gap 2 — only the first `#[Route]` on a method becomes a row (ADR 0110 § 1).
- ADR 0085's OpenAPI emitter reads `ExprTypeTable::routes`; nothing does yet (`loop-goal.toml`
  stage 4).
- `Core\Router::match`/`methodsFor` and `Core\Router\Match` — out of scope by standing decision.
