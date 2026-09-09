---
milestone: M8
---
# Loop goal 22 — a configured store is authorized by its configuring, and may be a Unix socket

`rule:config/cache-shared-is-the-grant-over-the-configured-store`, whole. Two things
that are one thing: the grant over a store an operator wrote stops naming a host, and once it does the
store may be reached over a transport that has no host to name.

Today `Core\Cache::shared()` asks `net.connect` at a host scope and then walks
`rule:security/net-address-policy`'s denied-range table, so the ordinary
deployment — a Redis on loopback — has to write `net.connect` *and* `net.internal` for an endpoint only
the operator ever named. That is the outcome § 3's own carve-out exists to prevent, missed because the
carve-out named `[db.<name>]` and `[cache.shared]` was written after it. Fixing the question also
removes the obstacle to a Unix socket: `nvs_runtime::capability::pin_host` needs a host to ask about and
an address to pin, and a socket path has neither, so nothing outbound could reach the `NvsUnix` that
`crates/nvs-host/src/net.rs` has carried since goal `concurrency`.

The transport is not the work. `NvsUnix` exists, parks on the same reactor, and
`rule:http-server/the-server-block-is-boot-class` already admits a socket path on
the listening side. What this goal writes is the authority answer, the two spellings, one refusal, and
the `AF_UNIX` connect path in three database drivers.

**This goal is placed before the dossier deliberately.** Goal `dossier` turns the chain around — it emits the
dossier's generated goals and stops adding surface — so an entry that adds surface belongs in front of
it. Its floor is goal `parses`'s whole list, which is the parity program entire.

## Stage 0 — the catch-up

Two places state the rule this goal replaces, both written against `net.connect`:

1. `crates/nvs-stdlib/src/cache.rs`'s `SHARED_DOC` card and its `RuntimeError` description, plus the
   matching text in `crates/nvs-stdlib/src/ratelimit.rs` — both name `net.connect` as the grant. They
   are the source `docs/novis.md` is generated from, so the reference follows when they change and is
   never edited by hand.
2. Any `nvs.toml` fixture under `tests/` granting `net.connect` for a store's host. Re-point at
   `cache.shared`; a fixture keeping the old pair is a fixture proving the old rule.

## Stage 1 — the floor

Goal `parses`'s whole acceptance list — the parity program, never traded.

## Stage 2 — the keystone: the grant names the store

1. **`Cap::CacheShared`** in `crates/nvs-config/src/capability.rs`'s roster, spelled `cache.shared` in
   `nvs.toml`, asked at `Scope::Unscoped`. Its doc comment carries § 1's reasoning by pointing at
   `Cap::MailSend`'s, which already states it — one home, not two.
2. **The door stops asking about an address.** `open_configured` in
   `crates/nvs-stdlib/src/cache.rs:@open_configured` asks the new grant and drops `pin_host`, so neither
   `Core\Cache::shared()` nor `Core\RateLimit::consume` consults the denied-range table. Both callers
   keep their own refusal sentence — what to do instead is the caller's contract, which is why the door
   takes a `remedy` clause at all.
3. **The migration is reported at boot.** A `[cache.shared] url` set with no `cache.shared` grant is a
   pair the configuration holds, so it is a `Warn` naming both keys rather than a throw on the first
   request that touches the tier.
4. **The registry card and the reference** — `crates/nvs-stdlib/src/registry.rs`'s capability row for
   the shared tier moves from `net.connect` to `cache.shared`, and the local tier's "no grant" row and
   the asymmetry between them stay exactly as `cache.rs`'s module doc already states.

## Stage 3 — the transport for the cache tier

1. **`unix:` in `[cache.shared] url`.** `endpoint` in `crates/nvs-stdlib/src/cache.rs` gains one arm
   beside the `redis://` one; the `rediss://`, path-as-database-index and bad-port refusals are
   untouched. The connection type behind `open_shared` widens from a `SocketAddr` to an address-or-path
   — that key is what decides reuse-or-replace, which is how a reloaded configuration looks from there.
2. **`E0635`** — a Unix spelling on a build with no `AF_UNIX` transport, refused at boot with a note
   naming the platform and pointing at loopback TCP. Declared in `crates/nvs-diagnostics/src/lib.rs`
   at the end of its band, whose next free number this is; `E0626` is the neighbouring refusal in
   every sense but numerically, and `E0635`'s own doc comment is where the two are told apart.
3. **`crates/nvs-stdlib/src/cache/redis.rs` is transport-agnostic**, which it nearly is already: it
   takes a `SocketAddr` rather than a `Ctx` so a test can drive a whole exchange against a listener on
   loopback. The same split holds for a socket; what changes is what it is handed, not what it speaks.
4. **The proofs** — a record written through the socket and read back through it, and the boot refusal
   on a build without the transport, asserted by the diagnostic rather than by a failed connect.

## Stage 4 — the transport for the drivers

1. **A path in `[db.<name>] host`**, per § 3's overload: a value beginning with a path separator is a
   socket. `crates/nvs-config/src/db.rs` already distinguishes rooted from relative for SQLite's sake
   (`is_relative_file`), and that predicate is the neighbourhood, not the answer — SQLite's path is a
   database and this one is a socket.
2. **`AF_UNIX` connect in `mysql.rs`, `maria.rs` and `pg.rs`.** One transport added under three
   drivers that each keep their existing wire; `conn.rs` is where the two transports meet, and
   `rule:core-classes/db-drivers-are-an-enum`'s sans-IO split is
   what makes this an addition at the edge rather than a second codec.
3. **Postgres derives `<host>/.s.PGSQL.<port>`**, per § 5, asserted against the path the driver connects
   to rather than against a successful connection — the derivation is what is under test.
4. **`tds/mod.rs` refuses a path**, reported as a target the driver does not speak. **SQLite is untouched.**
5. **The matrix gains a socket leg** for the three drivers, running the case list the TCP legs run: the
   property a second transport has to have is that the driver agrees across both.

## Standing decisions

- **This goal opens no new ADR number.** `rule:config/cache-shared-is-the-grant-over-the-configured-store`
  is written and its cross-links are landed; every question this goal meets is answered in one of its six
  sections. A gap found in it is an edit to *that* rule's fragment, with a record whose `changes:` block
  names it, never an overlay.
- **`cache.shared` is unscoped and stays unscoped.** A deployment has one shared store, which
  `ratelimit.rs`'s module doc already states; a scope with one possible value reads as a decision nobody
  made.
- **A program-supplied socket path is refused at every door** — `Core\Net::connect`, `Core\Db::open`'s
  settings, `Core\Http\Client` — and refused as a target rather than as a file that could not be opened,
  so the two cases cannot be told apart by the message. § 6's `net.local` is named in the ADR and **is
  not** added to the roster here: it has no caller until `Core\Net` lands.
- **The boot refusal is a refusal, never a fallback.** A Unix spelling on a platform without the
  transport does not become loopback TCP; § 4 is the whole argument and it does not get re-litigated at
  the seam.
- **Ambiguity about where the address-or-path key lives resolves toward `cache.rs`** — it owns the
  policy half already, `redis.rs` owns what talks, and that split is the one the module doc states.
  Decided-and-recorded in that module's doc comment, never `BLOCKED`.
- **What this spends**, per `rule:programs/memory-priority`: nothing new per request.
  One socket per core either way, and a local connect that skips the IP stack is strictly less work than
  the loopback round trip it replaces.
