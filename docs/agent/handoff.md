# Handoff

## State

**Goal `outbound-proxy` — stage 3 is on disk and green: a `Core\Http\Client` call under
`[http.client.proxy] resolve = "local"` is tunnelled with `CONNECT` to the address the pin
approved, speaks TLS over it under the name the URL was approved for, and is pooled apart from a
direct connection to the same address.**

`crates/nvs-stdlib/src/http/transport.rs` holds `Proxy` (the block as a call carries it), `tunnel`
and `tunnel_head`; `one` forks on it between the pool draw and the TLS wrap, and `pool_key` carries
the operator's `url` text as its last field. `nvs_config::http::proxy_endpoint` is the one grammar
the boot's `E0645` and the transport's dial both take a `url` apart with.

Stage 4's config half was already green, and its credential half landed here:
`Proxy-Authorization: Basic` is built in `crates/nvs-stdlib/src/http.rs`'s `proxy_of` and written on
the `CONNECT` request alone. `HttpClientProxy::username`'s `[unread:]` trailer and `default.toml`'s
`# NOT IMPLEMENTED` note are both gone with it. `bypass` is matched in `Proxy::bypasses` and read by
`one`, but nothing tests it yet — that is the group below.

What stage 4 still owes is `resolve = "proxy"`, and it is a fork at the **door**, not in the
transport: under that word Novis must not resolve at all, because the network the word exists for
has no DNS route outward.

## Next group

**Stage 4: the proxy resolves, and what a bypass is exempt from** — one file set:
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **The door stops resolving under `resolve = "proxy"`** — `crates/nvs-stdlib/src/http.rs:2645`
      (`approved`, which `exchanged` calls) reads the block the same way
      `crates/nvs-stdlib/src/http.rs:3628` (`proxy_of`) does and, under the word and with the host
      not in `bypass`, calls `nvs_runtime::capability::pin_host`'s host half without the address
      half `crates/nvs-stdlib/src/http.rs:389` (`pin`) reaches — the scheme, the grant's host list
      and the tainted-URL check still hold, and only the address table is skipped. `Call::addresses`
      then arrives empty, so `crates/nvs-stdlib/src/http/transport.rs:1418`'s refusal of an empty
      approved set is what changes with it and its doc's *unreachable from source* stops being true.
      `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`.
- [ ] **`CONNECT` carries the host name, and so does the pool key** —
      `crates/nvs-stdlib/src/http/transport.rs:1643` writes the request line from a destination that
      is `SocketAddr` today, and `crates/nvs-stdlib/src/http/transport.rs:1769` (`pool_key`) takes
      the same value as its `socket` field: under the word there is no address to write in either,
      and the host and port are what identifies the destination in both. Test
      `resolve_proxy_sends_the_host_name_and_skips_only_the_address_check`.
      Same rule.
- [ ] **The two cases the fork makes assertable** —
      `a_bypassed_host_connects_directly_under_the_full_policy` (a `bypass` entry naming the URL's
      host under `resolve = "proxy"`: the proxy is never dialled *and* the address policy is asked,
      which is the half that only exists once the fork above does) and
      `no_proxy_environment_variable_is_ever_read` (`HTTP_PROXY`, `HTTPS_PROXY` and `NO_PROXY` set,
      and `crates/nvs-stdlib/src/http.rs:3628` still answers `None`).
      `rule:http-server/an-outbound-proxy-is-operator-configured`.

## Backlog

- Stage 5: flip both `[0182](../decisions/0182.md)` rules to `shipped` with `guardedBy` filled from
  this goal's tests, then `python tools/rules.py --render` — `docs/agent/loop-goal.md` § Stage 5.
- A `CONNECT` refusal walks the approved set one address at a time
  (`crates/nvs-stdlib/src/http/transport.rs:1606`); no case covers a set of two where the first is
  refused and the second opens.
- `nvs config dump` renders `resolve` beside `rule:security/net-address-policy`'s id — stage 4 of
  `docs/agent/loop-goal.md`, and unstarted.
