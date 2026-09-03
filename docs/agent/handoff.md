# Handoff

## State

**M8 goal 5, stage 9. The `-p nvs-types` acceptance list is whole** — its last unwritten test,
`a_tainted_settings_host_is_a_diagnostic_naming_assert_trusted`, exists and passes beside the two
ADR 0024 § 4 cases it belongs with.

**ADR 0135 § 3's qualifier classification now reaches the checker.** `CoreShapeField::qual` carries
it, `nvs_types::core_lib`'s two lowerings and `TypeInterner::options` fill it from the registry's own
`CoreTy`, and `nvs_types::expr::args::check_shape_field` reads it: a `Qual::Sink` key refused a
`tainted` value gets ADR 0067 § 3's way through named in the diagnostic's `help:`, where the bare
`expected string, found tainted string` left a reader nowhere to go. That function's doc comment is
the home of why the help is attached at a shape *key* and not at an ordinary sink parameter — the
query-text sink's answer is a bound parameter and the HTML sink's is `Core\Html::escape`, so naming
the escape hatch at either would push the wrong fix.

**The help names `Core\Taint::assertTrusted`, which no registry row declares.** The spelling is
ADR 0067 § 3's, so the diagnostic is right and the tree is one member short of it. That turns the
next group's first item from a decision at leisure into a real gap: a program told what to write
cannot write it yet.

`E0618`'s host-grant check is untouched, and `crates/nvs-types/src/intrinsics.rs`' known gaps 5-7
still stand as written — nothing this session did narrows any of them.

## Next group

**One file set: `crates/nvs-stdlib/src/registry.rs` with the class module beside it, widening to
`crates/nvs-types/src/core_lib.rs` on the third.**

- [ ] **Decide whether `Core\Taint::assertTrusted` earns a registry row, and record the decision.**
      ADR 0067 § 3 names it as the only way through a sink with no launderer and a diagnostic now
      names it too; ADR 0024 § 3 owns the axis. The mark is the real question:
      `crates/nvs-stdlib/src/registry.rs:152` is `Qual`, whose `Launder` doc says a launderer names
      *the sink it launders for*, and this member launders for every sink — the `tainted` twin of
      `Qual::Reveal`. `crates/nvs-stdlib/src/secret.rs:41` is `Core\Secret`, the same escape hatch
      one axis over and the model to copy, `$reason` parameter included.
- [ ] **If it earns one, write the five edits** — the row, the card, the body, the `address()` arm
      and three `.nvst` cases, per `docs/agent/conventions.md`.
      `crates/nvs-stdlib/src/registry.rs:1178` is `CLASSES`, which the new class's module joins.
- [ ] **Then hold the roster the way the `secret` axis is held.**
      `crates/nvs-types/src/core_lib.rs:994` is
      `reveal_and_the_password_helpers_are_the_only_launderers_of_secret`; the `tainted` axis has no
      such test, so nothing today would notice a second member claiming to launder every sink.

## Backlog

- `nvs check` reads no `nvs.toml`, so `E0618` is exercised and fires for nobody —
  `crates/nvs-types/src/intrinsics.rs` gap 6.
- A `db.open` grant is matched host-for-host, so § 3's `"*.tenants.internal"` matches nothing —
  same module doc, gap 7, and the rule is `nvs_config::capability`'s.
- § 10's unterminated string literal is refused by neither pass, and the two module docs disagree
  about which is right — `intrinsics.rs` gap 5.
- Stage 9's remaining acceptance legs are `-p nvs-db`'s span test and `examples/db.nvs` —
  `docs/agent/loop-goal.toml`.
