An owner marks a known-bad version **retracted** with a reason and a fixed-in version, and the bytes
stay fetchable forever. Resolution refuses to *select* a retracted version and names the fix, while an
existing lockfile still builds. Deletion is not offered.

**A retraction or an advisory sent with a token alone waits seven days**, the same fixed period a
token-only release waits (`rule:packaging/a-release-waits-seven-days-unless-an-owner-confirms`). One
confirmed with an owner's passkey on the website takes effect at once. A stolen token can therefore not
retract a good release, or announce a false advisory, before every owner has been notified
(`rule:packaging/an-owner-is-notified-without-email`) and has had time to answer.

**`nvs pkg audit`** reads the lock, never the network's idea of the graph, and checks it against
the registry's signed advisory feed. The feed has one entry per retraction and per withhold, signed
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
once a version fixes it, `fixed_in`, or a `withhold` with `package`, `version` and `reason`. The
advisories are in the log's order, and a client refuses a key it does not know. `nvs registry log
advisories` writes the feed from the log alone, so its every byte is derived from records the log
already signed.

```json
{"schema": 1, "advisories": [{"log": 7, "entry": {"kind": "retract", "package": "acme/http", "version": "1.1.0", "reason": "It deletes files.", "fixed_in": "1.1.1", "v": 1}}], "signed_at": 1793000000, "checkpoint": {"size": 42, "root": "<base64>"}}

— registry.example.com <base64 signature>
```

**A client checks the feed against the log before it uses it.** A signature that does not check
with the pinned key is an error. So is a retraction or a withhold among the log's first `size`
records that the feed does not list, a listed record the log has with other bytes, and a `root` that
is not the log's at `size` (`crates/nvs-cli/src/package/advisory.rs`). The feed covers a log that
the client's checkpoint continues, so a record past the feed's `size` is read from the log itself.
An `advisory` entry is not in the feed: no route writes one, and its `versions` has no syntax yet.

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

**Not on disk.** There is no `nvs pkg audit`, and the registry's service neither writes a
retraction's `fixed_in` into its log entry nor publishes the feed. Resolution refuses a retracted version
the lock does not already have and names its fix (`crates/nvs-cli/src/package/resolve.rs`).
