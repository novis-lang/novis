An owner marks a known-bad version **retracted** with a reason and a fixed-in version, and the bytes
stay fetchable forever. Resolution refuses to *select* a retracted version and names the fix, while an
existing lockfile still builds. Deletion is not offered.

**A retraction or an advisory sent with a token alone waits seven days**, the same fixed period a
token-only release waits (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`). One
confirmed with an owner's passkey on the website takes effect at once. A stolen token can therefore not
retract a good release, or announce a false advisory, before every owner has been notified
(`rule:packaging/an-owner-is-notified-without-email`) and has had time to answer.

**`nvs pkg audit`** reads the lock, never the network's idea of the graph, and checks it against
the registry's signed advisory feed. The feed has one entry per retraction, withhold and advisory, signed
with the log's key, and every entry is also a log entry
(`rule:packaging/the-registry-keeps-an-append-only-log`), so a feed that hides an advisory disagrees
with the log. For each locked version under an advisory, audit prints the advisory, its reason and the
smallest version that clears it, and exits non-zero while any remain, for CI. A withheld version
(`rule:packaging/the-operator-only-withholds`) is reported the same way. An unlisted version
(`rule:packaging/an-owner-may-unlist`) is not an advisory and is not reported.

**The advisory feed** is `log/advisories` under each file host, beside `log/checkpoint`. It is a
signed note under the log's key whose text is its JSON on one line, signed as an index file is
(`rule:packaging/a-package-is-its-digest`): `"schema": 1`, `advisories`, `signed_at` and
`checkpoint`, the `size` and base64 `root` of the log it lists. Each advisory is `log`, the number of
its record, and `entry`, that record's entry: a `retract` with `package`, `version`, `reason` and,
once a version fixes it, `fixed_in`, a `withhold` with `package`, `version` and `reason`, or an
`advisory` with `id`, `package`, `versions` and `summary`. The
advisories are in the log's order, and a client refuses a key it does not know. `nvs registry log
advisories` writes the feed from the log alone, so its every byte is derived from records the log
already signed.

```json
{"schema": 1, "advisories": [{"log": 7, "entry": {"kind": "retract", "package": "acme/http", "version": "1.1.0", "reason": "It deletes files.", "fixed_in": "1.1.1", "v": 1}}], "signed_at": 1793000000, "checkpoint": {"size": 42, "root": "<base64>"}}

— registry.example.com <base64 signature>
```

**A client checks the feed against the log before it uses it.** A signature that does not check
with the pinned key is an error. So is a retraction, a withhold or an advisory among the log's first `size`
records that the feed does not list, a listed record the log has with other bytes, and a `root` that
is not the log's at `size` (`crates/nvs-cli/src/package/advisory.rs`). The feed covers a log that
the client's checkpoint continues, so a record past the feed's `size` is read from the log itself.

**An advisory's `versions` is one range.** `A..B` covers the versions from `A` up to `B` and not
including it, and `A..` covers every version from `A`, while no version fixes it yet. `A` and `B` are
full versions and `A` is below `B`. A second range is a second advisory: there is no list and no other
operator, so this is not the range system dependencies decline. One parser reads it
(`crates/nvs-cli/src/package/manifest.rs`'s `Range`), and the log refuses an `advisory` entry whose
range it does not read (`crates/nvs-cli/src/package/entry.rs`).

An owner sends an advisory with `POST /api/packages/<vendor>/<name>/advisory` and
`{"versions", "summary"}` (`App::advise`). The registry checks the range with the same rules
(`apps/registry/src/VersionRange.nvs`) and answers `422` for one it does not read. It gives the
advisory its `id`, `<vendor>/<name>-<n>`, where `n` counts that package's advisories from 1, and
saves the row, the waiting log entry and the owners' notice in one transaction (`Releases::advise`).
A waiting entry has a `due` time, and the log takes it only from then (`Index.nvs`), so a token's
advisory is in neither the log nor the feed for seven days. The same advisory sent again with a
passkey moves that time to now. The package page lists each advisory, and a version's page lists the
ones whose range has that version (`PackagePage.nvs`).

The client is `nvs pkg advise <package> <versions> <summary>` (`crates/nvs-cli/src/package/advise.rs`),
one request to that route with the stored publish token, shaped like `nvs pkg retract`. Audit
reports each locked version whose advisory's range contains it (`Range::contains`), with the `id`,
the summary as the reason and `B` as the version that clears it. An `A..` range has no fix.
Resolution does not change: only a retraction or a withhold stops a version from being selected.

Retraction plus audit is the answer to minimal version selection's one real weakness, that a patch
nobody asked for does not arrive on its own (`rule:packaging/resolution-takes-the-highest-minimum`),
and it covers the one case a version range expresses that a minimum cannot: "not 2.3.1, it is broken".
If audit proves insufficient and users sit on known-vulnerable versions, the thing to argue is a
narrowly scoped automatic patch floor, not a general range system.

**Declined:**
- **Deleting a bad version.** Every lockfile that holds its digest stops building, and the log would
  record a name whose bytes no longer exist.
- **A token-sent retraction taking effect at once.** A stolen token could then pull a good release out
  of every fresh resolution before any owner saw it.

`nvs pkg audit` is `crates/nvs-cli/src/package/audit.rs`. The service writes a retraction's
`fixed_in` into its log entry (`apps/registry/src/Events.nvs`) and writes the feed again from the log
whenever the index is flushed (`apps/registry/src/Index.nvs`). Resolution refuses a retracted version
the lock does not already have and names its fix (`crates/nvs-cli/src/package/resolve.rs`).
