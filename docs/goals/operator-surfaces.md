---
looper: 1
title: operator tools print no secret and trust no other local account
summary: "Reached when no printer of configuration values shows a secret directive, a test holds every credential-named key to the secret table, the Windows ownership check reads the whole access list, and the debug viewer answers only a request that carries its token."
context: [docs/rules/config/ownership-is-the-trust-boundary.md, docs/rules/config/a-secret-is-a-file-whose-content-is-the-value.md, docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md]
stages:
  - id: 1
    title: a dump redacts every secret directive
    status: done
    context: [crates/nvs-config/src/secret.rs, crates/nvs-config/src/audit.rs, crates/nvs-config/tests/secret.rs, crates/nvs-cli/tests/config_audit.rs, docs/rules/config/check-and-dump-audit-the-tree-offline.md]
  - id: 2
    title: the Windows ownership check reads the whole access list
    status: done
    context: [crates/nvs-config/src/trust.rs, crates/nvs-config/tests/trust.rs]
  - id: 3
    title: the debug viewer answers only a request that carries its token
    status: done
    context: [crates/nvs-cli/src/debug/listen.rs, crates/nvs-cli/src/debug/secure.rs, crates/nvs-cli/tests/debug_viewer.rs]
  - id: 4
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: no-printer-shows-a-secret
    stage: 1
    name: "for every row of the secret table, no printer of configuration values shows the value, inline or from a file"
    run: [cargo, test, --test, secret, --, no_printer_shows_a_secret_directive, every_key_named_like_a_credential_is_a_secret_or_listed_with_a_reason]
    timeout: 1800
    tests:
      - no_printer_shows_a_secret_directive
      - every_key_named_like_a_credential_is_a_secret_or_listed_with_a_reason
  - id: dump-prints-no-inline-secret
    stage: 1
    name: "nvs config dump, dump --origin and dump --toml print <secret> for an inline secret directive"
    run: [cargo, test, --test, config_audit, --, dump_prints_no_inline_secret_directive]
    timeout: 1800
    tests:
      - dump_prints_no_inline_secret_directive
  - id: any-other-writer-is-a-breach
    stage: 2
    name: "a path another account may write fails the ownership check on every platform, and the owner, SYSTEM and Administrators pass"
    run: [cargo, test, --test, trust, --, a_write_entry_for_any_other_account_is_a_breach, a_write_entry_for_a_trusted_account_passes]
    timeout: 1800
    tests:
      - a_write_entry_for_any_other_account_is_a_breach
      - a_write_entry_for_a_trusted_account_passes
  - id: viewer-needs-its-token
    stage: 3
    name: "on loopback the viewer answers 401 on every path without the token, and the printed link opens it"
    run: [cargo, test, --test, debug_viewer, --, on_loopback_every_path_needs_the_token, the_printed_loopback_link_opens_the_viewer]
    timeout: 1800
    tests:
      - on_loopback_every_path_needs_the_token
      - the_printed_loopback_link_opens_the_viewer
---

# operator tools print no secret and trust no other local account

## Why

Three tools an operator runs give a local reader more than the rules say.

**`nvs config dump` prints an inline secret in clear.** A secret directive has two ways to write
it: `password_file = "..."` and inline `password = "..."`. `materialize` moves only the file half
out of the merged table (`crates/nvs-config/src/secret.rs:392`). The inline half stays in the table
(`crates/nvs-config/src/secret.rs:405`). `Audit::listing` prints every leaf of the table and renders
`<secret>` only for the keys of its `secrets` map (`crates/nvs-config/src/audit.rs:85`). `dump
--toml` serializes the whole table (`crates/nvs-cli/src/config.rs:1726`). So `db.<name>.password`,
`http.csrf_key`, `cache.shared.password` and every other inline row of `SECRETS`
(`crates/nvs-config/src/secret.rs:161`) are printed. `Request::all` does check `is_secret_key`
(`crates/nvs-config/src/request.rs:597`), so the runtime reader is right and the offline one is not.
e97554bd5f added `strip_inline` for `[secrets]` and `[app.secrets]` and left the secret directives
(`crates/nvs-config/src/secret.rs:498`). `rule:config/check-and-dump-audit-the-tree-offline` names
the dump as the output to paste into CI, so the values reach CI logs and tickets. A leaked
`csrf_key` lets its reader forge a token every unsafe verb accepts.

**The Windows ownership check passes most accounts with write access.** On Unix the check refuses
any group or world write bit (`crates/nvs-config/src/trust.rs:578`). On Windows it asks about five
well-known groups only (`UNTRUSTED`, `crates/nvs-config/src/trust.rs:772`), in one loop
(`crates/nvs-config/src/trust.rs:1067`). An allow entry for a named user, a custom group,
`INTERACTIVE` or `SERVICE` is never read. 13c4cc743c lets an owner in `Administrators` pass, so a
tree an administrator copied into a user's profile now passes the owner test and reaches this one.
A local account with such an entry edits `nvs.toml`, grants itself `process.exec`, and a service
runs its program as `LocalSystem`. `nvs service install` and `nvs config check --as` report the
tree as trusted. `rule:config/ownership-is-the-trust-boundary` says "writable by nobody else", and
the code checks a list.

**The debug viewer on loopback has no credential.** The spool is private on disk and checked
(`crates/nvs-config/src/data.rs:293`). The viewer serves the same records on `127.0.0.1:8137`, and
on a loopback bind `secure` is `None` (`crates/nvs-cli/src/debug/listen.rs:130`). The token is
checked only when `secure` is set (`crates/nvs-cli/src/debug/listen.rs:306`). The `Host` and
`Origin` checks stop a browser page, and they do not stop a process of another local account, which
sends any header it likes. That account reads every recorded request, body and header. With an
`Origin` header and a JSON body it also reaches the routes that discard the index, replay a request
and change the settings.

## Standing decisions

**The secret table is the mark, and the merged table never holds a secret value.**
`nvs_config::secret::SECRETS` is the one registry of secret directives, and `is_secret_key` is its
one reader. `directive::DIRECTIVES` gets no second flag, because one fact in two tables drifts.
`materialize` carries an inline value into `Materialized::secrets` as it carries a file's, and
`strip_inline` removes it from the table by walking `SECRETS`. `apply` already puts each value back
on the typed tree, and `Request::get` already reads `secrets` first. A `Secret` records whether a
file or the tree wrote it, so nothing reads a configuration file again as a secret file. Every
printer of the table then has no value to print, and a printer that renders by key calls
`is_secret_key`. **What it spends:** nothing per request, and one map entry per inline secret at
boot and at reload. Declined: redaction inside each printer alone, because the next printer forgets
it. Safe default if a reader needs the inline value in the table: that reader moves to the `secrets`
map, and the table stays clean.

**`dump --toml` writes an inline secret's key with the value `"<secret>"`.** The diff of two
environments then shows that the key is set. A file-backed secret keeps its `_file` row, as now.
The rule says a dumped document's secrets are filled in before it is served. Declined: leaving the
key out, because a diff would then show a set password as absent.

**The printers of configuration values are a list, and a test drives each.** Known today:
`Audit::listing` and `Audit::render` for `nvs config dump`, `dump --origin` and the count `nvs
config check` prints (`crates/nvs-config/src/audit.rs:85`, `crates/nvs-config/src/audit.rs:109`,
`crates/nvs-cli/src/config.rs:1474`), `dump --toml` (`crates/nvs-cli/src/config.rs:1726`),
`Request::all` behind `Core\Config::all` (`crates/nvs-config/src/request.rs:584`), and the text of a
configuration diagnostic such as `both_set` (`crates/nvs-config/src/secret.rs:646`). An `Explore`
subagent lists the others before the first edit: the reload record, the debug viewer, telemetry and
any error text that quotes a value. `no_printer_shows_a_secret_directive` loops over `SECRETS` and
calls every listed printer.

**A key named like a credential is a secret or is listed with a reason.** The test reads every field
the typed tree declares, through the census `tree_structs` already builds
(`crates/nvs-config/tests/secret.rs:651`). A name that ends in `password`, `key`, `secret` or
`token`, or is a `_file` sibling of one, needs a `SECRETS` row or a row of a `NOT_A_SECRET` list
with its reason, such as "a path to a key file". A list row that names no field fails too. This
replaces the hand-written `CREDENTIAL_FIELDS` (`crates/nvs-config/tests/secret.rs:635`).

**The Windows check reads the whole access list, by the rule Unix already has.** One walk of the
DACL replaces the loop over five groups. A breach is an allow entry that is not inherit-only, holds
a refused right after the deny entries ahead of it for the same trustee, and names an untrusted
trustee. Trusted are the file's owner, the account the check judges for, `SYSTEM`, `Administrators`,
`TrustedInstaller`, and a member of `Administrators` by the lookup the owner test uses
(`crates/nvs-config/src/trust.rs:1301`). The judged account is on the list because a service's data
folder grants it write. A member of `Administrators` is on it because that account can take the
file anyway. An allow entry of a kind the walk does not know counts as an allow, so the check fails
closed. A lookup that fails is not a member. The breach names the trustee's SID, so the existing
`/remove:g <SID>` help and the `nvs init` repair still apply. **What it spends:** at boot and at
reload, one walk per checked path where five ran, and one group lookup per distinct unknown trustee
that holds a refused right. Nothing per request. Usability: a tree that grants a deploy user or a
custom group write now fails with `E0607`, which is what Unix does to a group-writable file. A data
folder granted to a service account fails for any other account that is not its owner. Declined:
a longer list of untrusted groups, because a list is what left the gap. Declined: a key that names
extra trusted accounts, because it would sit in the file the check guards.

**The debug viewer needs its token on every bind.** The token is the one the off-loopback bind
already uses: `[debug] token_file` where it is set, else a file generated once in the private
`debug/` folder and read at each start (`crates/nvs-cli/src/debug/secure.rs:48`). So an open tab
survives a restart, and whoever can read the token can already read the spool. `nvs debug` prints
the link with `?token=`, the first request sets a cookie and redirects, and opening the viewer stays
one click. On a plain loopback bind the cookie has its own name, with `HttpOnly`, `SameSite=Strict`
and `Path=/`, and without `Secure` and the `__Host-` prefix, which need TLS. TLS stays off loopback
only. No new setting. **What it spends:** one constant-time comparison per viewer request, and
nothing on a request path. Usability: a bookmark of the bare address answers `401` with the
sentence that names the printed link. Declined: a peer-credential check, because TCP has none on
every platform. Declined: a token made new at each start, because it signs an open tab out at each
restart of a development server. Known bound: a browser sends a loopback cookie to every port of
that address, so the cookie is `SameSite=Strict` and `HttpOnly`. The loopback cookie is named
`nvs-debug-<port>`, so two services on one address, each with its own data folder and token, do
not replace each other's cookie.

**Lanes:** stages 1, 2 and 3 touch disjoint files and may run side by side. Stage 4 runs last.

## Stages

### 1. a dump redacts every secret directive

**Does:** move every inline secret directive out of the merged table, hold every printer to the
secret table, and hold every credential-named key to it.

**Split:** before the first edit, an `Explore` subagent lists every site outside
`crates/nvs-config/src/audit.rs` and `crates/nvs-config/src/request.rs` that prints, logs or sends
a configuration value, as `file.rs:NN` and one line each. It looks in `crates/nvs-config/src`,
`crates/nvs-cli/src` (the reload record, `debug/`, `telemetry.rs`), `crates/nvs-server/src` and
`crates/nvs-stdlib/src/config.rs`.

- [x] `Secret` records its source, a file or the configuration tree
  (`rule:config/a-secret-is-a-file-whose-content-is-the-value`,
  `crates/nvs-config/src/secret.rs:82`). Every reader of `Secret::file` follows, the dump's origin
  column and the secret file's integrity check among them.
- [x] `materialize` inserts a `Secret` for the inline half
  (`crates/nvs-config/src/secret.rs:405`).
- [x] `strip_inline` removes each inline secret directive from the table by walking `SECRETS`, and
  keeps the typed tree's value for `apply` (`crates/nvs-config/src/secret.rs:498`,
  `crates/nvs-config/src/secret.rs:538`, `crates/nvs-config/src/resolve.rs:611`).
- [x] `Audit::listing` and `Audit::render` name the configuration file as the origin of an inline
  secret (`rule:config/check-and-dump-audit-the-tree-offline`,
  `crates/nvs-config/src/audit.rs:85`, `crates/nvs-config/src/audit.rs:109`).
- [x] `dump --toml` writes `"<secret>"` under the key of each inline secret
  (`crates/nvs-cli/src/config.rs:1726`).
- [x] Each printer the `Explore` list names prints `<secret>` for a key `is_secret_key` accepts, or
  prints no value (`rule:security/redaction-reaches-the-tools-own-renderings`,
  `crates/nvs-config/src/secret.rs:328`).
- [x] The module doc of `secret.rs` says what is true now (`crates/nvs-config/src/secret.rs:1`).
- [x] Test `no_printer_shows_a_secret_directive`: for each row of `SECRETS`, one tree with an inline
  value and one with a file value. Neither value appears in `render(false)`, `render(true)`, the
  `--toml` text, `Request::all`, the `both_set` diagnostic, or any printer the list names
  (`crates/nvs-config/tests/secret.rs:1`).
- [x] Test `every_key_named_like_a_credential_is_a_secret_or_listed_with_a_reason`, with the
  `NOT_A_SECRET` list, replacing `CREDENTIAL_FIELDS` and keeping the `_file` sibling assertion
  (`crates/nvs-config/tests/secret.rs:635`, `crates/nvs-config/tests/secret.rs:684`).
- [x] Test `dump_prints_no_inline_secret_directive`: the built `nvs` runs `config dump`, `dump
  --origin` and `dump --toml` on a tree with an inline `[db.main] password`, `[http] csrf_key` and
  `[cache.shared] password` (`crates/nvs-cli/tests/config_audit.rs:1`).
- [x] `rule:config/a-secret-is-a-file-whose-content-is-the-value` says an inline value leaves the
  table as a file's does (`docs/rules/config/a-secret-is-a-file-whose-content-is-the-value.md:1`).
- [x] `rule:config/check-and-dump-audit-the-tree-offline` says what `--toml` writes for an inline
  secret (`docs/rules/config/check-and-dump-audit-the-tree-offline.md:28`).

### 2. the Windows ownership check reads the whole access list

**Does:** replace the loop over five groups with one walk of the DACL that refuses every untrusted
trustee, and state one rule for both platforms.

- [x] `examine` walks the DACL once and returns the first untrusted trustee with a refused right,
  for the file test, the parent test and the link-holder test alike
  (`rule:config/ownership-is-the-trust-boundary`, `crates/nvs-config/src/trust.rs:1067`,
  `crates/nvs-config/src/trust.rs:1108`).
- [x] The trusted trustees are the standing decision's list, with `in_administrators` asked once per
  distinct trustee (`crates/nvs-config/src/trust.rs:1042`, `crates/nvs-config/src/trust.rs:1301`).
- [x] An allow entry of an unknown kind counts as an allow (`crates/nvs-config/src/trust.rs:1123`).
- [x] `UNTRUSTED` goes, and the breach prints the trustee's name where Windows gives one and its
  SID always (`crates/nvs-config/src/trust.rs:772`, `crates/nvs-config/src/trust.rs:1075`).
- [x] Test `a_write_entry_for_any_other_account_is_a_breach`. On Windows it grants Modify, by SID
  through the file's `icacls` helper, to `INTERACTIVE`, `NETWORK`, `BATCH` and `SERVICE` in turn, on
  the file and on its directory, and expects `E0607` naming that SID each time. On Unix it sets the
  group bit and the world bit in turn (`crates/nvs-config/tests/trust.rs:287`,
  `crates/nvs-config/tests/trust.rs:82`).
- [x] Test `a_write_entry_for_a_trusted_account_passes`: on Windows, full control for the owner,
  `SYSTEM` and `Administrators` passes. On Unix, an owner-writable file passes
  (`crates/nvs-config/tests/trust.rs:1`).
- [x] The existing Windows cases follow, the deny case and the parent's replace test among them
  (`crates/nvs-config/tests/trust.rs:194`, `crates/nvs-config/tests/trust.rs:309`,
  `crates/nvs-config/tests/trust.rs:331`, `crates/nvs-config/tests/trust.rs:450`).
- [x] `rule:config/ownership-is-the-trust-boundary` states the Windows test as "no other account",
  lists the trusted trustees, and drops the five names from the parent paragraph
  (`docs/rules/config/ownership-is-the-trust-boundary.md:10`,
  `docs/rules/config/ownership-is-the-trust-boundary.md:38`).

### 3. the debug viewer answers only a request that carries its token

**Does:** load the token on every bind, check it before any route, and print the link that carries
it.

**Split:** an `Explore` subagent lists every page under `website/src/content/docs` and every help
text that names the viewer's address, as `path:NN`.

- [x] `Secure` splits into the token, loaded on every bind, and the TLS configuration, loaded off
  loopback (`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`,
  `crates/nvs-cli/src/debug/secure.rs:25`, `crates/nvs-cli/src/debug/listen.rs:130`).
- [x] `refused` checks the query token and the cookie on every bind
  (`crates/nvs-cli/src/debug/listen.rs:306`).
- [x] The loopback cookie has its own name and no `Secure` attribute, and `admitted` and
  `cookie_token` take the bind's cookie (`crates/nvs-cli/src/debug/listen.rs:77`,
  `crates/nvs-cli/src/debug/listen.rs:456`, `crates/nvs-cli/src/debug/listen.rs:466`).
- [x] The loopback link carries `?token=`, and `nvs debug` prints it
  (`crates/nvs-cli/src/debug/listen.rs:135`, `crates/nvs-cli/src/debug.rs:227`).
- [x] The module doc and the help of `nvs debug` and `--listen` say the link is printed
  (`crates/nvs-cli/src/debug/listen.rs:13`, `crates/nvs-cli/src/debug.rs:105`,
  `crates/nvs-cli/src/debug.rs:128`).
- [x] Test `on_loopback_every_path_needs_the_token`: the page, each API path the router matches, and
  a path it does not match answer `401` with no token and with a wrong one. A changing request with
  the own `Origin` and a JSON body answers `401` too (`crates/nvs-cli/tests/debug_viewer.rs:1`,
  `crates/nvs-cli/src/debug/listen.rs:325`).
- [x] Test `the_printed_loopback_link_opens_the_viewer`: the printed link answers a redirect that
  sets the cookie, and the cookie alone then opens the page and one API path.
- [x] The existing loopback cases send the token (`crates/nvs-cli/tests/debug_viewer.rs:181`,
  `crates/nvs-cli/tests/debug_viewer.rs:210`, `crates/nvs-cli/tests/debug_viewer.rs:226`,
  `crates/nvs-cli/tests/debug_viewer.rs:263`), and so does `crates/nvs-cli/tests/debug_service.rs:47`.
- [x] `rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact` says every bind needs
  the token and what it costs
  (`docs/rules/testing/the-viewer-shows-the-stream-and-hands-off-every-artifact.md:64`).
- [x] Each page and help text the `Explore` list names tells the reader to open the printed link.

### 4. the goal is verified once, and every failure is fixed

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done, and the goal is reached. `looper verify` ran three times. The third run is
green. The four acceptance checks pass by name, and `bun nv verify --doc`, `bun nv owners --closes
operator-surfaces` and `bun nv ci --check` are green.

The gate fixed three things:

- A Windows breach names a well-known group in English (`trust::platform::WELL_KNOWN`), because
  this host's German Windows printed `VORDEFINIERT\Benutzer` and the `init` tests pin the English
  name. The rule says so.
- `crates/nvs-cli/tests/debug_api.rs` sends the loopback cookie, because without it every request
  answered `401` and the event stream waited out its time limit.
- `crates/nvs-cli/tests/init.rs`'s Windows install test now expects a cache granted to the service
  account to go unused by another account's run, as the stage 2 decision says, and then used once
  the grant is removed. `docs/reference/tools/15-install.md` tells the reader.

The website's debug viewer page split its token paragraph under the prose bound, three pages' questions
ask about the token, and `llms-full.txt` is rendered again.

The sweep items were stale: no `env-secret-proofs` check exists or has a log, and Miri run
`43201099245` ran on `578534cc7c`, before `7568aaa0fb` skipped those tests under Miri.

### Next

None: the goal is reached, and the driver retires it.

### Backlog

- A header token kept by the viewer's page would keep the token away from other loopback ports.
