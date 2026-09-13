# Handoff

## State

**Goal `outbound-proxy` — stage 4's door and transport halves are on disk and green: under
`[http.client.proxy] resolve = "proxy"` the door approves a destination it never resolves, the `CONNECT`
carries that destination's own `host:port`, and a `bypass` entry puts a host back under the whole address
policy and dials it directly.**

`crates/nvs-stdlib/src/http.rs:415` (`pin_unless_the_proxy_resolves`) is the fork both call doors take —
`approved` for the member's own URL, `repinned` for a redirect hop — and the empty address set it answers
with is what the transport reads as *ask for this one by name*. `transport::Proxy` carries `by_name` and
answers `resolves(host)`, which is the one question the door and the transport both ask, so a bypassed
host is pinned and dialled like any other. `transport::reached_at` gives a destination one spelling for
the `CONNECT` line, the pool key, the trace and an `IOError`; `HttpSpan`'s address is an `Option` because
a call that never learned one has nothing honest to report there.

Stage 4's config half — the two refusals, the boot `Warn`, the secret pair — was already green, and the
credential half landed before this group.

## Next group

**Stage 4: the launderer under the same word** — one file set: `crates/nvs-stdlib/src/http.rs`.

- [ ] **`allowUrl` stops resolving under `resolve = "proxy"` too** — `crates/nvs-stdlib/src/http.rs:481`
      (`nvs_core_http_allow_url`) calls `pin`, where every call door now calls
      `crates/nvs-stdlib/src/http.rs:415` (`pin_unless_the_proxy_resolves`), so a *tainted* URL cannot be
      laundered at all in the network the word exists for — the resolver it reaches has no route outward.
      The `Target` it builds then carries an empty address array, and
      `crates/nvs-stdlib/src/http.rs:2720` (`addresses_of`) reads an empty slot as one it cannot parse:
      that refusal has to tell a set the door approved empty from a slot another writer mangled, and its
      doc's reason for folding the two together is what changes with it.
      `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`.
- [ ] **A `Target` with no address is refused where nothing tunnels it** —
      `crates/nvs-stdlib/src/http/transport.rs:1517` is the direct arm's `Fault::fatal` for an approved
      set with no address in it, reachable once a `Target` can carry one: a block reloaded from `proxy`
      to `local` between the laundering and the call is the path. A thrown `RuntimeError` naming the
      reload is the answer a program can degrade around; a `FATAL` is not.
      `rule:errors/escalation-ladder`.
- [ ] **The case for both** — `crates/nvs-stdlib/src/http.rs:5017` is where the door's own proxy cases
      sit (`resolve_proxy_sends_the_host_name_and_skips_only_the_address_check` and the `AT_THE_PROXY`
      fixture beside it): a tainted URL laundered under the word answers a `Target` the member then sends
      by name, and the same `Target` under `local` is refused rather than fatal.

## Backlog

- Stage 5: flip both `[0182](../decisions/0182.md)` rules to `shipped` with `guardedBy` filled from this
  goal's tests, then `python tools/rules.py --render` — `docs/agent/loop-goal.md` § Stage 5.
- `nvs config dump` renders `resolve` beside `rule:security/net-address-policy`'s id —
  `crates/nvs-cli/src/config.rs:611` (`dump`), stage 4 of `docs/agent/loop-goal.md`, and unstarted.
- A `CONNECT` refusal walks the approved set one address at a time
  (`crates/nvs-stdlib/src/http/transport.rs:1649`); no case covers a set of two where the first is
  refused and the second opens.
