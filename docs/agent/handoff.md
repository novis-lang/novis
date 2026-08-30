# Handoff

## State

**Goal 3, Stage 2 is part-landed and the driver is measuring again.** `crates/nvs-config` holds the
directive registry (`directive.rs`) and one file's parse with ADR 0064 § 3's duplicate refusal
(`file.rs`), seven tests green.

**The acceptance check had been aborting before any check ran.** Three of `loop-goal.toml`'s `files`
were missing, and `tools/loop.py:1779` returns on the first one — see the playbook bullet. All three
now exist: `examples/{config,limits,capability}.nvs`, plus `examples/capability/greeting.txt`, and
`nvs.toml` carries the `[limits] memory = "256M"` / `[limits.hard] memory = "512M"` pair that
`config.nvs`'s frozen output is derived from. All three call APIs Stages 3–4 have not written, so they
fail their own checks — that is the ordinary open-item state, and what changed is that every Stage 0–2
check now runs at all.

**Item 1 of the old group is blocked on a three-ADR contradiction, and resolving it is now slice 1.**
The typed block tree cannot be written until `[app]` has one spelling: ADR 0064 § 2a's table
(`docs/adr/0064-configuration-file-format.md:129`) lists only `[[app]]`, owned by ADR 0104, which gives
it no `origin` key; ADR 0097 § 6 (`docs/adr/0097-development-server-and-proxied-origin.md:169`) makes a
mount's `origin` fall back to `[app] origin`; and this repository's own `nvs.toml:18` writes `[app]
origin`, read by `configured_origin` (`crates/nvs-cli/src/main.rs:551`), with `examples/routes.nvs`'s
frozen output depending on it. **TOML forbids one file defining `app` as both a table and an array of
tables**, so the tree must pick one. Recommended: `origin` becomes an `[[app]]` field — 0104's unit is
exactly "an application" and 0097 § 6 wants a per-application fallback — with 0097 § 6 amended to name
it, `nvs.toml` migrated to `[[app]] root = "."`, and `configured_origin` following.

Nothing is blocked on the user. Goals 1 and 2 and M4 remain the floor.

## Next group

**`[app]` gets one spelling, then the typed tree over it.** One file set: `docs/adr/{0064,0097,0104}`,
`nvs.toml`, `crates/nvs-cli/src/main.rs`, `crates/nvs-config/src/`.

- [ ] **`[app]` resolves to `[[app]]`, carrying `origin`.** ADR 0104 gains the key and the `Amends:`
      pair with ADR 0097 § 6 (`docs/adr/0097-development-server-and-proxied-origin.md:169`); 0064 § 2a's
      table (`docs/adr/0064-configuration-file-format.md:129`) already says `[[app]]` and needs nothing.
      Then `nvs.toml:18` and `configured_origin` (`crates/nvs-cli/src/main.rs:551`), whose naive
      line-scanner tracks `[table]` headers and must learn `[[app]]`. `examples/routes.nvs`'s frozen
      output must not move.
- [ ] **The typed block tree, and an unknown key refused naming its block.** ADR 0064 §§ 2a, 3 — § 2a's
      table names the owning ADR of all sixteen blocks and each block's fields come from that ADR rather
      than being invented, which is this slice's whole cost. `crates/nvs-config/src/file.rs:38`'s
      `parse` is already generic over the tree type with `toml::Table` as the stand-in, so nothing above
      it changes. Needs `derive` on the workspace `serde` (`Cargo.toml:199`, currently featureless).
- [ ] **The tree resolves.** ADR 0103 §§ 1–5: a root named by repeatable `--config`, else the search
      order; includes depth-first in list order; later wins with both origins recorded.
- [ ] **Ownership is the trust boundary.** ADR 0103 § 6 — owner-or-root, not group- or world-writable,
      and an absent `optional` include puts the check on the directory that would hold it.

## Backlog

- `loop-goal.toml`'s `[context] modules` names `crates/nvs-host/src/budget.rs`, which matches nothing;
  `orient.py` warns every session. Fix or drop the selector.
- `examples/capability.nvs` catches `Throwable`; narrow it once Stage 4's ADR slot decides what class a
  capability denial throws.
- `examples/capability.nvs` needs an `[[app]]` grant block in `nvs.toml` — an `fs.read` root, no
  `fs.write`, no `script.spawn` — which is Stage 4's (ADR 0104).
- `Core\File::read`/`::write` are goal 4's and are what `capability.nvs` calls; it cannot compile first.
- The three known gaps, each recorded where its code is: `Core\Secret::reveal()`, `Live::admit`'s
  same-class check, `Core\Script`'s members.
