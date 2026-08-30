# Handoff

## State

**ADR 0064 § 5's `Core\Config` is on disk end to end, and the driver's `examples/config.nvs` check
passes** — `memory=256M / set ok / set refused / still=512M`, off this repository's own `nvs.toml`.
The chain is four hops and each owns its own rule: `nvs run` resolves the tree in `boot_snapshot`
(`crates/nvs-cli/src/main.rs:610`), `Snapshot::build` folds the matching `[[app]]` blocks over it,
`Ctx::set_config` (`crates/nvs-runtime/src/ctx.rs`) gives the request the one `Arc` clone ADR 0078
§ 1 allows it, and `nvs_config::request::Request` holds that snapshot plus the copy-on-write
overlay. `crates/nvs-stdlib/src/config.rs`'s four members are marshalling and nothing else — every
rule about names, classes and ceilings lives in `request.rs`'s module doc.

**One decision is recorded in code and not in an ADR, deliberately.** `nvs run` reads the tree
through `LocalFiles` (`crates/nvs-cli/src/main.rs:560`), which does **not** apply ADR 0103 § 6's
ownership check; `Disk` still does, and `nvs serve`/`nvs ctl reload` will use it. That type's own
doc comment carries the three-part argument, the shortest of which is that § 6 itself names the
Windows default this repository sits on. Nothing is granted out of the unchecked tree today —
`[capabilities]` is enforced nowhere — so Stage 4's reserved ADR slot is where it is settled rather
than here.

`configured_origin`'s hand-rolled one-key `nvs.toml` scanner is gone: ADR 0102 § 6's origin now
comes off the snapshot, so it is the origin of the blocks that actually match the entry file.

`orient.py` did not print ADR 0064 § 5, which is this group's own specification — `[context] adrs`
wants `0064:5`, and `0103:1` for the roots the next group needs.

## Next group

**`nvs config` and `--config` — ADR 0103 §§ 1 and 9, the last of Stage 2 still open.** One file
set: `crates/nvs-cli/src/main.rs` (the `Command` enum at `:107`, `LocalFiles` at `:560`,
`boot_snapshot` at `:610`), a new `crates/nvs-cli/src/config.rs` for the subcommand's own body, and
`crates/nvs-config/src/resolve.rs:246` (`roots`) with `crates/nvs-config/src/snapshot.rs:83`
(`origins`) behind it. Everything the first two need is already there; neither adds a crate.

- [ ] **`nvs config check <file>...`** — ADR 0103 § 9. `loop-goal.toml:1725`'s `command` check is
      the acceptance: `nvs config check tests/config/duplicate-key.toml` exits non-zero and prints
      both `duplicate` and `duplicate-key.toml:`. The fixture exists; the refusal already carries
      its span, so this is `boot_snapshot`'s resolve half plus `render_diagnostics`.
- [ ] **`nvs config dump [--origin]`** — the same section's other half: every key in force, and
      with `--origin` the file each was written in. `Snapshot::origins` is that map by dotted key
      and `Snapshot::overrides` is § 3's record, both already populated.
- [ ] **Repeatable `--config <path>`** — § 1 step 1, which `roots` already takes a flag list for
      and which `boot_snapshot` passes `&[]` to. It belongs on `run`, `check` and `dump` alike, and
      an explicit one disables step 2 entirely.

## Backlog

- Stage 4 item 10's ADR slot: where a capability check sits — and with it whether a CLI run may be
  granted anything out of a tree read without § 6 (`crates/nvs-cli/src/main.rs`'s `LocalFiles`).
- `nvs-host` still runs on compiled-in defaults, so a limit a request moved is visible to
  `Core\Config::get` and not yet to what enforces it (`crates/nvs-config/src/lib.rs`'s module doc).
- A `RuntimeTighten` directive that is not a quantity — `[capabilities]` — cannot be set at all;
  `crates/nvs-config/src/request.rs`'s module doc owns why refusing is the safe direction.
- `nvs test <program>` and `crates/nvs-cli/src/script.rs` build a `Ctx` with no configuration, so a
  `#[Test]` method and a spawned script both read an empty one.
- Item 18's `Core\Secret::reveal()` is still not in the registry, though `Qual::Reveal` is decided.
- `Live::admit`'s same-class check is asked of the answer and not of the argument
  (`crates/nvs-runtime/src/graph.rs` § *Known gaps*).
