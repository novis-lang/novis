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

**Not on disk.** There is no registry and no `nvs pkg audit`. Resolution refuses a retracted version
the lock does not already have and names its fix (`crates/nvs-cli/src/package/resolve.rs`).
