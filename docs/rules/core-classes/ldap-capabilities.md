Three deny-by-default capabilities under `[capabilities.ldap]` sit in front of a directory, in `db.*`'s shape. `ldap.connect` names the `[ldap.<name>]` blocks a program may open with `Ldap::connect`; `ldap.open` names the hosts an `Ldap\Settings` may reach with `Ldap::open`, and takes `*.` at a label boundary; `ldap.cleartext` names the hosts a bind may reach without TLS (`rule:security/ldap-cleartext-bind-is-granted-per-host`).

A block's address is the operator's, so a `connect`-named block is pre-approved and is not checked
against the outbound address policy, as `rule:core-classes/db-capabilities` reasons for a database. An
`open` target is the program's: its host is checked against `ldap.open` and its address against
`rule:security/net-address-policy` in full before anything is dialled.

`[ldap.<name>]` follows `[db.<name>]`: `url` (one `ldaps://` or `ldap://` URL, or a list), `base`,
`user`, `password` or `password_file` (`rule:config/a-secret-is-a-file-whose-content-is-the-value`),
`tls` (`"required"`, the default, or `"none"`), `tls_ca_file`, `timeout` and `pool`, and every table
refuses an unknown key. A block with no `user` binds anonymously. `Ldap\Settings` is the same fields as
a shape: `url` is a sink with no launderer, as `Settings.host` is for a database; `base` is
`Dn|string`; `user` accepts `tainted`, being a length-framed protocol field; `password` is `secret
tainted string`; and `tls` is the enum `Ldap::Tls`.

**A block's URL list fails over in order**, and so does every address one URL's host resolves to. A
server that does not answer, or whose TLS handshake fails, moves the walk on. A server that answers and
refuses the credentials stops it, because the next server would refuse them too and the walk would
only add to the account's lockout count.

A timeout has a default and no spelling for unlimited. Nothing is read from `ldap.conf`, `LDAPTLS_*` or
any other environment, so no setting one request's code makes can reach another request's connection,
and TLS trust is relaxed only where `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`
already allows it per host.

**Declined:**
- **A binding to OpenLDAP's `libldap`.** It brings process-wide options, a second TLS stack beside the
  one `rule:security/one-tls-client` allows, `ldap.conf` and the environment, and SASL through
  `libsasl2`, and a TLS option set for one connection can then apply to the next request's.
- **A tokio-based client crate.** It needs a second scheduler per core, which
  `rule:concurrency/one-scheduler` refuses; the BER codec is borrowed sans-IO and the sequencing is
  written over the runtime's own TCP and TLS streams (`rule:core-classes/db-crate-boundary`).
- **An unlimited timeout, and reading `ldap.conf` or `LDAPTLS_*`.** Both put a connection's behaviour
  outside what the operator's configuration states.
