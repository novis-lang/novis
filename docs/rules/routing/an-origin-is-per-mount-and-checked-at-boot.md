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

**Not shipped.** The configuration reader parses a mount's `origin` and substitutes its captures, but a
served request never receives it — only a command-line run installs an origin, from `[[app]]` — and the
boot check is recorded as not yet built beside the mount expander.
