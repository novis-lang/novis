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

A ceiling is sized to stop a runaway, not to shape ordinary code: a script that raises its own
`memory` for an import is doing something the operator already allowed, and one that asks for more
than the ceiling gets `false`. A `[limits.hard]` key the file does not write is the `[limits]` value
the run started with, written or default, so a request may raise a limit only once its operator
writes a ceiling above the start (`rule:config/no-configuration-file-is-a-complete-configuration`).
`[limits.hard] memory = false` removes a ceiling explicitly, for a trusted single-tenant host — and
`false` is the only spelling of "no ceiling" anywhere in the tree.

The enforceable per-request cap is therefore the **ceiling**, not the default, and sizing a
deployment means sizing against that number. That is memory spent to buy simplicity, since ordinary
code never has to know a ceiling exists, which is `rule:programs/memory-priority`'s ordering working
as written. An operator who cannot afford it lowers one number in one root-owned file.

- **Memory:** a process's worst case is its in-flight requests times `[limits.hard] memory`, not
  times the `[limits] memory` a request starts with.

`[mode]` is the second, and last, block with this shape — a `Runtime` `default` beside a `System`
`ceiling`, where the ceiling bounds how permissive a request may make itself
(`rule:config/a-program-may-read-and-flip-its-mode`). Two instances of the pattern is something a
reader learns once; a third would need its own argument.

**Declined:**
- **No ceiling directives at all**, so that a request may raise any limit without bound. A process
  serves many requests at once, so one script that removes its own memory limit would take the host
  from every request beside it. The ceiling is what makes the freedom to widen affordable.
