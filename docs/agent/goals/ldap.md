---
milestone: M8
position: last
---
# Loop goal 198 — `Core\Ldap`: a native LDAPv3 client over LDAPS and StartTLS, with a filter builder and typed Active Directory values

ADR 0051 § 3 placed `ldap` as *Native but unscheduled*: Native by test 1, because a bound directory
session is state across calls, and with its injection answer in `Core` by test 2. This goal schedules
it. It builds `Core\Ldap`, a general LDAPv3 client tested against Active Directory, the only directory
most applications meet. The wire code is our own, over `nvs-host`'s sockets and `NvsTls`.

```nvs
$dir = Ldap::connect('corp');                       // names are the record's; this is a sketch

$people = $dir->search(
    Ldap\Filter::all(
        Ldap\Filter::equals('objectClass', 'user'),
        Ldap\Filter::startsWith('sAMAccountName', $prefix),   // tainted input is fine: values are never text
        Ldap\Ad::memberOf($staff, {nested: true}),
        Ldap\Ad::enabled(),
    ),
    {in: 'OU=Staff', select: ['displayName', 'mail', 'objectGUID', 'lastLogonTimestamp']},
);
foreach ($people as $p) {                           // pages are fetched as the loop runs
    $p->uuid('objectGUID');                         // Core\Uuid, AD's mixed-endian bytes put right
    $p->instant('lastLogonTimestamp');              // FILETIME; AD's "never" is null
    $p->accountFlags()->disabled;
}
$dir->authenticate($login, $password);              // throws Ldap\LdapError with a kind
```

Its scope is the list in § *Standing decisions* and nothing more: simple bind over LDAPS or StartTLS,
or over plain `ldap://` where the operator grants that host, paged search, the filter builder and `Dn`, typed values from the schema, writes and passwords,
`authenticate`, and the AD extras the user named. Kerberos and NTLM are not in it and never will be.

## Why here

It shares no files with the M9 goals in front of it. Those open `extensions/`, `wit/` and `nvs-ext`;
this one opens a new `crates/nvs-ldap`, `nvs-stdlib`'s registry, `nvs-config` and `tests/db/compose.yaml`.
It stands on finished work only: `NvsTls` and its StartTLS-style upgrade (`Core\Mail` and TDS use it),
`Core\Db`'s pool and its reset boundary, the capability table, the outbound address policy and
`Core\Uuid`. Nothing in front of it waits for it, so it goes at the end of the chain. The user put it
there.

It carries `position: last` because every goal on the chain is pinned there.

## Stage 0 — the catch-up

No sentence on disk becomes untrue before Stage 2's record lands. ADR 0051 § 3's "Native but
unscheduled" is frozen history and stays. The search that closes the stage, run after the record:
`grep -rn -i "ldap" docs/rules docs/reference crates --include=*.md --include=*.rs`, read line by line.
Every hit is true as it stands, or rewritten. `docs/ground-rules.md`, the `docs/rules/*.md` chapters and
`website/` are generated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check goals `ext-image`,
`ext-image-analysis` and `ext-intl` turned green stays green, and
`tokio_appears_in_neither_the_manifest_nor_the_lockfile` (`crates/nvs-runtime/tests/manifest_policy.rs:148`)
stays green with the new crate in the graph.

## Stage 2 — the record

**Does:** Writes the `Core\Ldap` record and the rule fragments it creates, from § *Standing decisions*.

One file set: `docs/decisions/` (the record), `docs/rules/core-classes/` (its fragments),
`docs/rules/core-api/tier-roster.md` (read only).

- **The record**, this goal's one slot, written first. It names every class, member and options shape
  under `Core\Ldap` (`rule:core-api/one-paradigm-per-operation`, `rule:core-api/options-bag`), the
  config block and the capability, the type table, the error kinds, the pool and its reset, and the
  tradeoffs. It records the user's six calls as decided, and Kerberos and NTLM as declined.
- **The filter builder is `Core`, and the record says why** against
  `rule:core-classes/db-one-api`'s "Query builders … are not `Core` at all". An LDAP filter on the wire
  is a BER structure, not text. A builder that encodes straight to BER is the protocol's own shape, and
  it is the only form in which a filter carries `tainted` values safely, as `Core\Db`'s bound parameters
  do. So no filter escaper exists. The record does not edit `db-one-api`, which is about SQL.
- **Fragments**, at least these six, under these ids, which the Stage 2 check reads:
  `core-classes/ldap-filter-is-a-value` (the builder and its sink split),
  `core-classes/ldap-dn-is-the-launderer` (`rule:security/launderers-are-sink-named`),
  `security/ldap-empty-password-is-refused`, `security/ldap-cleartext-bind-is-granted-per-host`,
  `core-classes/ldap-value-types` (the type table) and `security/ldap-pool-is-bound-as-its-block` (the
  pool and `authenticate`'s own connection). Each
  fragment's `because` names the record. `bun nv brief --where` prints rules in chapter order, and
  `security` comes before `core-classes`, so the check's `want` lists the three `security` ids first.
- Its last item adds, to this goal's Stage 10 in `data/goals/ldap.json`, one `bun nv proofs --verify
  --group '<class>'` check per class it names, and every class's `Ldap-*` directories to the
  `--comments` check. A check is added, never removed.
- **Pinned by** the Stage 2 check.

## Stage 3 — the wire, and a directory to test against

**Does:** Builds `crates/nvs-ldap`: LDAPv3 messages over LDAPS and StartTLS, simple bind, unbind, search
with the paged-results control, and result codes as typed errors. Adds a Samba AD domain controller to
the test servers.

Two file sets, in this order. The test server: `tests/db/compose.yaml`, `.github/workflows/ci.yml`,
`tools/nv/cmd/db-matrix.ts`. The crate: `crates/nvs-ldap/` (new, made by hand, never `cargo new`),
`Cargo.toml`, `deny.toml`.

- **The test server.** A `samba-ad` service in `tests/db/compose.yaml`, image `smblds/smblds` pinned by
  digest. It ships Samba ≥ 4.21, which checks channel bindings. It gets a fixed `hostname`, and the
  `certs` volume's leaf with that name in its SAN (Samba's own certificate names the random container
  id), on loopback ports 16389 and 16636 with a health check. It needs no `--privileged`. CI gets an
  `ldap` step beside the database matrix. The same slice adds `env.docker` naming `samba-ad` to
  `data/goals/ldap.json`, so the driver brings the server up for every later session. The record has no
  `env.docker` until then, because the driver's preflight fails on a service the compose file does not
  have yet.
- **A directory test skips only where nothing listens on 16636**, and says so on stderr, as the Redis
  cases do (`crates/nvs-stdlib/src/cache/redis.rs:1286`).
- **The codec is borrowed, the state machine is written** (`rule:core-classes/db-crate-boundary`). A
  sans-IO BER/LDAP crate encodes and decodes. `rasn-ldap` is the first candidate, and the session checks
  that it pulls neither tokio nor an async runtime before it adopts it. `ldap3` is out because it runs
  on tokio (`rule:concurrency/one-scheduler`). The connection runs over `nvs_host::net::NvsTcp` and
  `NvsTls` (`crates/nvs-host/src/tls.rs:224` `over`), with StartTLS as the extended operation followed
  by the upgrade the way `Core\Mail` does it.
- **A simple bind with an empty password is refused before a byte is sent** (RFC 4513 § 5.1.2). AD
  answers it as a successful anonymous bind. Samba answers it `invalidCredentials`, so the test asserts
  the refusal happens in the client, not at the server.
- **A simple bind on a connection that is not encrypted needs two things**, and the client checks both
  before the password is sent. The block or `Settings` says `tls = "none"`, and the host is on the
  operator's `[capabilities.ldap] cleartext` list. That list is a host list like `tls.insecure` and
  `net.downgrade`, and `true` is not a spelling it has. Without both, plain `ldap://` only runs
  StartTLS. A controller that refuses the bind (`strongerAuthRequired`, 8) throws the kind
  `EncryptionRequired`. Password writes stay encrypted-only (Stage 7).
- **The test server allows a cleartext bind** (`ldap server require strong auth = no`, the image's
  `INSECURE_LDAP`), so the granted path is tested end to end. The `EncryptionRequired` mapping is a unit
  test over a recorded response.
- **Paging** sends the paged-results control on every search and follows the cookie. A search returns
  pages lazily and never holds more than one page.
- **Referrals and continuation references** are returned as data, never followed.
- **Pinned by** the Stage 3 checks, run against the `samba-ad` service.

## Stage 4 — `Core\Ldap`: connect, open, search, read

**Does:** Registers `Core\Ldap` with `connect` (a config block) and `open` (`Ldap\Settings`), the
`[ldap.<name>]` block and the `ldap.connect` and `ldap.open` grants, the pool, `search` and `read`
returning entries with raw readers, and `Ldap\LdapError`.

One file set: `crates/nvs-stdlib/src/ldap/` (new), `crates/nvs-stdlib/src/registry.rs` (the `CLASSES`
and `CAPABILITIES` rows), `crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/capability.rs`,
`crates/nvs-config/src/secret.rs`, `crates/nvs-runtime/src/throwable.rs`, `crates/nvs-hir/src/errors.rs`,
`tests/conformance/core/`.

- **Config** follows `[db.<name>]`: `url` (a list, tried in order), `base`, `user`, `password` or
  `password_file`, `tls` (`"required"` by default, or `"none"` under the cleartext grant), `tls_ca_file`,
  `timeout`, `pool`. Every struct is `deny_unknown_fields`. The
  password is a config secret (`rule:config/a-secret-is-a-file-whose-content-is-the-value`).
- **Grants** follow `rule:core-classes/db-capabilities`: `ldap.connect` by block name, pre-approved
  against the address policy, and `ldap.open` for a program-supplied host. That host refuses `tainted`
  and passes `rule:security/net-address-policy`. `ldap.cleartext` is a host list (Stage 3) that both
  `connect` and `open` are checked against.
- **The pool** follows `rule:security/db-pool-reset-is-a-boundary`: per core, keyed by every credential
  and the config generation. A pooled connection is always bound as the block's own identity.
- **Entries**: `dn()`, `has`, `string`, `strings`, `bytes`, and `toArray`. An attribute name is matched
  without case and keeps the server's case. Nothing is lowercased, and there is no `count` key.
- **A size or time limit the server hit throws.** A partial result is never returned as if it were
  complete.
- **Pinned by** the Stage 4 checks.

## Stage 5 — the filter builder and `Dn`

**Does:** Adds `Ldap\Filter` and `Ldap\Dn`, and the AD matching rules under `Ldap\Ad`.

One file set: `crates/nvs-stdlib/src/ldap/filter.rs`, `crates/nvs-stdlib/src/ldap/dn.rs` (new),
`crates/nvs-ldap/src/` (the filter's BER encoding), `tests/conformance/core/`.

- **Filters are immutable values** made by static functions: `equals`, `startsWith`, `endsWith`,
  `contains`, `present`, `atLeast`, `atMost`, `approx`, and `all`, `any`, `not`. There is no `<` or `>`,
  because RFC 4515 has none. A value may be `tainted`. An attribute name is a sink and is checked
  against the attribute-description grammar.
- **`Filter::parse`** takes RFC 4515 text. Its parameter refuses `tainted`
  (`rule:security/unclassified-parameter-refuses-tainted`), so input never reaches it.
- **`Ldap\Ad`**: `memberOf(dn, {nested})` (nested is `1.2.840.113556.1.4.1941`), `enabled`,
  `disabled`, `bitAnd`, `bitOr` (`.803` and `.804`), and a filter on a `Core\Uuid` or a `Sid` that
  encodes its binary form.
- **`Dn`** is built from parts and escapes each value (RFC 4514). It is the one launderer for the DN
  sink, and every member that takes a DN takes a `Dn` or a string that refuses `tainted`.
- **Injection is tested as an attack.** A value of `*)(objectClass=*` and one with a NUL byte match only
  themselves.
- **Pinned by** the Stage 5 checks.

## Stage 6 — typed values

**Does:** Reads the schema once per pool, types each attribute from it plus a fixed table of names, and
merges ranged attributes.

One file set: `crates/nvs-stdlib/src/ldap/schema.rs`, `crates/nvs-stdlib/src/ldap/value.rs`,
`crates/nvs-stdlib/src/ldap/sid.rs`, `crates/nvs-stdlib/src/ldap/ad.rs` (new),
`crates/nvs-stdlib/src/uuid.rs` (read only), `crates/nvs-stdlib/src/time.rs` (read only),
`tests/conformance/core/`.

- **The rule is `rule:core-classes/db-column-types`'s**: each attribute has one natural type, a typed
  reader converts only where nothing is lost, and anything else throws. Text and bytes come back
  `tainted`.
- **The schema** gives the base type: AD's `attributeSyntax`/`oMSyntax`, or the standard subschema's
  SYNTAX OID on a server that is not AD. It is read lazily, once per pool, and is immutable.
- **The fixed table** gives what the schema cannot say. GUIDs (`objectGUID` and the rest) are
  `Core\Uuid`, with the first three groups byte-swapped. `objectSid`, `tokenGroups` and `sIDHistory` are
  a new `Ldap\Sid` (`S-1-5-21-…`). FILETIME attributes are `Instant`, where `0` and `0x7FFFFFFFFFFFFFFF`
  are `null`, and `pwdLastSet = 0` is readable as "must change". Negative intervals (`maxPwdAge`,
  `lockoutDuration`) are `Duration`. GeneralizedTime is `Instant`. `TRUE`/`FALSE` is `bool`.
- **Flag fields are readonly objects with one `bool` per flag and an immutable `with`**:
  `userAccountControl` and `msDS-User-Account-Control-Computed`, `groupType`, and `sAMAccountType` as
  an enum. Bits the object does not name are kept, so a write never drops them. Lockout and an expired
  password are read from the computed attribute, because AD does not keep them in `userAccountControl`.
- **Ranged attributes** (`member;range=0-1499`) are fetched to the end and merged under the plain
  name.
- **Pinned by** the Stage 6 checks.

## Stage 7 — writes and passwords

**Does:** Adds `add`, `modify` with an ordered list of `Ldap\Change`, `delete`, `rename`, `compare`,
`whoami`, `setPassword` and `changePassword`.

One file set: `crates/nvs-stdlib/src/ldap/write.rs` (new), `crates/nvs-ldap/src/`,
`tests/conformance/core/`.

- **`modify` is one atomic request**: `Change::add`, `remove`, `removeAll`, `replace`, in order. A typed
  value is written in the form it was read, so a `Uuid`, a `Sid`, an `Instant` and a flag object
  round-trip.
- **Passwords** are AD's `unicodePwd`: the quoted string, UTF-16LE. A reset is `replace` and a change is
  `remove` old plus `add` new. Both take `secret` strings (`rule:security/secret-qualifier`) and run
  only on an encrypted connection. AD's policy refusal is the error kind `PasswordPolicy`. RFC 3062's
  Password Modify is not used, because AD does not have it.
- **`compare`** returns `bool` and throws on error.
- **Pinned by** the Stage 7 checks.

## Stage 8 — `authenticate` and AD's reasons

**Does:** Adds `authenticate(login, secret password)` and maps AD's bind sub-codes to error kinds.

One file set: `crates/nvs-stdlib/src/ldap/auth.rs` (new), `crates/nvs-stdlib/src/ldap/error.rs`,
`tests/conformance/core/`, `tests/hostile/core/`.

- **It binds on its own connection**, never a pooled one, and closes it. A pooled session never
  changes identity.
- **The login forms** are a DN, `user@upn-suffix` and `DOMAIN\user`. An empty password is refused
  (Stage 3).
- **Kinds** from the `data <hex>` in AD's diagnostic: `AccountDisabled` (533), `AccountLocked` (775),
  `PasswordExpired` (532), `MustChangePassword` (773), `AccountExpired` (701), `NotAllowedNow` (530,
  531). **525 (no such user) and 52e (wrong password) are both `InvalidCredentials`**, so a program
  cannot tell an attacker which accounts exist. The message never carries the password.
- **Pinned by** the Stage 8 checks.

## Stage 9 — the AD extras

**Does:** Adds server-side sort and VLV, deleted objects, incremental sync and the Global Catalog, each
as a named search option or member and never a raw control.

One file set: `crates/nvs-stdlib/src/ldap/search.rs`, `crates/nvs-stdlib/src/ldap/sync.rs` (new),
`crates/nvs-ldap/src/` (the controls), `tests/conformance/core/`.

- **Sort and VLV**: a `sort` option (`.473`) and a window (`2.16.840.1.113730.3.4.9`) for "page n of m"
  lists.
- **Deleted objects**: a `showDeleted` option (`.417`).
- **Incremental sync**: DirSync (`.841`) returns the changed entries and an opaque cookie the program
  stores. The record decides whether `uSNChanged` polling pinned to one DC's `invocationId` is a second
  form or is left out (`rule:core-api/one-paradigm-per-operation`).
- **The Global Catalog** is a block whose URL uses port 3268 or 3269. It is read-only, and a write to it
  throws before it is sent.
- **Samba's gaps** are known and named in the tests' doc comments. Its nested-group rule returns nothing
  when an object in the chain is unreadable (Samba bug 15515), and it lacks `.2253`.
- **Pinned by** the Stage 9 checks.

## Stage 10 — the feature proofs

**Does:** Writes the feature proofs for every `Core\Ldap` member the record named.

One file set: `docs/examples/core/Ldap*/`, `tests/hostile/core/Ldap*/`, `benches/members/core/Ldap*/`
(new, one directory per class as `Db` and `Db-Connection` are laid out), `data/proofs/policy.json`.

- **What each member owes** (`rule:testing/feature-proofs`): `about.md`, tests from Novis and Rust,
  three examples, one bench with its growth, one attack and its help in the binary. `bun nv proofs --id
  '<member>'` prints what is owed.
- **The attacks**: filter injection through every builder function, a DN injection through `Dn`, an
  empty password, a bind over plain `ldap://` to a host the cleartext grant does not name, a server that answers a search with a million entries
  under a small memory cap, a `member` range that never ends, a server that never answers, and a
  continuation reference to another host.
- **Pinned by** the Stage 10 checks, and by the per-class checks Stage 2's record added.

## Standing decisions

- **The user's calls, as instructions.**
  - Simple bind only. Kerberos, NTLM and every SASL mechanism are declined for good. The record says
    so, and no session re-proposes them.
  - A simple bind over LDAPS or StartTLS by default. A bind over plain `ldap://` is allowed only where
    the block or `Settings` says `tls = "none"` and the host is on `[capabilities.ldap] cleartext`, a
    host list in the shape of `tls.insecure` and `net.downgrade`. Some controllers accept it, and the
    operator decides per host. The record states the cost: the password and every answer cross the
    network readable and changeable.
  - The filter builder is immutable filter values combined with `all`, `any` and `not`. Base,
    attributes and paging are search options. There is one spelling, and there is no fluent chain.
  - AD's flag fields are readonly objects with one `bool` per flag. No general flag-set type is added
    to `Core`.
  - Both `Ldap::connect('<block>')` and `Ldap::open(Settings)`, the same as `Core\Db`.
  - A general LDAPv3 client with AD helpers under `Ldap\Ad`, tested against Samba AD only.
  - All four AD extras are in: sort with VLV, deleted objects, incremental sync and the Global Catalog.
- **The goal writer's calls, not confirmed by the user, also standing.**
  - The namespace is `Core\Ldap`, Native like `Core\Db`, with no Cargo feature
    (`rule:core-classes/db-crate-boundary`). `ldap.connect` and `ldap.open` are deny-by-default.
  - The wire crate is `crates/nvs-ldap`, and the Novis-facing half is `crates/nvs-stdlib/src/ldap/`.
  - `authenticate` binds on a fresh connection and closes it.
  - The schema is cached per pool, immutable, and read lazily.
  - 525 and 52e are one kind.
  - Referrals are never followed.
  - No raw-control member and no raw-exop member exist. A control is a named option.
  - There are no TLS relaxations beyond what `NvsTls` and `[capabilities.tls]` already allow per host.
  - The URL list fails over in order. The first server that answers the TLS handshake and the bind is
    used.
- **What we take from PHP's ext/ldap, and what we do not.** We take: the atomic multi-change modify
  (`ldap_modify_batch`), `whoami`, a URL list for failover, and controls attached to the operation they
  change. We do not take: process-wide TLS options (php-src issue #17776 leaked one request's "do not
  verify" into the next), any "do not verify" switch, an empty password as an anonymous bind, referral
  chasing and `ldap_set_rebind_proc`, unlimited timeouts, `false` plus `ldap_errno`, the `count`-keyed
  array with lowercased keys, `ldap_escape`, `ldap_sort`, T.61, the Oracle wallet, the `_ext` twins, and
  reading `ldap.conf` or `LDAPTLS_*` from the environment.
- **One record slot**: one new record and no other number. Its number is checked against
  `docs/decisions/` right before it is written, because another agent may take a number first. It is
  Stage 2.
- **The test server is Samba, and real AD is a person's check.** Samba 4.23 in `smblds/smblds` was
  run by hand when the goal was written. It was healthy in about 27 s without `--privileged`. Over
  LDAPS it returned `objectGUID` and `objectSid` as binary, `pwdLastSet` as FILETIME and `whenCreated`
  as GeneralizedTime. It answered an empty-password bind with `data 52e`, a simple bind on 389 with
  `strongerAuthRequired`, and evaluated the nested-group rule. A run against a Windows Server
  evaluation VM is the user's to do by hand, and a gap it finds is a backlog item.
- **The tradeoffs**, stated here and in the record because AGENTS.md asks.
  - Performance: one round trip per page of up to 1000 entries. The filter is encoded once, with no
    text step. `authenticate` pays one TCP and TLS handshake per login.
  - Memory: one page and the current entry per search, per request. The schema cache is per pool, so
    it is O(endpoints) and not O(requests). A merged `member` list is the whole list, under the
    request's memory cap.
  - Usability: AD values arrive as `Uuid`, `Sid`, `Instant`, `Duration` and flag objects. Filters
    cannot be injected. The login reasons are named.
  - Simplicity: one new Native subsystem, a filter builder in `Core` with a recorded reason, and no
    second spelling of anything.
- **Neutral names only** in every test, example and record: `example.test`, `Shop`, `Blog`, `Staff`.
  A config block in an example is named for the directory it reaches, such as `[ldap.corp]`, never for
  a judgement such as `legacy` or `old`. Where a block runs without TLS, a comment says so in plain
  words.
- **Every comment in a new `.nvs` and every `about.md` this goal writes follows `AGENTS.md`
  § *Text an end user reads* at the first write**, and `bun nv proofs --comments <paths>` is run over
  them before the wrap.
- **A debug cargo command never takes `-p`.** Narrow what runs with `bun nv verify -p nvs-ldap` or a
  `--test` filter.
