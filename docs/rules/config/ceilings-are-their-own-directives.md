What the host is willing to lose to one request is stated separately from what a request starts
with, in a `System`-class block using the **same key names**:

```toml
[limits]                     # Runtime — what a request starts with
memory     = "128M"
cpu_time   = "5s"

[limits.hard]                # System — what one request may raise itself to
memory     = "2G"
cpu_time   = "60s"
```

The shipped ceilings are generous on purpose: they are sized to stop a runaway, not to shape ordinary
code. `[limits.hard] memory = false` removes the ceiling entirely, for a trusted single-tenant host —
and `false` is the only spelling of "no ceiling" anywhere in the tree.

The enforceable per-request cap is therefore the **ceiling**, not the default: a process's worst case
is in-flight requests times `[limits.hard] memory`, and sizing a deployment means sizing against that
number. That is memory spent to buy simplicity, since ordinary code never has to know a ceiling
exists, which is `rule:programs/memory-priority`'s ordering working as written. An operator who cannot afford it lowers one number in one root-owned
file.

`[mode]` is the second, and last, block with this shape — a `Runtime` `default` beside a `System`
`ceiling`, where the ceiling bounds how permissive a request may make itself
(`rule:config/a-program-may-read-and-flip-its-mode`). Two instances of the pattern is something a
reader learns once; a third would need its own argument.
