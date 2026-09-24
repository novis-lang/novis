---
milestone: post-parity
---

# Side goal — an unwritten `dispatch` and `static` follow the mode a configuration names

When this goal is green, a configuration that writes `[mode] default = "development"` and leaves
`[server] dispatch` and `[server] static` unwritten serves as `dispatch = "path"` and
`static = true`. A production configuration, and one that names no mode, serves as
`dispatch = "entry"` and `static = false`. A written value always wins. The two are chosen when a
configuration is published, at boot and at every reload, and a runtime mode flip never changes them.
That is `rule:config/a-startup-default-is-never-flipped` as written, whose `settle` row is on disk
and whose `dispatch` and `static` rows are not.

## Why a side goal

It needs nothing the chain has not built, and nothing on the chain waits for it. Its files are the
mount table in `nvs-server`, the core-local table in `nvs-cli`'s serve loop, one helper in
`nvs-config`, the shipped template and the server chapter. The chain's live goals are the generated
dossier goals over `Core` members, which open none of them.

## What is on disk today, measured

- `crates/nvs-server/src/mount.rs:176` `Table::from_config` reads the two switches as written and
  falls back to `Dispatch::Entry` and `false` in every mode. Its module doc § *Decision: an unwritten
  switch is the closed one, not development's* (`mount.rs:35`) says so and names this as the mode
  slice's work.
- `crates/nvs-config/src/cache.rs:517` `Revalidation::from_config` is the row that is on disk: it
  reads `config.mode.default` each time a configuration is published and picks `settle` from it.
  That is the shape to copy.
- `crates/nvs-cli/src/serve/mounts.rs:125` `switches` is what a core compares to decide whether its
  table must be rebuilt after a reload. It reads only `dispatch`, `static` and `health_path`, so a
  reload that changes only the mode would keep the old table.
- `crates/nvs-config/src/default.toml:449` and `:451` say `# default` over the production value,
  which is true today. `docs/reference/tools/25-server.md`'s `[server]` block says "unset is
  "entry" in both modes" and "unset is false in both modes", which is true today.

## Stage 0 — the catch-up

The sentences on disk this goal makes wrong. Each is rewritten whole by the session that lands the
behaviour, and not before:

- `docs/rules/config/a-startup-default-is-never-flipped.md` — its *What is on disk* paragraph.
- `crates/nvs-server/src/mount.rs` — the module doc's § *Decision* and `Table::from_config`'s doc
  comment.
- `crates/nvs-config/src/default.toml` — the `dispatch` and `static` lines, and the `[mode]`
  `default` line's comment, which says what development turns on.
- `docs/reference/tools/25-server.md` — the two comments in the `[server]` block, and the sentence
  under the dispatch steps that begins "With `dispatch = "path"` and `static = true`".

`docs/novis.md` is generated and is regenerated with `python tools/reference.py --no-examples`,
never edited.

## Stage 1 — the floor

Main's carried floor, which a side run is always checked against (`tools/side.py`). Never traded.

## Stage 2 — the switches follow the mode (the keystone)

One file set: `crates/nvs-config/src/server.rs`, `crates/nvs-server/src/mount.rs`,
`crates/nvs-cli/src/serve/mounts.rs`.

- **One function resolves both switches.** It lives in `nvs-config` beside the other
  `[server]` readers, takes a `&Config`, and returns the written value where one is written and
  the mode's row otherwise. It reads the mode exactly as `Revalidation::from_config` does
  (`cache.rs:517`). No second copy of the two rows exists anywhere.
- **`Table::from_config` calls it** (`mount.rs:176`), and the module doc's § *Decision* is
  rewritten whole to say what the table does now.
- **A reload that changes only the mode rebuilds the table.** `switches` (`mounts.rs:125`)
  compares what the new function returns, not the raw keys.
- **The tests**, in `mount.rs`'s own test module: the development mode with both switches
  unwritten gives `Dispatch::Path` and static on; a written `dispatch = "entry"` or
  `static = false` under development wins; production and a configuration with no `[mode]` give
  `Dispatch::Entry` and static off. In `mounts.rs`: a snapshot that differs only in its mode
  gives a different table.

## Stage 3 — the template and the reference say it, and the proofs exist

One file set: `crates/nvs-config/src/default.toml` and its guard in `tools/directives.py`,
`docs/reference/tools/25-server.md`, `docs/rules/config/a-startup-default-is-never-flipped.md`.

- **The template's two lines** end `# default: "entry"; "path" in development` and
  `# default: false; true in development`, and `python tools/directives.py --check-template`
  stays green.
- **The `[mode] default` line's comment** says that development also turns on path dispatch and
  static files when those two are unwritten.
- **The server chapter's `[server]` block** says the same in its two comments, and the sentence
  under the dispatch steps names the development mode again.
- **The rule's *What is on disk* paragraph** names the function and drops the sentence saying
  the two rows are not derived. `python tools/rules.py --render` regenerates the chapter.
- **The proofs.** `python tools/dossier.py --id` for each feature this touches — the server
  chapter's sections and the two configuration keys — names what is still owed, and the goal
  pays it.

## Standing decisions

These are the user's calls, made on 2026-09-24, or follow from the rule the user already decided.
No session re-decides one.

- **The rule stands as written.** Development gives `"path"` and `true`, production gives `"entry"`
  and `false`, a written value wins, and a runtime mode flip never changes either
  (`rule:config/a-startup-default-is-never-flipped`). An unwritten `[mode] default` is production.
- **The choice is made when a configuration is published**, at boot and at every reload, in the same
  place and the same way `settle` is chosen.
- **An `[[app]]` block is not consulted**, for the reason `Revalidation::from_config` gives: the two
  switches are `[server]` keys and one server holds one mount table.
- **No new decision record.** The rule is already decided; this goal builds it. The mount module's
  § *Decision* is rewritten whole, not kept beside the new text.
- **The tradeoffs, stated once.** Performance: none on the request path; the value is read at
  publish. Memory: none. Usability: a development server behaves like `try_files $uri /index.nvs`
  without writing two keys. Security: a host that writes `mode = "development"` runs any `.nvs` file
  under its mount root. The rule accepted that, and production, and a host with no configuration,
  never do. Simplicity: two fewer keys to write in development.
- **Every comment in a new `.nvs` and every new or changed `about.md` follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `python tools/dossier.py --comments <paths>`
  is run over them before the wrap.
