# Handoff

## State

**Goal `outbound-proxy` — an operator routes outbound calls through a forward proxy, and the grant says
what the address policy can no longer see. Stage 2 is done: nothing of the goal's code has landed yet.**

[0182](../decisions/0182.md) is on disk and accepted, and it carries the whole design the goal's
§ *Standing decisions* pre-authorized: the block is the operator's alone and the environment is never read,
`resolve` is mandatory with no default, `local` tunnels to the address Novis approved so the pin survives,
`proxy` narrows the policy to the URL's text and writes a `Warn` at every boot, and only
`Core\Http\Client` is proxied. Its two rules —
`rule:http-server/an-outbound-proxy-is-operator-configured` and
`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise` — are in the rulebook as
`designed`, `rule:security/net-address-policy` carries the one-sentence amendment, and the chapters are
rendered. Stage 5 flips both to `shipped` and fills `guardedBy` from the goal's tests.

The record and its rules are **one commit**, not two, because `conventions.md` § *A decision record* makes
them one — `records.py --check` refuses a record naming a rule the rulebook does not define.

## Next group

**Stage 3: the keystone — a tunnel that keeps the pin** — one file set: `crates/nvs-config/src/directive.rs`,
`crates/nvs-config/src/http.rs`, `crates/nvs-stdlib/src/http/transport.rs`.

- [ ] **The `[http.client.proxy]` block** — the registry rows beside `[http.client]`'s at
      `crates/nvs-config/src/directive.rs:126`, all `Class::System` and `Apply::Reload` (the `tls` row at
      `:133` is `Boot` and this is deliberately not), and the typed tree plus its boot refusals beside
      `[http.client.tls]`'s at `crates/nvs-config/src/http.rs:342`: a missing `resolve` naming both words, a
      third word, a non-`http://` `url`, a `bypass` entry with a port, a scheme, a `*` or a `/`, and
      `password` with `password_file`. `rule:http-server/an-outbound-proxy-is-operator-configured`.
- [ ] **The `CONNECT` tunnel under `resolve = "local"`** — `crates/nvs-stdlib/src/http/transport.rs:1344`
      (`fn one`, which is where an attempt opens or draws a connection): open to the proxy, send
      `CONNECT <approved address>:<port>` with `Host` naming the same, wait for a `2xx` under the same
      `connectTimeout` budget the direct path already clamps, then speak TLS with the launderer's server
      name, or plain HTTP, over it. An `http` destination is tunnelled too — never an absolute-form request
      line. `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`.
- [ ] **The pool key gains the proxy, and `CONNECT` refusals are two faults** —
      `crates/nvs-stdlib/src/http/transport.rs:1503` (`fn pool_key`, today
      `scheme|host|socket|identity|policy`), so a tunnelled and a direct connection are never confused and a
      reload strands the old value's connections; and in `fn attempts` at `:1290`, a `407` as a
      `RuntimeError` naming proxy authentication, any other refusal as an `IOError`, neither retried.
      `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`.

## Backlog

- Stage 4 — `resolve = "proxy"`, the boot `Warn`, `nvs config dump`'s rendering, `bypass`, and
  `Proxy-Authorization` on `CONNECT` alone — `docs/agent/loop-goal.md` § *Stage 4*.
- Stage 5 — flip both rules to `shipped` with `guardedBy` filled, then `python tools/rules.py --render` —
  `docs/agent/loop-goal.md` § *Stage 5*.
- The `[context] adrs` field named only `0058:In short`; 0180's front matter and heading list were read by
  hand for the client's shape. Add `0180:In short` and `0058:Consequences` to it.
- The `[context] rules` field was missing `config/reloadability-is-its-own-field` and
  `config/a-secret-is-a-file-whose-content-is-the-value`; both were fetched by hand and both are cited by
  the new rules.
- `crates/nvs-config/src/directive.rs` and `crates/nvs-config/src/http.rs` are not in `[context] modules`
  and stage 3 edits both — the driver's sweep picks them up only after they are committed.
