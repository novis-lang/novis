# Handoff

## State

**Goal `http-client` — a program talks to a real API: bodies, headers, streams and pooled
connections. Stages 1–8 are on disk.** [ADR 0180](../decisions/0180.md) is the record and the home
of every decision this goal executes.

Stage 8 landed: a redirect hop to another origin — scheme, host or port differing — carries **none**
of the caller's headers, and a hop inside one origin carries all of them. `compose` asks
`same_origin` (`crates/nvs-stdlib/src/http/transport.rs:1712`), and the `traceparent` guard reads
what the hop carries rather than what the call holds, so a hop that left the caller's own behind
still sends ours.

**One deviation from the goal's § *Stage 8* prose, and it is the whole of stage 8's first item.**
The prose has a hop drop the three named headers *and every header whose value was `secret`*; a hop
drops every caller header instead. `secret` is checked once and erased before codegen
(`rule:security/secret-qualifier`), so a `secret string` is the same bytes as a plain one by the time
the transport holds it: nothing can carry per-header secretness into `Call::headers` without a
representation the language deliberately does not spend. Dropping all of them is the only way to drop
every secret one, and it errs on the priority-1 side. The rule fragment
`rule:http-server/a-cross-origin-redirect-drops-credentials` says so and is `shipped`; ADR 0180 § 9 is
frozen and still reads as the narrower form.

Nothing is blocked.

## Next group

**Stage 9: the trust roots are the operator's** — one file set, and a different one from stage 8's:
`crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/default.toml`, `crates/nvs-host/src/tls.rs`.

- [ ] **`[http.client.tls]` is a `System`-class block with `roots`, `min_version` and `keylog`** —
      the block joins `HttpClient` at `crates/nvs-config/src/tree.rs:507` and its shipped values
      `crates/nvs-config/src/default.toml:32`. Each `roots` entry is `"bundled"` or a PEM file
      resolved and trust-checked at boot exactly as `[db.<name>] tls_ca_file` is
      (`crates/nvs-config/src/db.rs:14`). The goal's § *Stage 9* table is what it executes, and
      `rule:config/three-changeability-classes` is the class.
- [ ] **`nvs_host::tls` parses `roots` once at boot, beside or in place of the bundled set** —
      the process-wide anchor set is `crates/nvs-host/src/tls.rs:86`, and it is the one place a PEM
      file is read. `min_version` raises the floor for every call from the same place.
- [ ] **`keylog` is refused at boot in `production` and announced in `development`** — the mode's
      own default is `crates/nvs-config/src/default.toml:32`, and the diagnostic names the key.
- [ ] **The client's first end-to-end `https` case** — a loopback origin with an `rcgen` certificate
      under a `roots` file, fetched through the transport rather than through `NvsTls` alone; the
      seam no case covers is `crates/nvs-stdlib/src/http/transport.rs:264`.

## Backlog

- The pack's `[context] rules` did not name `security/secret-qualifier`, and stage 8 could not be
  decided without it — add it, since stages 9 and 10 sit on the same qualifier.
- Stage 10's four relaxing options and their `[capabilities.tls]` grants — goal § *Stage 10*.
- `Core\Http\Client` has no outbound `http` trace event yet, so the rule's "no header value reaches a
  trace span" clause is guarded only over the transport's messages — ADR 0180 § *Standing decisions*.
- The REST package, OAuth and `Link`/RFC 9457 parsing stay out of this goal — goal § *Not this goal*.
