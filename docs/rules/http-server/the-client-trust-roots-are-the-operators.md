Which certificates the outbound client believes is the operator's decision and has no code-side spelling
at all. `[http.client.tls]` carries three keys, every one of them `System` class
(`rule:config/three-changeability-classes`) because they configure the one `ClientConfig` the process
shares (`rule:security/one-tls-client`):

| Key | Ships | Allows |
|---|---|---|
| `roots` | `["bundled"]` | Each entry is `"bundled"` — the compiled-in Mozilla set — or a PEM file. `["bundled", "/etc/novis/corp-ca.pem"]` adds a company CA; a list without `"bundled"` trusts only its files. Each file is resolved and trust-checked at boot exactly as `[db.<name>] tls_ca_file` is, and parsed by `nvs_host::tls` alone. |
| `min_version` | `"1.2"` | `"1.3"` raises the floor for every call. There is nothing below `1.2` to write. |
| `keylog` | unset | A file every session's secrets are appended to in the `SSLKEYLOGFILE` format, so an operator can read their own traffic. **Refused at boot in `production`**, naming the key; in `development` the boot says it is on, every start. |

`1.2` as the shipped floor is the one place the client trades a stronger default for reach: a great many
corporate and payment endpoints still speak nothing else, and a client that cannot reach them is a client
a deployment replaces with `curl`. Raising the floor is one line, and `nvs config dump` says what it
currently is.

`keylog` is refused rather than warned about, because a file of live session secrets is not a
configuration mistake a warning improves — the process must not start with it. A program relaxes *its
own* call's verification only through `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`, and
never through any of these three keys.
