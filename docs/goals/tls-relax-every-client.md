---
looper: 1
title: "every outbound TLS client relaxes trust the same way, under the same host grants"
summary: "Reached when every outbound TLS client (PostgreSQL, MySQL, MariaDB, SQL Server, LDAP, SMTP, IMAP and Redis) takes the four relaxations Core\\Http\\Client takes, as block keys and as Core\\Db::open and Core\\Ldap::open options, each only under its [capabilities.tls] host grant; Core\\Db::Tls is gone; a relaxed connection is pooled apart from a verifying one; a connection in plaintext stays refused; and the rules, the feature proofs and the website say so."
context: [crates/nvs-host/src/tls.rs, docs/rules/security/tls-trust-is-relaxed-only-under-a-host-grant.md, docs/rules/security/one-tls-client.md, crates/nvs-config/src/capability.rs]
stages:
  - id: 1
    title: the rules say every outbound TLS client takes the same relaxations
    status: open
    context: [docs/rules/core-classes/db-safe-connection-defaults.md, docs/rules/security/the-client-trust-roots-are-the-operators.md, docs/rules/enums/an-enum-lives-in-the-class-that-uses-it.md, docs/agent/doc-style.md]
  - id: 2
    title: "one door into the TLS client, and nvs.toml reads the relaxation keys on every client block"
    status: open
    context: [crates/nvs-config/src/tree.rs, crates/nvs-config/src/db.rs, crates/nvs-config/src/ldap.rs, crates/nvs-config/src/mail.rs, crates/nvs-config/src/imap.rs, crates/nvs-config/src/default.toml]
  - id: 3
    title: "the database drivers and Core\\Db::open take the policy, and Core\\Db::Tls is gone"
    status: open
    context: [crates/nvs-db/src/pg.rs, crates/nvs-db/src/mysql.rs, crates/nvs-db/src/tds/prelogin.rs, crates/nvs-stdlib/src/db/open.rs, crates/nvs-stdlib/src/db/registry.rs, crates/nvs-stdlib/src/http.rs, crates/nvs-cli/tests/client_roots.rs]
  - id: 4
    title: "LDAP, SMTP, IMAP and Redis take the policy, and Core\\Ldap::open takes the options"
    status: open
    context: [crates/nvs-ldap/src/conn.rs, crates/nvs-stdlib/src/ldap/mod.rs, crates/nvs-stdlib/src/mail/smtp.rs, crates/nvs-stdlib/src/imap/session.rs, crates/nvs-stdlib/src/cache/redis.rs, crates/nvs-stdlib/src/tls_fix.rs, crates/nvs-cli/tests/client_roots.rs]
  - id: 5
    title: the feature proofs, the help, the reference and the website say so
    status: open
    context: [docs/agent/testing.md, docs/agent/prose.md, website/src/data/changes.json]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: config
    stage: 2
    name: "each relaxation key boots on every client block whose host its grant lists, and fails at boot naming the grant where it does not"
    run: [cargo, test, --test, tls_relax]
    timeout: 1800
  - id: db
    stage: 3
    name: "a PostgreSQL and a MySQL block reach a self-signed server under each relaxation and its grant, and a verifying connection is never drawn from a relaxed pool"
    run: [cargo, test, --test, client_relax, db_]
    timeout: 1800
  - id: tds
    stage: 3
    name: "the SQL Server handshake accepts a self-signed server under a relaxed policy and refuses it under the default one"
    run: [cargo, test, --lib, tds::prelogin]
    timeout: 1800
  - id: db-open
    stage: 3
    name: "Core\\Db::open throws naming the grant for each TLS option whose host the grant does not list, before it connects"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/db-open-tls-options-need-their-host-grant.nvst]
    timeout: 1800
  - id: clients
    stage: 4
    name: "an LDAP, an SMTP, an IMAP and a Redis client reach a self-signed server under each relaxation and its grant, and only there"
    run: [cargo, test, --test, client_relax]
    timeout: 1800
  - id: proofs
    stage: 5
    name: "every feature the goal's work reaches has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, tls-relax-every-client]
    timeout: 600
  - id: close
    stage: 6
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, tls-relax-every-client]
    timeout: 900
---

# every outbound TLS client relaxes trust the same way, under the same host grants

## Why

`rule:security/tls-trust-is-relaxed-only-under-a-host-grant` says verification on "an outbound call"
is relaxed under a `[capabilities.tls]` grant. Only `Core\Http\Client` implements it:
`crates/nvs-stdlib/src/http/transport.rs:1902` is the one caller of `NvsTls::over_policy`
(`crates/nvs-host/src/tls.rs:283`). Seven other clients call `NvsTls::over` or `over_bundle` with no
policy at all: PostgreSQL (`crates/nvs-db/src/pg.rs:1142`), MySQL and MariaDB
(`crates/nvs-db/src/mysql.rs:342`), SQL Server (`crates/nvs-db/src/tds/prelogin.rs:442`), LDAP
(`crates/nvs-ldap/src/conn.rs:350`), SMTP (`crates/nvs-stdlib/src/mail/smtp.rs:168`), IMAP
(`crates/nvs-stdlib/src/imap/session.rs:157`) and Redis (`crates/nvs-stdlib/src/cache/redis.rs:649`).

So a deployment can reach an HTTP origin with a self-signed certificate, and cannot reach a database
with one at all. The common case is SQL Server with no certificate provisioned: it presents a
self-signed fallback certificate that it generates again at every service start, so neither a pin nor
a private CA reaches it. `Core\Db::Tls` (`crates/nvs-stdlib/src/db/registry.rs:860`) already declares
`Required` and `VerifyCa`, the database spellings of `tlsVerify: false` and `tlsVerifyHost: false`,
and throws for both (`crates/nvs-stdlib/src/db/open.rs:613`). That is a second, broken way to write
the same four relaxations.

The user's call: **one relaxation set for every outbound TLS client**, gated by the grants that exist.
`rule:core-classes/db-safe-connection-defaults` declined only plaintext, and that decline stands.

## Standing decisions

- **The four relaxations and the version floor, everywhere.** Every client that opens TLS takes what
  `Core\Http\Client` takes. A program-built connection takes the same option names HTTP uses:
  `tlsCa`, `tlsPin`, `tlsVerifyHost: false`, `tlsVerify: false` and `tlsMinVersion`. That is
  `Core\Db::open`'s settings and `Core\Ldap::open`'s settings. An operator's block takes the snake-case
  keys `tls_pin` (a string or a list), `tls_verify_host = false`, `tls_verify = false` and
  `tls_min_version`. The blocks are `[db.<name>]`, `[ldap.<name>]`, `[mail.<name>]`,
  `[imap.<name>]` and `[cache.shared]`. A block's `tls_ca_file` stays the block's anchors, as now,
  and needs no grant: it is the operator's own trust-checked file. `true` is accepted for
  `tls_verify` and `tls_verify_host` and changes nothing.
- **Every relaxation still needs its host grant.** A block key whose host the matching
  `[capabilities.tls]` grant does not list fails at boot and at reload, naming the key and the grant.
  Both sides are static configuration, so the earliest point is boot. A program option throws a
  `LogicError` naming the grant at the call, before any connection, exactly as `judge_trust` does
  (`crates/nvs-stdlib/src/http.rs:2929`). The grants themselves do not change, and their boot line
  per relaxed host keeps printing.
- **One door into the TLS client.** `CallPolicy` (`crates/nvs-host/src/tls.rs:1172`) gains the block's
  bundle path (`bundle: Option<PathBuf>`). Every caller goes through `NvsTls::over_policy`, and `over`
  and `over_bundle` are removed, so a client cannot skip the policy again. Safe fallback if a caller
  cannot take a policy: it passes `CallPolicy::default()` and its test proves it verifies.
- **A relaxed connection is pooled apart.** The policy joins every client pool's key: the database
  key (`crates/nvs-stdlib/src/db/open.rs:696`, whose doc then drops "and `tls`" from what it leaves
  out), and the LDAP, mail, IMAP and Redis pools. A connection opened under a relaxation never serves
  one that verifies.
- **`Core\Db::Tls` and the `tls` settings key of `Core\Db::open` are removed.** Pre-alpha, so no
  rename help (`changes.json` is the record). `docs/examples/types/Db-Tls/` is deleted. The
  conformance case `tests/conformance/core/db-open-refuses-a-tls-mode-weaker-than-verify-full.nvst`
  becomes `db-open-tls-options-need-their-host-grant.nvst`. `Core\Ldap::Tls` stays: it picks the
  transport (`required`, `starttls`, `none`), not the trust.
- **Plaintext stays refused.** `rule:core-classes/db-safe-connection-defaults`'s Declined entry is
  unchanged, and so is LDAP's cleartext grant and mail's `tls = "none"` rules. A relaxation is always
  an encrypted session.
- **The mail relay's relaxation does not reach its OAuth token endpoint.** The token fetch
  (`crates/nvs-stdlib/src/mail/oauth.rs:316`) is a different host and keeps verifying, with the block's
  `tls_ca_file` as now.
- **The unknown-issuer hint names `tls_ca_file` first, then `tls_pin`, never `tls_verify`**
  (`crates/nvs-stdlib/src/tls_fix.rs:66`). A hint must not steer a reader to the widest relaxation.
- **The docs say what `tls_verify = false` costs on a database.** SQL Server's login sends the password
  inside TLS with only a reversible scramble, so an unverified session lets a man-in-the-middle read it.
  PostgreSQL's SCRAM does not leak the password, but the queries and rows are readable. The reference
  and `about.md` say this in plain words, and recommend a real certificate plus `tls_ca_file`.
- **Tests.** Clients are tested through an `nvs` child process in a new
  `crates/nvs-cli/tests/client_relax.rs`, as `crates/nvs-cli/tests/client_roots.rs` does, against its
  self-signed servers. Database tests are named `db_*` so stage 3's check runs them alone. SQL Server
  is proven by a unit test in `crates/nvs-db/src/tds/prelogin.rs` beside its handshake tests. MySQL
  and MariaDB share one test. The config test is `crates/nvs-config/tests/tls_relax.rs`.
- **Lanes.** The stages run in order. Stage 2 is the keystone for 3 and 4, and 3 and 4 both edit
  `crates/nvs-cli/tests/client_relax.rs`. `file-transfer`, next on the chain, builds its FTPS client on
  `over_policy` and gives its block the same keys.

## Stages

### 1. the rules say every outbound TLS client takes the same relaxations

**Does:** the rule fragments say what this goal builds, before the code.

- [ ] `docs/rules/security/tls-trust-is-relaxed-only-under-a-host-grant.md:1`: every outbound TLS
  client, the block keys beside the program options, the boot check of a block key against its grant,
  and the policy in every pool key. Declined: a database-only spelling (`Core\Db::Tls`), because two
  ways to write one relaxation drift apart.
- [ ] `docs/rules/core-classes/db-safe-connection-defaults.md:5`: TLS is verified by default and relaxed
  only under the security rule. Nothing names a weaker mode any more. The plaintext decline stays.
- [ ] `docs/rules/security/one-tls-client.md:8`: one door, `over_policy`, for every client.
- [ ] `docs/rules/enums/an-enum-lives-in-the-class-that-uses-it.md:42`: replace the `Core\Db::Tls`
  example row with another nested Core enum.

### 2. one door into the TLS client, and nvs.toml reads the relaxation keys on every client block

**Does:** the host seam and the configuration every later stage reads.

- [ ] `crates/nvs-host/src/tls.rs:1172`: `CallPolicy` gains `bundle`, `config_for` reads it, and
  `over` (`:224`) and `over_bundle` (`:241`) are removed. Every caller passes a policy, the default
  where it has nothing to relax, so the tree builds with no behaviour change.
- [ ] `crates/nvs-config/src/tree.rs:1189`: `tls_pin`, `tls_verify_host`, `tls_verify` and
  `tls_min_version` on `Database`, and the same on the LDAP (`:970`), mail (`:1017`), IMAP (`:1078`)
  and `[cache.shared]` structs.
- [ ] `crates/nvs-config/src/capability.rs:875`: at boot and reload, each block key is checked against
  its grant for the block's host, and fails naming the key and the grant. A pin is checked for the
  `sha256//<base64>` shape at boot. A `tls_min_version` below `[tls.client] min_version` fails.
- [ ] `crates/nvs-config/src/default.toml:808`: each block documents the keys beside its `tls_ca_file`,
  and `[capabilities.tls]` (`:235`) says its grants reach every client.
- [ ] `crates/nvs-config/tests/tls_relax.rs:1`: every key on every block, granted and not granted.

### 3. the database drivers and Core\Db::open take the policy, and Core\Db::Tls is gone

**Does:** every database connection carries a policy, and a program can ask for one.

- [ ] `crates/nvs-db/src/pg.rs:1142`, `crates/nvs-db/src/mysql.rs:342` and
  `crates/nvs-db/src/tds/prelogin.rs:442` hand their target's policy to `over_policy`. The targets
  (`crates/nvs-db/src/tds/mod.rs:236` and its pg and MySQL peers) carry a `CallPolicy` in place of
  `tls_ca_file`.
- [ ] `crates/nvs-stdlib/src/db/registry.rs:89`: the `tls` field and `TLS` enum (`:873`) are removed, and
  the five option fields are added with HTTP's types. `settings_tls` (`crates/nvs-stdlib/src/db/open.rs:613`)
  is replaced by a grant check that shares `judge_trust`'s table rather than copying it.
- [ ] `crates/nvs-stdlib/src/db/open.rs:696`: the policy joins the pool key, and its doc says so.
- [ ] `tests/conformance/core/db-open-tls-options-need-their-host-grant.nvst:1` replaces the old case,
  and `docs/examples/types/Db-Tls/` is deleted.
- [ ] `crates/nvs-cli/tests/client_relax.rs:1`: `db_*` tests for PostgreSQL and MySQL under each
  relaxation, and the pool test. `crates/nvs-db/src/tds/prelogin.rs:500`: the SQL Server unit test.

### 4. LDAP, SMTP, IMAP and Redis take the policy, and Core\Ldap::open takes the options

**Does:** every other client carries a policy.

- [ ] `crates/nvs-ldap/src/conn.rs:350` takes a policy. `crates/nvs-stdlib/src/ldap/mod.rs:280`:
  `Settings` carries it, `Core\Ldap::open`'s settings gain the five options with their grant check,
  and the block's keys fill it (`:460`).
- [ ] `crates/nvs-stdlib/src/mail/smtp.rs:168` and `crates/nvs-stdlib/src/imap/session.rs:157` take
  the block's policy, and the mail and IMAP pool keys include it.
- [ ] `crates/nvs-stdlib/src/cache/redis.rs:649` takes `[cache.shared]`'s policy.
- [ ] `crates/nvs-stdlib/src/tls_fix.rs:66`: the hint names `tls_ca_file`, then `tls_pin`.
- [ ] `crates/nvs-cli/tests/client_relax.rs:1`: LDAP, SMTP, IMAP and Redis tests, granted and not.

### 5. the feature proofs, the help, the reference and the website say so

**Does:** what this goal owes the reader.

**Split:** an `Explore` subagent first lists every website page, `about.md`, example and help text that
names `Core\Db::Tls`, `tls_ca_file` or `capabilities.tls`. The pages then go to one writing subagent.

- [ ] Every page that subagent lists is rewritten, reread and stamped (`bun nv site --stamp`).
  `website/src/content/docs/syntax/values/enums.mdx:3` drops `Core\Db::Tls` from `covers`.
- [ ] `Core\Db::open` and `Core\Ldap::open`'s `about.md` and help cards gain the options, with the
  password warning from the standing decisions.
- [ ] `website/src/data/changes.json:1`: `removed` `Core\Db::Tls`, `changed` `Core\Db::open`, `changed`
  `Core\Ldap::open`, and `changed` for each block and for `[capabilities.tls]`.
- [ ] `bun nv proofs --owed --goal tls-relax-every-client` names nothing.

### 6. the goal is verified once, and every failure is fixed

- [ ] The newcomer review ([prose.md](../agent/prose.md) § *A newcomer reads it*), so `bun nv site --check review` is green.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Not started. The rules come first (stage 1).

### Next

- [ ] Stage 1: `docs/rules/security/tls-trust-is-relaxed-only-under-a-host-grant.md:1`,
  `docs/rules/core-classes/db-safe-connection-defaults.md:5`, `docs/rules/security/one-tls-client.md:8`,
  `docs/rules/enums/an-enum-lives-in-the-class-that-uses-it.md:42`.

### Backlog
