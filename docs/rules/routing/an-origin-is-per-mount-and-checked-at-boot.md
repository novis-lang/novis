The origin `urlAbsolute` prepends (`rule:routing/an-absolute-link-takes-a-configured-origin`) is
declared **per mount**, on the `[[server.mount]]` block, falling back to `[app] origin`. A single global
origin is wrong for a host-mounted deployment the server already supports: one binary serving
`tenant1.example.com` and `tenant2.example.com` would put one tenant's origin in the other's email. The
same `{1}` that selects the mount builds its origin — `origin = "https://{1}.example.com"` — so onboarding
a tenant is zero new configuration, and the request reads the same captures back through
`rule:routing/a-request-reads-its-mount`.

`origin` is `System`-class: a request may not set it, because a value a request can choose is a value an
attacker can influence, and this one ends up in mail. It is `Reload`-able, so a new tenant needs no
restart. **The check that an origin resolves runs at mount expansion**, per resolved mount, and re-runs
on reload: a mount whose unit contains a literal `urlAbsolute` call and resolves no origin is a boot
error, so the failure is at deploy time rather than in a sent message.

**The check is asked at the boot compile**, which is the first point at which both halves are in hand:
`nvs serve` compiles every mounted entry before it binds anything, and asks of each resolved row whether
the unit it just compiled builds an absolute link. One entry serves every tenant a `scan` glob
enumerated, so the same unit is a refusal under a row that resolved no origin and a start under the row
beside it. A served request then receives its mount's origin on the isolate that answers it, because a
process serving many mounts has no one origin its accept loop could hold. A mount and its origin cannot
change under a running process — `[server]` is `Boot`-class as a whole
(`rule:http-server/the-server-block-is-boot-class`) — so the boot is the only place the question arises.
