# Handoff

## State

**M8 goal 5, stage 9. ADR 0067 § 10's host check is live and its `-p nvs-types` test passes.**
`E0618` refuses a **literal** `Core\Db::open` host that the compiling machine's `db.open` grant
does not cover; `crates/nvs-types/src/intrinsics.rs`' module doc owns the pass and `E0618`'s own
doc comment in `nvs-diagnostics` owns why it is a capability code and not a type one. Three
things had to meet: `Intrinsic::field` now addresses a key *inside* the written argument, so § 18's
`Db\Settings` shape is reachable from § 1's table; `Env::grants` carries the deployment's
`[capabilities]` block, filled by `nvs_types::check_program_granted`; and `Grammar::Host` walks the
grant through `nvs_config::capability::Capabilities::allows_host`, the same list walk
`nvs_runtime::capability::require` uses, so a check cannot disagree with the run it precedes.

**`None` grants say nothing rather than deny**, and that is the load-bearing half —
`check_program_granted`'s doc is its home. Every other fixture in the tree checks with `None`, and
so does every `nvs check` today, because `nvs-cli`'s check path reads no `nvs.toml`. The refusal is
therefore exercised and fires for nobody yet; `intrinsics.rs` gap 6 is that gap and nothing else.

**Stage 9's acceptance check still fails, on its last unwritten test.**
`a_tainted_settings_host_is_a_diagnostic_naming_assert_trusted` does not exist, and the playbook
bullet added this session corrects what gap 6 used to claim about it: the refusal already fires as
`E0401` at the `host:` field, and only the *naming* of ADR 0067 § 3's way through is missing.

## Next group

**One file set: `crates/nvs-types/src/expr/args.rs` with `crates/nvs-types/tests/intrinsics.rs`,
widening to `crates/nvs-stdlib/src/registry.rs` on the third.**

- [ ] **Name ADR 0067 § 3's only way through in the tainted-host refusal.** § 3 makes
      `Settings.host` a sink with no launderer, so the bare "expected `string`, found
      `tainted string`" leaves a reader with nowhere to go. Attach the help where a `Qual::Sink`
      text parameter is refused a `tainted` argument. `crates/nvs-types/src/expr/args.rs:729` is
      `select_arm`, `crates/nvs-types/src/core_lib.rs:253` is where a `CoreTy::Shape` parameter's
      arms are lowered.
- [ ] **Write `a_tainted_settings_host_is_a_diagnostic_naming_assert_trusted`**, beside its two
      ADR 0024 § 4 siblings. `crates/nvs-types/tests/intrinsics.rs:422` is
      `a_tainted_value_at_a_query_text_parameter_is_a_diagnostic`, and
      `crates/nvs-types/tests/intrinsics.rs:628` is `open`, the `Core\Db::open` fixture builder
      this session added — pass it a `tainted` host rather than writing a third program shape.
- [ ] **Decide whether `Core\Taint::assertTrusted` earns a registry row**, since after the first
      slice a diagnostic names a member no class declares. `crates/nvs-stdlib/src/registry.rs:1176`
      is `CLASSES`; ADR 0024 § 4 and ADR 0067 § 3 are the two that ask for it.

## Backlog

- Wire `nvs check` to a resolved `[capabilities]` block — `crates/nvs-cli/src/main.rs:637` calls
  `check_program`, `crates/nvs-cli/src/config.rs:142` is `load`. A command that reads configuration
  is a command a broken `nvs.toml` can fail, which is the decision; `intrinsics.rs` gap 6 owns it.
- That wiring flips an already-green case:
  `tests/conformance/core/db-open-asks-the-grant-about-the-host-and-then-the-address.nvst` expects a
  **runtime** refusal for a literal ungranted host, which becomes `E0618` at compile time.
- No `.nvst` case pins `E0618` yet, and none can until the wiring above lands — the runner compiles
  through `nvs`, which passes no grants.
- A `db.open` grant is matched host-for-host, so ADR 0067 § 3's own `"*.tenants.internal"` example
  matches nothing. `nvs_config::capability`'s rule, and `intrinsics.rs` gap 7 records it.
- `docs/adr/0067-core-db.md` § 10 is not in the goal's `[context] adrs`; this session worked from
  the gap list and the `[[check]]` block instead. Add `0067 §10` to the manifest.
