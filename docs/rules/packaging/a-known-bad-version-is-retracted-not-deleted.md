A publisher marks a version **retracted** with a reason and a fixed-in version. Resolution refuses to
*select* a retracted version and names the fix, while the bytes remain fetchable forever, so an
existing lockfile still builds. Deletion is not offered.

**`nvs audit`** reads the registry's signed advisory feed, reports the minimum bump that clears each
advisory, and exits non-zero for CI. Retraction plus audit is the answer to minimal version
selection's one real weakness — that a patch nobody asked for does not arrive on its own
(`rule:packaging/resolution-takes-the-highest-minimum`) — and it covers the one case a version
range expresses that a minimum cannot: "not 2.3.1, it is broken".

If audit proves insufficient in practice and users sit on known-vulnerable versions, the thing to
argue is a narrowly scoped automatic patch floor, not a general range system.
