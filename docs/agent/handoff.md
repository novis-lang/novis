# Handoff

## State

**ADR 0077's router has its first two pieces on disk, both in the new
`crates/nvs-stdlib/src/router.rs`.** `Core\Http\Method` is `ENUMS`' ninth row
(`crates/nvs-stdlib/src/registry.rs:855`): ADR 0074 § 7's eight verbs, safe ones first so
ADR 0096 § 4's CSRF set is the contiguous tail from `Post` (value 4) on, `CONNECT`
deliberately absent. `Core\Router` is in `CLASSES` with § 4's link half only — `url` and
`urlAbsolute`, `params` declared `array<mixed>` (see the playbook bullet on `array<K, V>`).

**`nvs_types::routes`' gap 1 is closed.** The `method` roster row
(`crates/nvs-types/src/routes.rs:56`) now interns to a real type, so a case of the *wrong*
enum is refused — `crates/nvs-types/tests/routes.rs`'s
`a_case_of_another_enum_is_refused_at_the_method_option` is the new twin.
`OptionTy::intern`'s `None` arm (`crates/nvs-types/src/testing.rs:174`) is now reachable
by no row and its doc says so.

**`examples/routes.nvs` compiles and reaches its `echo`s, then throws.** Both `url` calls
resolve; the bodies are one shared `no_such_route` refusal, because the route table is not
built and every name is unknown. The driver's acceptance check therefore still fails, one
phase later than before — its `want` needs `url=/users/7` and
`absolute=https://example.test/…`, which is the whole next group plus ADR 0102 § 6's
configured origin. `App\Users` is still never scanned: `Core\Router::url` is not yet one of
the calls that opt a program into ADR 0061 § 5's scan.

**`python tools/verify.py` is still red at `crates/nvs-ir/tests/refusals.rs:179`** — the
same 17 unattributed lowering refusals, unchanged and untouched by this session. It is the
first failure, so the gate never reaches the `.nvst` trees or clippy; both were run by
hand instead — `nvs test tests/conformance` is 883 green, `cargo clippy -p nvs-stdlib -p
nvs-types --all-targets` is clean, and `cargo test -p nvs-stdlib -p nvs-types` is green.

## Next group

**One file set: `crates/nvs-hir/src/requires.rs`, `crates/nvs-types/src/routes.rs`,
`crates/nvs-types/src/commands.rs` (as the model, not to edit).** In this order — nothing
can build a table over classes the scan never loaded.

- [ ] **`Core\Router::url`/`urlAbsolute` opt a program into § 5's scan** —
      `is_program_scan` is `crates/nvs-hir/src/requires.rs:587` and the scan site it gates
      is `crates/nvs-hir/src/requires.rs:352`. Today only `Core\Program::implementing<T>()`
      answers it, so `examples/routes.nvs` never loads `App\Users`. ADR 0077 § 5,
      ADR 0061 § 3.
- [ ] **The route table is collected from the scanned classes** — one row per
      `#[Core\Route]`, keyed by `name`, over the roster at
      `crates/nvs-types/src/routes.rs:56`. `commands::check_class_commands`
      (`crates/nvs-types/src/commands.rs:91`) is the worked shape for a per-class
      attribute pass, and `nvs_types::derive::ROUTE` (`crates/nvs-types/src/derive.rs:136`)
      is the name. ADR 0077 §§ 1-3.
- [ ] **§ 2's path grammar and § 3's `{param}`-to-parameter check, in that same pass** —
      the two of ADR 0077's four compile errors that are answerable from one method and its
      attribute. The duplicate-route and duplicate-`name` errors need the whole table and
      come with it. ADR 0077 §§ 2-3, `nvs_types::routes`' gap 1 and gap 2.

## Backlog

- `Core\Router::url` still laundering nothing: percent-encoding and the mount prefix —
  `nvs_stdlib::router`'s gap 2, ADR 0077 § 4.
- `urlAbsolute`'s configured origin, per mount falling back to `[app] origin` —
  ADR 0102 § 6.
- `Core\Router::match`/`methodsFor` and `Core\Router\Match` — out of scope by
  `docs/agent/loop-goal.md` § *Standing decisions*.
- `Core\Request` and the verb parse that turns an unrecognized method into a 501 rather
  than a case — `nvs_stdlib::router`'s gap 4.
- The spec's `array<string, mixed>` spellings name a type the grammar has no form for —
  `docs/spec/01-core-library.md:918` and `:1058`, same family as commit 23b5781's ADR pass.
- The 17 unattributed lowering refusals at `crates/nvs-ir/tests/refusals.rs:179`, which
  keep `verify.py` red for every session.
