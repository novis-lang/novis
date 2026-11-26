Which certificates an outbound connection believes is the operator's decision and has no code-side
spelling at all. `[tls.client]` carries three keys, every one of them `System` class
(`rule:config/set-changes-only-listed-keys`) because they configure the one `ClientConfig` the process
shares (`rule:security/one-tls-client`):

| Key | Ships | Allows |
|---|---|---|
| `roots` | `["bundled"]` | Each entry is `"bundled"` — the compiled-in Mozilla set — or a PEM file. `["bundled", "/etc/novis/corp-ca.pem"]` adds a company CA; a list without `"bundled"` trusts only its files. Each file is resolved and trust-checked at boot exactly as `[db.<name>] tls_ca_file` is, and parsed by `nvs_host::tls` alone. |
| `min_version` | `"1.2"` | `"1.3"` raises the floor for every connection. There is nothing below `1.2` to write. |
| `keylog` | unset | A file every session's secrets are appended to in the `SSLKEYLOGFILE` format, so an operator can read their own traffic. **Refused at boot in `production`**, naming the key; in `development` the boot says it is on, every start. |

The block is every outbound client's, not HTTP's: `Core\Http\Client`, `Core\Mail` and each
`[mail.<name>]` block, `Core\Ldap::open` and each `[ldap.<name>]` block, `Core\Db::open` and each
`[db.<name>]` block over PostgreSQL, MySQL and SQL Server, and the shared cache over `rediss://`. Each
of them reaches `NvsTls::over`, which hands the handshake this one configuration. A user who looked for
a program-wide CA setting under LDAP did not find it under an HTTP name, which is why the block is
named for TLS.

A block's own `tls_ca_file` **replaces** `roots` for that block, never adds to it, as `anchors_from` in
`nvs_host::tls` argues; every `tls_ca_file` doc says its default is `[tls.client] roots`. A program's
own call widens trust only through `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`, and
`Ldap::open` and `Db::open` take no per-call CA at all.

A handshake that fails for a reason a setting fixes says which one, worded once in
`crates/nvs-stdlib/src/tls_fix.rs` for every client. A certificate from an unknown issuer names the
block's `tls_ca_file` and `[tls.client] roots`, or `roots` alone where there is no block or the block
has no CA file (the shared cache). A certificate for another name lists the names it is for and names
the setting that holds the address, never a CA setting, because no CA fixes a name. An HTTP call that
named its own CAs with `tlsCa` gets no `roots` hint, because `roots` does not apply to it. Every other
handshake failure keeps the error `rustls` gave.

`1.2` as the shipped floor is the one place the client trades a stronger default for reach: a great many
corporate and payment endpoints still speak nothing else, and a client that cannot reach them is a client
a deployment replaces with `curl`. Raising the floor is one line, and `nvs config dump` says what it
currently is.

`keylog` is refused rather than warned about, because a file of live session secrets is not a
configuration mistake a warning improves — the process must not start with it. A program relaxes *its
own* call's verification only through `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`, and
never through any of these three keys.

- **Memory:** per process, the `roots` files, parsed once beside or in place of the bundled set. A call
  naming any per-call TLS option builds its own client configuration and frees it with the call, so that
  cost is O(in-flight). A hint costs one check of the error, on a handshake that already failed.

**Declined:**
- **A per-call CA for `Ldap::open` and `Db::open`.** HTTP's `tlsCa` under a `tls.anchors` grant is the
  shape it would take, and no report needs it; `[tls.client] roots` covers the company CA.
- **Keeping `[http.client.tls]` as a second name.** Pre-alpha, so it is an unknown key like any other.
