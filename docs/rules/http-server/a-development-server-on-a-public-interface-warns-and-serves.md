When the server binds a non-loopback address while the mode is `development`, it emits an unmissable startup banner **and** a `Warn` record naming the bound address (`rule:errors/log-level`), then serves. Bound to loopback it emits neither.

The banner alone is not enough — it scrolls past in a container log — which is why it is *also* a `Warn` record: it lands in the same JSON-Lines stream everything else does, queryable after the fact rather than only observable at the moment of startup.

This is the one place where letting the mode flag win over the file (`rule:config/the-mode-flag-wins-over-the-file`) is paid for, and the payment is deliberately visible: the banner is what stands between a stale `--mode=development` in a deploy script and a public debug surface.

**Declined:**
- **Refusing the bind behind an `allow_public_development` unlock.** It would turn a development server exposed to the internet into a startup error for the price of one directive, but binding `0.0.0.0` inside a container or a VM is the normal case, not the exceptional one, and a refusal would put a required directive in front of every containerised development workflow.
