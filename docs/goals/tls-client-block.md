---
looper: 1
title: "one [tls.client] block sets whose certificates every outbound connection believes"
summary: "Reached when [http.client.tls] is [tls.client], every outbound TLS client that names no tls_ca_file is proven to trust its roots, a certificate from an unknown issuer names roots and tls_ca_file in its error, and the rule, the settings docs, the feature proofs and the website say so."
context: [crates/nvs-host/src/tls.rs, docs/rules/security/the-client-trust-roots-are-the-operators.md, docs/rules/security/one-tls-client.md]
stages:
  - id: 1
    title: "the block is [tls.client] everywhere in the tree's code, tests and settings files"
    status: done
    context: [crates/nvs-config/src/http.rs, crates/nvs-config/src/tree.rs, crates/nvs-config/src/directive.rs, crates/nvs-cli/src/config.rs, crates/nvs-cli/src/reload.rs]
  - id: 2
    title: "every outbound client that names no tls_ca_file is proven to trust [tls.client] roots, and an unknown issuer names both settings"
    status: done
    context: [crates/nvs-stdlib/src/mail/smtp.rs, crates/nvs-stdlib/src/ldap/mod.rs, crates/nvs-ldap/src/conn.rs, crates/nvs-cli/tests/live_config.rs]
  - id: 3
    title: "the rule, the settings docs, the feature proofs and the website say so"
    status: open
    context: [crates/nvs-config/src/default.toml, docs/examples/config/tls-client/about.md, website/src/data/changes.json]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: rename
    stage: 1
    name: "[tls.client] builds the outbound client and [http.client.tls] is an unknown key"
    run: [cargo, test, --test, resolve]
    tests: [a_tls_client_block_sets_the_outbound_roots, an_http_client_tls_block_is_an_unknown_key]
    timeout: 1800
  - id: every-client
    stage: 2
    name: "Ldap::open, an [ldap] block, a [mail] block, a [db] block and a shared cache over TLS trust a CA listed only in [tls.client] roots, a block's tls_ca_file replaces roots, an unknown issuer names the settings that fix it, and a certificate for another name lists the names it is for"
    run: [cargo, test, --test, client_roots]
    tests: [ldap_open_trusts_a_ca_in_tls_client_roots, an_ldap_block_without_a_ca_file_trusts_tls_client_roots, a_mail_block_without_a_ca_file_trusts_tls_client_roots, a_postgres_block_without_a_ca_file_trusts_tls_client_roots, a_shared_cache_over_tls_trusts_tls_client_roots, a_block_ca_file_replaces_tls_client_roots, an_unknown_issuer_from_ldap_open_names_tls_client_roots, an_unknown_issuer_from_a_block_names_both_settings, a_certificate_for_another_name_lists_its_names]
    timeout: 1800
  - id: rules
    stage: 3
    name: "the rulebook is consistent after the rule moves to the security chapter"
    run: [bun, nv, rules, --check]
    timeout: 600
  - id: close
    stage: 4
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, tls-client-block]
    timeout: 900
---

# one [tls.client] block sets whose certificates every outbound connection believes

## Why

The process has one outbound TLS client (`rule:security/one-tls-client`). `[http.client.tls]` builds
it at boot (`crates/nvs-cli/src/config.rs:799`, `crates/nvs-cli/src/shared.rs:146`), and
`NvsTls::over` hands every handshake that config (`anchors()`, `crates/nvs-host/src/tls.rs:1013`).
Every client that names no CA file of its own calls `over`: HTTP, PostgreSQL
(`crates/nvs-db/src/pg.rs:1143`), MySQL (`crates/nvs-db/src/mysql.rs:343`), SQL Server
(`crates/nvs-db/src/tds/prelogin.rs:443`), LDAP (`crates/nvs-ldap/src/conn.rs:351`), mail
(`crates/nvs-stdlib/src/mail/smtp.rs:163`) and the shared cache (`crates/nvs-stdlib/src/cache/redis.rs:631`).
So `roots`, `min_version` and `keylog` already apply to all of them.

Nothing says so. The block is named for HTTP, its doc says "an outbound `https` call"
(`crates/nvs-config/src/default.toml:580`), the rule is in the HTTP chapter, and each block's
`tls_ca_file` says its default is "the bundled Mozilla set" (`default.toml:792`, `:847`, `:892`),
which is wrong once `roots` names a file. A user report showed the cost. A deployment whose LDAP
server has a company CA could not reach it with `Ldap::open`, whose settings take no CA
(`crates/nvs-stdlib/src/ldap/mod.rs:1002`). They asked for a program-wide CA setting, and that
setting already existed under a name they had no reason to read.

## Standing decisions

**The name is `[tls.client]`.** A new top-level `tls` table in `Config`
(`crates/nvs-config/src/tree.rs:73`) with one sub-table, `client`, which takes `HttpClientTls`'s three
keys unchanged (`tree.rs:791`; the type becomes `TlsClient`). `[http.client]` keeps its pool, socket
and proxy keys, which are HTTP's alone. Pre-alpha: no old name is kept, no moved-key row and no rename
hint. `[http.client.tls]` is then an unknown key, refused as any other is. The changelog entry is the
one record, as a `BREAKING CHANGE`.

**Keys and behaviour do not change.** `roots`, `min_version` and `keylog` keep their values, defaults,
boot checks, diagnostics codes (`crates/nvs-diagnostics/src/code/config.rs:564`), `System` class,
reload behaviour (`crates/nvs-config/src/directive.rs:116`, `crates/nvs-cli/src/reload.rs:474`) and
the production refusal of `keylog`. Only the dotted key and every message that names it change. The
directive's feature id becomes `directive:tls.client`.

**A block's `tls_ca_file` still replaces `roots`** for that block, as `anchors_from` argues
(`crates/nvs-host/src/tls.rs:80`). Every `tls_ca_file` doc says: "default: `[tls.client] roots`".

**The unknown-issuer hint is one shape for every client.** `nvs_host::tls::unknown_issuer`
(`tls.rs:465`) already tells the case apart, and mail uses it (`smtp.rs:166`). Each client that
can fail a handshake names what fixes it, in its own error class:
- a block (`[db.<name>]`, `[ldap.<name>]`, `[mail.<name>]`): "set `[<kind>.<name>] tls_ca_file` to a
  PEM file of that CA, or add it to `[tls.client] roots`";
- a connection the program opened (`Ldap::open`, `Db::open`) and the shared cache: "add that CA to
  `[tls.client] roots`".
Every other handshake failure keeps its message. The hint is built where the block name is known: the
stdlib layer, never `nvs-ldap` or `nvs-db`.

**A certificate for another name gets its own hint.** `rustls` reports it as
`CertificateError::NotValidForNameContext { expected, presented }` (0.23.46; `NotValidForName`
without the list). A new `nvs_host::tls::names_presented(&io::Error) -> Option<Vec<String>>` beside
`unknown_issuer` returns the certificate's names, or an empty list for the variant without them.
The message says the certificate is for those names and not for the one connected to, and that the
fix is to set the block's `host` or `url` (or the program's URL) to one of them. It never names
`roots` or `tls_ca_file`, because no CA setting fixes a name. A user report met this right after the
unknown issuer. Both hints cover HTTP as well (`crates/nvs-stdlib/src/http/transport.rs:1902`), so
every client words the two cases the same way.

**The hint is worded once, in `crates/nvs-stdlib/src/tls_fix.rs`.** `nvs_host::tls::Refusal` is
the two cases as a plain value, so `nvs_ldap::Error` keeps it beside its message (an `io::Error` is
not `Clone`). The shared cache has no `tls_ca_file`, so its hint names `[tls.client] roots` alone,
and `[cache.shared] url` for a name (`Settings::BlockWithoutCaFile`). An HTTP call that named its
own CAs with `tlsCa` gets no roots hint for an unknown issuer, because the roots do not apply to it.

**No per-call CA for `Ldap::open` or `Db::open`.** HTTP's `tlsCa` under the `tls.anchors` grant
(`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`) is the shape one would take, and no
report needs it. It is not built here.

**The tests run `nvs` as a child process.** The client config is process-wide, so a test that installs
roots inside a shared test binary changes what every other test there trusts. The new integration test
`crates/nvs-cli/tests/client_roots.rs` starts the binary with a settings file whose `[tls.client]
roots` names a CA the test made with `rcgen` (as `live_config.rs:2054` does), against a loopback
listener that speaks TLS from the first byte (`ldaps://`, mail `tls = "implicit"`, `rediss://`), or
answers PostgreSQL's `SSLRequest` with `S` first. A completed handshake is the pass. The protocol after
it may fail, and the test reads only whether the error is a TLS one. MySQL and SQL Server share
PostgreSQL's call to `over` in `nvs-db` and get no test of their own.

**The rule moves.** `docs/rules/http-server/the-client-trust-roots-are-the-operators.md` moves to
`docs/rules/security/` with the same slug, its text names every client above, and its ten citations
are rewritten (`grep -rn the-client-trust-roots-are-the-operators`). Then `bun nv rules --render`.

**Cost.** Nothing at run time, in memory or on the request path. The rename moves one table, and the
hint is one check on a failed handshake.

**Website.** One `website/src/data/changes.json` entry of kind `breaking` for `directive:tls.client`:
the block is renamed, and it sets the trusted CAs of every outbound connection. The examples and the
attack move to `docs/examples/config/tls-client/` and `tests/hostile/config/tls-client/`.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt goes to `### Backlog` with what blocked it.

**Lanes.** A chain goal right behind `mail-complete`. It shares `crates/nvs-config/src/tree.rs`,
`default.toml` and `crates/nvs-stdlib/src/mail/smtp.rs` with it, and `mail-receive` after it adds an
`[imap.<name>]` block whose `tls_ca_file` doc follows this goal's wording.

## Stages

### 1. the block is [tls.client] everywhere in the tree's code, tests and settings files

**Split:** one session. The edits are mechanical across about 30 files. `bun nv splice --patch` takes them in one
call per crate.

- [x] `crates/nvs-config/src/tree.rs:73` and `:791`: `Config.tls: Option<Tls>` with `client:
  Option<TlsClient>`; `Http.client.tls` (`tree.rs:773`) is removed. `crates/nvs-config/src/http.rs:442`
  to `:600` read the new path, and the dotted keys in `defaults.rs:558`, `directive.rs:116`,
  `resolve.rs:629`, `db.rs:161`, `snapshot.rs:256` and `capability.rs:131` follow.
- [x] `crates/nvs-diagnostics/src/code/config.rs:564` and `crates/nvs-diagnostics/src/code/warnings.rs:89`:
  every message names `[tls.client]`.
- [x] `crates/nvs-cli/src/config.rs:768`, `:1197`, `:1963` and `crates/nvs-cli/src/reload.rs:234`,
  `:474`, `:922`; `crates/nvs-stdlib/src/http.rs:2934` reads `tls.client.min_version`, and its
  messages at `:2353` and `:2941` name it; the doc comments in `crates/nvs-host/src/tls.rs` and
  `crates/nvs-stdlib/src/http/transport.rs:2403`.
- [x] Tests and settings files: `crates/nvs-config/tests/directives.rs:1`, `crates/nvs-config/tests/resolve.rs:1`,
  `crates/nvs-cli/tests/live_config.rs:1`, `run_command.rs:1`, `package_download.rs:1`,
  `tools/nv/test/registry-browser.test.ts:1`, `nvs.toml:1` and `examples/cache-shared-tls.toml:1`.
- [x] `crates/nvs-config/tests/resolve.rs:1`: the two tests of the check `rename`, then `cargo test
  --test resolve` once. `rg -n 'http\.client\.tls' crates tools nvs.toml examples` prints only
  `an_http_client_tls_block_is_an_unknown_key`, which has to write the old name.

### 2. every outbound client that names no tls_ca_file is proven to trust [tls.client] roots, and an unknown issuer names both settings

- [x] `crates/nvs-cli/tests/client_roots.rs:1` (new): the five trust tests of the check `every-client`,
  shaped per § *The tests run `nvs` as a child process*.
- [x] `crates/nvs-stdlib/src/ldap/mod.rs:299` (`fault_of`) or the open and connect paths that call it
  (`:1002`, `:634`): an unknown issuer gives the hint of § *The unknown-issuer hint*.
  `crates/nvs-ldap/src/conn.rs:353` keeps the `io::Error` reachable so the stdlib layer can ask
  `unknown_issuer`.
- [x] `crates/nvs-stdlib/src/mail/smtp.rs:166`: the message also names `[tls.client] roots`.
- [x] `crates/nvs-stdlib/src/db/open.rs:604` and the place a `[db]` handshake failure becomes a
  `Db\DbError` (not checked: find it from `crates/nvs-db/src/pg.rs:1143`); `crates/nvs-stdlib/src/cache/redis.rs:631`.
- [x] `crates/nvs-host/src/tls.rs:465`: `names_presented` beside `unknown_issuer`, with a unit test
  on a certificate made for another name. Every handshake site above and HTTP's
  (`crates/nvs-stdlib/src/http/transport.rs:1902`) gives both hints of § *Standing decisions*.
- [x] The three hint tests of `every-client`, then `cargo test --test client_roots` once.

### 3. the rule, the settings docs, the feature proofs and the website say so

- [x] `docs/rules/http-server/the-client-trust-roots-are-the-operators.md:1` moves to the security
  chapter per § *The rule moves*, and its table names `[tls.client]` and every client of § *Why*.
  `docs/rules/security/tls-trust-is-relaxed-only-under-a-host-grant.md:1` and every other citation
  follow. `bun nv rules --render`, then `bun nv rules --check`.
- [x] `crates/nvs-config/src/default.toml:579`: the block moves out of `[http.client]` to its own
  `[tls.client]`, and its comment says it is every outbound connection's trust. `:241`, `:792`,
  `:847`, `:892` and the `rediss://` lines at `:939` say "default: `[tls.client] roots`".
- [x] `docs/examples/config/http-client-tls/about.md:1` and `tests/hostile/config/http-client-tls/01-choosing-who-is-believed-from-inside-a-request.nvs:1`
  move to `tls-client/`, rewritten to AGENTS.md § *Text an end user reads*. `bun nv proofs --owed
  --goal tls-client-block` lists what `directive:tls.client` still owes, and each is paid. The
  example's `nvs.toml` and `.nvs` and two more attacks still name `http.client.tls`:
  `tests/hostile/config/http-client-socket/01-removing-the-one-bound-a-request-is-allowed-to-set.nvs:73`
  and `tests/hostile/config/http/01-bending-the-response-policy-from-inside-a-request.nvs:36`, `:91`.
- [ ] `website/src/data/changes.json:1`: the entry of § *Website*. Then `bun nv site --check stale`,
  `changes` and `review` are green.
  - [x] the entry is written, and `site --check changes` is green
  - [x] the newcomer review of the 14 texts ran, and the TLS texts' findings are fixed
  - [ ] the remaining findings are fixed, and the 14 review keys are stamped
  - [ ] the 20 stale texts are reread and stamped, and `site --check stale` is green

### 4. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done. Stage 3's first three items landed, and the check `rules` is green:

- `ddf5d3e031`: the rule is `docs/rules/security/the-client-trust-roots-are-the-operators.md`,
  naming every client, `tls_ca_file` replacing `roots`, the two hints and the Declined list. Its
  metadata moved to `data/rules/security/`, and every citation follows.
- `3670009ee1`: `default.toml` has its own `[tls.client]` section before `[http.*]`, and every
  `tls_ca_file` doc (the file, `tree.rs`, the four `nvs-db` drivers) says `[tls.client] roots`.
- `4a75c2a147`: the example and attack live under `docs/examples/config/tls-client/` and
  `tests/hostile/config/tls-client/`; the two other attacks name `tls.client.*`. All three
  directives' proofs run green (`proofs --run --id`).
- `f71aa0f99f`: the breaking `changes.json` entry (`site --check changes` green), the HTTP help's
  `tlsCa` and returns text, and the mail guide's `tls_ca_file` line name `[tls.client] roots`.

The newcomer review ran over the 14 keys `bun nv site --review` listed (saved list:
`help:Core\Http\Client::{delete,get,head,openSocket,openStream,patch,post,put,request}`, the three
`tls-client` texts, `sending-mail.mdx` and `changes.json`). The TLS texts' findings are fixed. No key
is stamped yet.

The nightly sweep's nine failures (run 45813627231) tested `4e43e1fa2e`, older than both fixes:
`c115a735a1` (Miri) and `28a4d7a6bf` (conformance). Nothing to do.

### Next

Stage 3's last item, then stage 4.

1. The review's remaining findings, all in text this goal did not write: the HTTP client help cards'
   shared params and throws blocks (`crates/nvs-stdlib/src/http.rs`, from `:2300`): the `request`
   sentence that appears on every member, "A body that is not text does not cause this error", the
   `openStream` sentence on other members, "`1` is the same as the default", "pinned address",
   `head`'s returns saying "body", and `openStream`'s `deadline` text contradicting the shared one.
   And `sending-mail.mdx`: "nothing else", "token", the `.note` and `unusedCss()` pair, and
   `inReplyTo` not saying it takes message ids. Fix each, rebuild, then
   `bun nv site --review --stamp <the 14 keys>`. The `help.patch`-style edit goes through
   `bun nv splice`.
2. `bun nv site --check stale` names 20 texts (the `Core\Http\Client` about.md files,
   `Core\Mail::send`, `directive:http`, `http.client.proxy`, `http.client.socket`,
   `directive:tls.client`). None mentions TLS trust; reread each and `bun nv site --stamp` it.
3. `bun nv proofs --owed --goal tls-client-block` then owes `perf` (stale) for 28 `Core\Http*`
   members, because `http.rs` changed. Re-measure them as `proofs --id` says, and per the playbook,
   run `bun nv scaling --budget` before any `--write`.
4. Stage 4: `bun nv close --goal tls-client-block`.

Expected red: none in tests. `site --check stale` and `review` stay red until 1 and 2.

### Backlog
