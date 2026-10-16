`Ldap\Connection::search` returns an `Ldap\Entries` that reads one page at a time as the loop reaches it, throws when the server stopped at a size or time limit, and never follows a referral. Every control `Core\Ldap` sends is a named option of the member it changes, and every extended operation is the member that needs it.

`search` sends the paged-results control (RFC 2696) on every request, `pageSize` 1000 by default, and
reads the first page itself, so a base that does not exist or a filter the server refuses throws at the
call. **A size or time limit the server hit throws** `SizeLimitExceeded` or `TimeLimitExceeded`, and
the page it came with is dropped, so a partial result is never returned as if it were complete. A
connection released with a search still paging is closed, not pooled
(`rule:security/ldap-pool-is-bound-as-its-block`). `read` is a base-scope search for one entry and
returns `null` for `noSuchObject`.

**A referral is never followed.** A continuation reference in a search is data, read from
`Entries::references()`; a referral answering any other operation throws the kind `Referral`. A
referral the client followed would dial a server the server named, not one the operator or a grant in
`rule:core-classes/ldap-capabilities` named.

Active Directory's four extras are named options or members:

- **Sort and a window.** `sort` (one attribute, because AD sorts on one key) with `descending`, and
  `window` with `offset`, which need `sort`, replace paging for that search, and fill
  `Entries::total()` with the server's count. Both are checked before anything is sent.
- **Deleted objects.** `showDeleted` includes deleted entries, and `Entry::isDeleted` says which.
- **Incremental sync.** `changes` is DirSync: it returns the entries changed since the `cookie` the
  program stored, and `Changes::cookie()` is the next one. On a server without DirSync it throws
  `Unsupported`.
- **The Global Catalog** is a block whose URL uses port 3268 or 3269. It is read-only: every write but
  `compare` throws `ReadOnly` before a byte is sent.

`modify` sends one Modify request, so the server applies every `Ldap\Change` in order or none of them.
`rename` derives the new first level and, where it moved, the new parent from its second DN. `compare`
returns `bool` and throws on an error.

- **Memory:** a search holds one page of up to `pageSize` entries and the current entry, per search a
  request has open. A finished search leaves nothing on its connection.

**Declined:**
- **A `bind` member, a raw-control member and a raw-extended-operation member.** Each would be a
  second form of an operation the named members already have
  (`rule:core-api/one-paradigm-per-operation`), and a rebind would let a request change a pooled
  connection's identity.
- **Following referrals, or a rebind callback.** See above.
- **`uSNChanged` polling pinned to one controller.** It is a second form of DirSync's job, correct only
  while every call reaches the same controller, which the URL list's failover does not promise.
- **A client-side sort, as PHP's `ldap_sort`.** The server sorts with `sort`, so a second sort would be
  a second form of the same operation.
- **A `compare` that returns `-1` on an error.** A third value a caller has to test for beside `true`
  and `false`.
