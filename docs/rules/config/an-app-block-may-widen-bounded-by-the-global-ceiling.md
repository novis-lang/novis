`[app.limits]` sets any value for its application — wider or narrower than the global `[limits]` — up
to the global `[limits.hard]` ceiling, and `[app.capabilities]` may **grant** a capability the global
`[capabilities]` block withholds as well as drop one it holds.

The reason is the layout it produces. Under narrowing-only, a host running one application that needs
`process.exec` must grant it globally so that every *other* application's block can take it away — a
root file where forgetting a block is a grant. Under this rule the root file denies, each application
states what it needs, and forgetting a block denies.

The bound stays where the changeability model put it: **`[limits.hard]` is the host's answer and an
application cannot exceed it.** `[app.limits.hard]` may only *lower* an application's own ceiling —
what lets an operator give one tenant a smaller worst case than the host tolerates in general. A block
raising its own ceiling, or asking for a value above the global one, is refused at boot (`E0610`)
naming the ceiling, not clamped, exactly as a `Core\Config::set` above the ceiling is refused rather
than clamped. A block is bounded only by a ceiling the host actually wrote.

Everything an application sets for itself at runtime is unchanged and layers on top:
`rule:config/ini-set-is-core-config-set` writes the per-request overlay over the effective per-app
value, and its ceiling is still `[limits.hard]`. Where no ceiling is written for a key, the ceiling
is the per-app value the request started with
(`rule:config/no-configuration-file-is-a-complete-configuration`).

A capability grant is therefore something a reader looks for in more than one place: the global
`[capabilities]` block and every `[[app]]` block whose `root` covers the code in question. `nvs config
dump --origin` names which block set each effective value
(`rule:config/check-and-dump-audit-the-tree-offline`), which is what makes that answerable.

**Declined:**
- **Narrowing only.** It forces the global block to grant every right any application needs so that
  each block can remove some, and a missing block becomes a grant.
- **Widening limits but only narrowing capabilities.** Resources are not rights, so the split is
  defensible, and it fails for the same reason: one application that needs `process.exec` forces a
  global grant that every other application on the host inherits unless its own block removes it.
