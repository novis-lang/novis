The registry's search matches text, then sorts by an order the reader chooses: by default a mix of
witnessed work and downloads, and also by date, by activity, by downloads or by name.

**Witnessed work is only what the registry saw with its own clock or measured in the code it built:**
age on the registry, months with at least one release, time since the last release, public API size
(from `nvs pkg api`), doc-comment coverage, and retractions that name a fixed-in version. Git history,
such as the first commit date or the commit count, never ranks, because anybody can write it. The
registry clones only the commit it builds, so it never reads that history, and a page shows only the
repository and the commit each version was built from.

**The formula is one file in the repository**, `apps/registry/src/Ranking.nvs`, whose doc comment states
it in words. Every package page shows every fact the formula reads and links to that file. The aim is
real packages, not one-function helpers, so the facts favour a maintained API over a count of names.

**A package page shows** each version with its state (pending, unlisted, retracted or withheld), the
capabilities each version requests and what changed from the version before, the API diff per version,
the commit each version was built from, the owners and their history, advisories, `supersedes`, and the
facts behind the ranking.

**Downloads are counted without storing an address.** A download is counted per version per day, from
the access lines of the hosts that serve the archives, or as the service serves one itself
(`rule:packaging/registry-hosting-is-configuration`); both go through one function. Only a request
whose user agent is `nvs` counts. One count per address per day goes through a hash with a daily salt,
and the salt is deleted when the day ends, so no address and no lasting hash of one is kept. Copies
from mirrors and from `vendor/` are not counted.

A ranking a publisher can write into a repository is a ranking a publisher can fake. Facts the registry
observed itself cannot be backdated, and showing every one of them lets a reader judge the default
order instead of trusting it.

Search is `GET /api/search?q=&sort=` (`apps/registry/src/Search.nvs`), over SQLite's full-text index
of each package's name, readme summary and doc comments. A query keeps its first eight runs of letters
and digits, so no full-text operator reaches the index. A pending, withheld or unlisted version counts
for nothing, and a package with no other version is not returned. `Ranking::facts` reads every fact in
one query per request, and the package page (`apps/registry/src/PackagePage.nvs`) shows each fact, the
score and a link to the formula. `Downloads` counts both kinds of download through `Downloads::add`.

A package's summary is the first paragraph of its readme, which `nvs registry build` prints beside
`doc_coverage`, `docs`, `readme`, `downloads` and `supersedes` (`crates/nvs-cli/src/package/build.rs`).
The release row keeps each of them, so a page never opens an archive or runs `nvs`; a release restored
from the log has them null, and its page says so. The release row's `declarations` is both the public
API size the ranking reads and the denominator of doc-comment coverage, so one count serves both facts.

The readme is escaped and shown as plain text. The account page (`apps/registry/src/AccountPage.nvs`)
needs any sign-in, a recovery code too: a visitor who lost a passkey must see the account to add one
and to revoke its tokens, the page changes nothing, and every change it leads to keeps its own passkey
check. A webhook is shown by its host alone, because its path can carry the receiver's secret.

**Memory:** a download count is one row per version per day. The day's salt and its set of hashes are
the only per-address data, and both are deleted when the day ends.

**Declined:**
- **Ranking without downloads.** Downloads are the one signal of what people actually use, and the
  reader can still choose a sort without them.
- **Ranking by git history.** A commit date and a commit count are whatever the repository says.
- **A secret or unpublished formula.** A reader cannot judge an order whose inputs are hidden.
- **Storing addresses to count downloads.** An address is personal data, and a daily salt counts
  unique fetches as well.
- **A `summary` key in `package.toml`.** It grows the manifest for a fact the readme already carries.
- **A rendered markdown readme.** `Core` has no markdown renderer, and one for text any publisher
  writes is a new parser on the request path and a new way to inject markup. It waits for a `Core`
  renderer with its own goal.
- **A cached score table.** The facts are computed per request in one query, O(packages matched); a
  cache waits until a few thousand packages make that cost measurable.
- **A separate search service.** SQLite's full-text index covers a few thousand packages.

**Not on disk.** There is no registry.
