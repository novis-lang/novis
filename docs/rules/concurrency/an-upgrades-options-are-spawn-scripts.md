An upgrade takes `args`, `limits`, `grants` and `on` — the same four options a spawned script spells
after its operand, with identical meaning and identical spelling (`limits: {memory: "8M"}`). They are
ordinary named arguments here rather than a `with(…)` clause because `upgrade` is a method, and a
`with(…)` written inside an argument list would read as a call to a function named `with`. Both
`Core\Socket::upgrade` and `Core\Sse::upgrade` take them, and the type checker holds each to the
check `spawn script` makes: a `limits` key that is not a sub-cap does not compile.

`grants` narrows and never widens: a request that gave up an authority cannot upgrade into a
connection that holds it, because the grant is asked against the upgrading request's own context
when the upgrade is prepared. The connection's own context is the server's, so asked there, a
capability the request gave up would still be held. A written name the request does not hold, or no
capability spells, is dropped; an empty list is a connection that may ask for nothing; and no
`grants` at all carries the request's own narrowing, so an upgrade never holds more than the
request that made it.

`limits` is the connection's own budget — memory, CPU time, output and spawn depth — and it is the
whole of what makes a connection a root rather than a request stretched over hours
(`rule:concurrency/a-connection-is-a-root-isolate`). It is applied to the connection's context as a
spawned child's is: a value wider than the ceiling in force leaves that ceiling standing.

`on` takes `"here"` and nothing else. A connection's socket or event stream belongs to the core that
accepted it, so the connection runs there, and `on: "worker"` does not compile where it is written.
It is refused rather than accepted and dropped, because a placement that is silently ignored is one
a reader believes was applied.

The narrowing travels on the prepared upgrade (`nvs_runtime::Upgrade::into_parts` hands it out with
the program and the argument), and every door that starts a connection — the server's two and
`nvs run --request`'s two — hands it to the isolate it starts.
