# Handoff

## State

**Goal 3, Stage 2 is part-landed and the driver is measuring again.** `crates/nvs-config` holds the
directive registry (`directive.rs`) and one file's parse with ADR 0064 § 3's duplicate refusal
(`file.rs`), seven tests green. `file.rs:38`'s `parse` is already generic over the tree type with
`toml::Table` as the stand-in, so the typed tree lands under it without touching anything above.

**`[app]` now has one spelling across the tree, and the contradiction the last handoff recorded is
closed.** ADR 0104 § 1 owns `origin` as a key sitting directly on an `[[app]]` block beside `mode`, with
the TOML argument in its own paragraph — one file cannot spell `app` as both a table and an array of
tables, so a global `[app] origin` is unspellable wherever a per-app block exists. ADR 0097 § 3's
fallback names that key, 0104's `Amends:` carries the clause, `nvs.toml` is `[[app]] root = "."`, and
`configured_origin` (`crates/nvs-cli/src/main.rs:551`) reads the array-of-tables header while every
ordinary header — `[app.limits]` included — ends the block. `examples/routes.nvs` still prints
`absolute=https://example.test/users/7`, and the four `tests/conformance/core/router-url-absolute-*`
cases moved with it: they write their own `nvs.toml`, and one pins the diagnostic naming the key. Two
doc comments followed (`nvs-runtime/src/ctx.rs`, `nvs-stdlib/src/router.rs`'s `urlAbsolute` card).
`verify.py` is 7 of 7 green — conformance 1000, differential 206.

**Two anchors in the last handoff were wrong, and cost a grep each**: the origin fallback is ADR 0097
**§ 3**, not § 6 (§ 6 is the proxy-trust section; the `§ 6` in the code's comments is *0102*'s), and
0064 § 2a needed nothing, as recorded.

The acceptance check still fails on `examples/config.nvs` — `Core\Config` has no `get`. That is Stage 3's
snapshot and Stage 4's members, unwritten, not a regression. Nothing is blocked on the user; goals 1 and
2 and M4 remain the floor.

## Next group

**The typed block tree, then the tree that resolves.** One file set: `crates/nvs-config/src/`,
`Cargo.toml`, `docs/adr/0103`.

- [ ] **The typed block tree, and an unknown key refused naming its block.** ADR 0064 §§ 2a
      (`docs/adr/0064-configuration-file-format.md:129`), 3 (`:150`). Each block's fields come from its
      owning ADR rather than being invented, which is the slice's whole cost — **these are the anchors,
      already resolved, so do not re-derive them**: `[[include]]` 0103:115; `[[app]]` 0104:68 (`root`,
      `entry`, `mode`, `origin`, and the `[app.limits]`/`[app.limits.hard]`/`[app.capabilities]`
      sub-tables); `[limits]`/`[limits.hard]` 0005:68,75; `[mode]` 0005:109 and 0091:103;
      `[capabilities]` 0006:183, with more rows at 0018:103 and 0067:114; `[[extension]]` 0003:118;
      `[debug]` 0018:100; `[log]` 0020:134,168 (`handler`, `handler_reserve_memory`,
      `handler_reserve_time`, `target`) plus `format`/`level` at 0092:114,187; `[http.errors]` 0020:227
      (`detail`); `[db.<name>]` 0103:175; `[deferred]` 0072:213; `[[schedule]]` 0073:62;
      `[http.headers]` 0074:72, `[http.cors]` :106, `[http.cookies]` :129, `[http.client]` :171;
      `[metrics]` 0076:190, `[trace]` :196; `[server]` 0097:212, `[[server.mount]]` 0097:131 (no `mode`
      — 0104 § 4 took it). Needs `derive` on the workspace `serde` (`Cargo.toml:199`, featureless
      today). The unknown-key refusal is a new `E06xx` (next free E0605) naming the block.
- [ ] **The tree resolves.** ADR 0103 §§ 1–5: a root named by repeatable `--config`, else the search
      order; includes depth-first in list order; later wins with both origins recorded.
- [ ] **Ownership is the trust boundary.** ADR 0103 § 6 — owner-or-root, not group- or world-writable,
      and an absent `optional` include puts the check on the directory that would hold it.

## Backlog

- `examples/config.nvs` needs `Core\Config::get`/`set` over Stage 3's snapshot — the standing acceptance
  failure; `docs/plan/m6.md` § *Verify* is the list.
- `examples/capability.nvs` needs an `[[app]]` grant block in `nvs.toml` — an `fs.read` root, no more.
- `configured_origin` retires the moment M6's reader resolves a tree; its doc comment says so.
- Item 18's `Core\Secret::reveal()` is not in the registry (`crates/nvs-stdlib/src/`).
- Item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`).
- `orient.py` warns that `[context] modules` names `crates/nvs-host/src/budget.rs`, which matches no
  module — the glob in `docs/agent/loop-goal.toml` is stale.
