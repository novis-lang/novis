# ADR 0104 — An application is an entry file path, and a per-app block is keyed on it

- **Status:** Accepted
- **Date:** 2026-08-27
- **Amended by:** 0097
- **Scope:** what an *application* is for configuration purposes, how a per-app block is spelled and
  matched, which of several matching blocks wins, and whether such a block may widen a limit or grant a
  capability. It does **not** decide the directive registry or the changeability classes, which stay
  [0005](0005-config-changeability.md)'s; nor the file's syntax ([0064](0064-configuration-file-format.md));
  nor how the configuration tree is assembled and trusted, which is
  [0103](0103-configuration-is-a-tree-of-files.md)'s.
- **Amends:** [0005](0005-config-changeability.md) — its § *Ceilings are their own directives* states that
  "what a per-app block is keyed on, and how one is spelled, is not stated anywhere yet"; § 1 below states
  it, and its sentence about a per-app block overriding "a ceiling downward" is corrected by § 3.
  [0097](0097-development-server-and-proxied-origin.md) § 10 — the gap it records is closed, and
  `[[server.mount]]` loses its `mode` key; and § 3's fallback, written as `[app] origin`, is this
  block's own `origin` key, since TOML cannot spell both (§ 1).
  [0064](0064-configuration-file-format.md) § 2a — the block list gains `[[app]]`.
  [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 5 — its mixed-application host
  now has the mechanism its third row assumed.

> **In short:** an application's identity is the **path of its entry file**, so `nvs run` on the command
> line has one exactly as a served request does. A per-app block is `[[app]]` carrying either a `root`
> path, which matches every entry file beneath it, or an `entry` path, which matches one file. The entry
> path is **canonicalized before matching**, so no `..` segment and no symlink inherits an application's
> rights. Every matching block applies, least-specific `root` first, so a host-wide block and an
> application-specific one compose instead of competing. A block may **widen as well as narrow** —
> setting any `[app.limits]` value up to the global `[limits.hard]` ceiling and granting a capability the
> global block withholds — which is what makes deny-by-default at the top and grants per application the
> natural layout. Only `[app.limits.hard]` is one-directional: an application's own ceiling may be lowered,
> never raised. `[[server.mount]]` loses `mode`, which it only ever carried because this block did not
> exist.

## Context

- **Three ADRs build on per-app blocks and none defines one.** [0005](0005-config-changeability.md) says a
  per-app block "may override a ceiling downward the same way it overrides a capability grant";
  [0064](0064-configuration-file-format.md) § *Revisiting* discusses their layout and calls it 0005's
  question; [0091 § 5](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) builds a
  mixed-application deployment on them. 0005 marks the gap in its own body, and
  [0097 § 10](0097-development-server-and-proxied-origin.md) marks it again from the other side, declining
  to fill it because doing so would put a second home on a fact 0005 nominally owns.
- **0097 also fixed the constraint that decides the shape.** It gave `[[server.mount]]` a `mode` key as a
  stopgap and said plainly why that is not the answer: "an application's identity should be its **entry
  file path**, not its mount. A mount-keyed block would leave `nvs run` on the CLI with no per-application
  identity at all, and would make per-app configuration unreachable until M7 even though 0005 lands in M6."
- **An application is a tree, not a file.** A served application resolves to one entry file
  ([0077](0077-compile-time-routing.md)), but the same deployment holds `bin/import.nvs`,
  `bin/migrate.nvs` and whatever else `nvs run` is pointed at, and all of them want the same limits and
  the same grants. Keying strictly on one file would mean one block per script and a new script silently
  inheriting the global defaults.
- **Direction is the question 0005 left ambiguous.** Its one sentence leans toward narrowing only, which
  reads as the safe answer and is not: if a block can only take rights away, the global block has to grant
  every right any application on the host needs, and deny-by-default inverts at exactly the layer that
  matters most.

## Decision

### 1. `[[app]]`, keyed on `root` or `entry`

```toml
[limits]
memory = "128M"                            # the global baseline

[capabilities]
process.exec = false                       # deny by default

[[app]]
root = "/srv/www"                          # every entry file beneath this directory
origin = "https://example.test"            # what `urlAbsolute` prepends (0097 § 3)
[app.limits]
memory = "256M"

[[app]]
root = "/srv/www/shop"
mode = "production"
[app.limits]
memory = "512M"
[app.capabilities]
process.exec = true                        # this application only

[[app]]
entry = "/srv/www/shop/bin/import.nvs"     # one file
[app.limits]
wall_time = "600s"
```

An entry carries `root` **or** `entry`, never both and never neither. `root` matches when it is a prefix
of the entry file's path **on path-component boundaries**, so `root = "/srv/www/shop"` matches
`/srv/www/shop/bin/import.nvs` and does not match `/srv/www/shopfront/index.nvs`. `entry` matches one
file exactly, and is simply the most specific form of the same test. A block's directives live in
`[app.limits]`, `[app.limits.hard]` and `[app.capabilities]` sub-tables, which attach to the preceding
`[[app]]` by ordinary TOML rules; `mode` and `origin` sit directly on the block.

**`origin` is one of the block's own keys, and there is no `[app]` table.**
[0097 § 3](0097-development-server-and-proxied-origin.md) makes `Core\Router::urlAbsolute`'s origin a
property of the mount, falling back to the application's; the application *is* this block, so that
fallback reads `origin` off every `[[app]]` matching the entry file, layered by § 2 like any other
directive. It cannot instead sit in a global `[app]` table beside these, because **TOML forbids one file
spelling `app` as both a table and an array of tables** — a file holding `[app]` and `[[app]]` fails to
parse outright, so a global origin would be unspellable in exactly the multi-application deployments
0097 § 3 wrote the fallback for. A host-wide default is a block with the widest `root`, which § 2 already
layers under the more specific ones. The key keeps the class 0097 § 3 gave it: `System` in
[0005](0005-config-changeability.md)'s model, `Reload` in
[0078 § 2](0078-config-reload-and-control-socket.md)'s orthogonal field.

Both keys are paths, so [0103 § 5](0103-configuration-is-a-tree-of-files.md) applies: a relative one
resolves against the directory of the file it is written in. `[[app]]` entries accumulate across the
configuration tree like every other array-of-tables
([0103 § 4](0103-configuration-is-a-tree-of-files.md)), so a per-host include may add an application
without restating the ones the base declared.

**The entry file path is canonicalized before it is matched** — symlinks resolved, `.` and `..` removed.
Without that, `/srv/www/shop/../other/x.nvs` matches `root = "/srv/www/shop"` and a symlink planted inside
an application's tree inherits that application's capabilities. This is the same
canonicalize-then-prefix rule [0006](0006-isolated-script-execution.md) already applies to
`script.spawn`'s roots, and for the same reason.

### 2. Every matching block applies, least-specific first

More than one `[[app]]` can match one entry file, and all of them do. They are applied in order of
**increasing specificity** — shortest `root` first, longest last, an `entry` match last of all — so the
example above gives `/srv/www/shop/bin/import.nvs` a memory of `512M` from the `/srv/www/shop` block and a
`wall_time` of `600s` from its own, while inheriting everything neither states.

This is not a third precedence rule: it is [0103 § 3](0103-configuration-is-a-tree-of-files.md)'s "later
wins", ordered by specificity instead of by file position, and every override is reported the same way.
Two blocks with the *same* `root` are a duplicate rather than a refinement, and are refused.

```console
$ nvs run /srv/www/shop/bin/import.nvs
info: app blocks: /srv/www, /srv/www/shop, /srv/www/shop/bin/import.nvs
info: limits.memory    = "512M"  (/srv/www/shop)
info: limits.wall_time = "600s"  (bin/import.nvs)
info: capabilities.process.exec = true  (/srv/www/shop)
```

An entry file matched by no block gets the global configuration, which is the ordinary case and needs no
block at all.

### 3. A block may widen, bounded by the global ceiling

`[app.limits]` sets any value for its application — wider or narrower than the global `[limits]` — up to
the global `[limits.hard]` ceiling, and `[app.capabilities]` may **grant** a capability the global
`[capabilities]` block withholds as well as drop one it holds. **This corrects
[0005](0005-config-changeability.md)'s "may override a ceiling downward"**, which read as narrowing-only
for both.

The reason is the layout it produces. Under narrowing-only, a host running one application that needs
`process.exec` must grant `process.exec` globally so that every *other* application's block can take it
away — a root file that is maximally permissive by construction, where forgetting a block is a grant.
Under this rule the root file denies, each application states what it needs, and forgetting a block denies.

The bound stays where [0005](0005-config-changeability.md) put it: **`[limits.hard]` is the host's answer
and an application cannot exceed it.** `[app.limits.hard]` may only *lower* an application's own ceiling —
that direction is real, and it is what lets an operator give one tenant a smaller worst case than the host
tolerates in general. A block raising its own ceiling is refused at boot, not clamped, exactly as
[0005](0005-config-changeability.md) refuses rather than clamps a `Core\Config::set`.

Everything an application sets for itself at runtime is unchanged and layers on top: `Core\Config::set`
writes the per-request copy-on-write overlay over the effective per-app value, and its ceiling is still
the global `[limits.hard]`.

### 4. `[[server.mount]]` loses `mode`

[0097 § 10](0097-development-server-and-proxied-origin.md) gave a mount a `mode` key for one reason —
[0091 § 5](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)'s mixed-application host
"promised per-application mode selection and had no configuration mechanism to point at" — and recorded
the missing block as "a doc bug this ADR records rather than fixes". § 1 fixes it, so the key goes: a
mount routes, an `[[app]]` block sets policy, and per-application mode now works identically for
`nvs serve` and `nvs run`.

```toml
[[server.mount]]
path = "/shop"
root = "/srv/www/shop"                     # routing

[[app]]
root = "/srv/www/shop"                     # policy
mode = "production"
```

The two blocks usually name the same directory, and that is the intended shape rather than a redundancy:
one says where requests arrive, the other says what the code that serves them may do. `[mode] ceiling`
([0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)) still bounds what any `[[app]]`
block may select, so nothing here widens what a mount could previously reach.

## Consequences

- **Cost: none on the request path.** Matching happens once per entry file, at the point the file is
  resolved — for a served request that is [0077](0077-compile-time-routing.md)'s compile-time route
  resolution, not a per-request walk. What it holds is one resolved config per distinct entry file,
  bounded by the number of applications on the host rather than by traffic
  ([0004](0004-memory-for-simplicity.md)).
- **Per-app configuration is reachable in M6, before any server exists**, which is the constraint
  [0097 § 10](0097-development-server-and-proxied-origin.md) asked to be carried forward. `nvs run`
  against a path under a `root` gets that application's limits and grants with no `nvs serve` involved.
- **A capability grant is now something a reader must look for in more than one place** — the global
  `[capabilities]` block and any `[[app]]` block whose root covers the code in question.
  `nvs config dump --origin` ([0103 § 9](0103-configuration-is-a-tree-of-files.md)) is what makes that
  answerable, and it is the same obligation the file tree already created.
- **`[app.limits]` sub-tables attaching to the preceding `[[app]]` is valid TOML that reads subtly.**
  A reader who mistakes `[app.limits]` for a global block will be wrong, and nothing about TOML flags it;
  the shipped example configuration is where that is taught.

## Alternatives rejected

- **Keying on the server mount.** Rejected in [0097 § 10](0097-development-server-and-proxied-origin.md)
  before this ADR existed: `nvs run` would have no per-application identity at all, and per-app
  configuration would be unreachable until M7 although [0005](0005-config-changeability.md) lands in M6.
- **An exact entry-file key only, with no prefix matching.** The simplest possible rule, and it makes a
  project with thirty `bin/*.nvs` scripts thirty blocks — where adding the thirty-first silently falls
  back to the global defaults, which is the failure direction that matters.
- **A quoted path as the table key** — `[app."/srv/www/shop/index.nvs".limits]`. Avoids the
  array-of-tables subtlety, at the cost of Windows paths needing literal-string quoting inside a table
  key, long unreadable lines, and no room for a selector that is not a path.
- **Narrowing only**, the reading [0005](0005-config-changeability.md)'s sentence invited. Argued in § 3:
  it forces the global block to grant every right any application needs, so that each block can remove
  some, and a missing block becomes a grant.
- **Widening limits but only narrowing capabilities.** A defensible split — resources are not rights — and
  it fails for the same reason: any application needing `process.exec` forces a global grant that every
  other application on the host then inherits unless its own block removes it.
- **Only the most specific matching block applies**, instead of layering all of them. Easier to explain,
  and it means a host-wide `[[app]]` block is silently discarded the moment a narrower one matches, so
  every narrow block has to restate everything the wide one said.
- **Deriving an application from the package or `require` graph** rather than from a path. It would
  identify an application more precisely than a directory does, and it needs the compiler to have run
  before configuration is known, which inverts the order — configuration is read at boot, and
  [0061](0061-compile-time-autoload-and-program-discovery.md) resolves programs after it.

## Verification

In M6, alongside [0005](0005-config-changeability.md)'s and
[0103](0103-configuration-is-a-tree-of-files.md)'s own lists:

- `nvs run` against a file under a `root` gets that block's limits, grants and `mode`; a file under no
  block gets the global configuration. The same file served through `nvs serve` resolves identically.
- Three blocks matching one entry file apply least-specific first, and `nvs config dump --origin` names
  which block set each effective value.
- `root = "/srv/www/shop"` does not match `/srv/www/shopfront/index.nvs`.
- An entry path reaching a `root` through `..` or through a symlink does not match it — the same case
  [0006](0006-isolated-script-execution.md) requires for `script.spawn`, asserted here for `[[app]]`.
- An `[[app]]` with both `root` and `entry`, with neither, or duplicating another block's `root`, refuses
  the boot.
- `[app.limits] memory` above the global `[limits]` default takes effect; above `[limits.hard]` refuses at
  boot, naming the ceiling. `[app.limits.hard]` below the global ceiling takes effect; above it refuses.
- `[app.capabilities] process.exec = true` under a global `process.exec = false` grants it to that
  application and to no other; the reverse narrows.
- A `[[app]]` block declared in an included file applies exactly as one in the root file does.
- `[[server.mount]]` carrying `mode` is refused as an unknown key (`E0601`), and the mixed-application
  host of [0091 § 5](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) is expressed with
  `[[app]]` blocks instead.
