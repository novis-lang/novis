An upgrade takes `args`, `limits`, `grants` and `on` — the same four options a spawned script spells
after its operand, with identical meaning. They are ordinary named arguments here rather than a
`with(…)` clause because `upgrade` is a method, and a `with(…)` written inside an argument list would
read as a call to a function named `with`.

`grants` narrows and never widens: a request that gave up an authority cannot upgrade into a
connection that holds it, because the grant is asked against the upgrading request's own overlay.
`limits` is the connection's own budget — memory, CPU time, output and spawn depth — and it is the
whole of what makes a connection a root rather than a request stretched over hours
(`rule:concurrency/a-connection-is-a-root-isolate`).

Until the parameters exist, `limits:`, `grants:` and `on:` are **refused by name** rather than
accepted and dropped. A narrowing that is silently ignored hands the connection the authority its
request meant to give up, and a refusal is the only spelling that lets a reader tell a narrowing that
was applied from one that was not.
