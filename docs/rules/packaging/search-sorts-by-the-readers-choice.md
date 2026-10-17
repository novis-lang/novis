The registry's search matches text, then sorts by an order the reader chooses: by default a mix of
witnessed work and downloads, and also by date, by activity, by downloads or by name.

**Witnessed work is only what the registry saw with its own clock or measured in the code it built:**
age on the registry, months with at least one release, time since the last release, public API size
(from `nvs pkg api`), doc-comment coverage, and retractions that name a fixed-in version. Git history,
such as the first commit date or the commit count, is shown as "from the repository, not checked" and
never ranks, because anybody can write it.

**The formula is one file in the repository**, `apps/registry/src/Ranking.nvs`, whose doc comment states
it in words. Every package page shows every fact the formula reads and links to that file. The aim is
real packages, not one-function helpers, so the facts favour a maintained API over a count of names.

**A package page shows** each version with its state (pending, unlisted, retracted or withheld), the
capabilities each version requests and what changed from the version before, the API diff per version,
the commit each version was built from, the owners and their history, advisories, `supersedes`, and the
facts behind the ranking.

**Downloads are counted without storing an address.** The service serves archives itself and counts
per version per day. One count per address per day goes through a hash with a daily salt, and the salt
is deleted when the day ends, so no address and no lasting hash of one is kept. Copies from mirrors and
from `vendor/` are not counted.

A ranking a publisher can write into a repository is a ranking a publisher can fake. Facts the registry
observed itself cannot be backdated, and showing every one of them lets a reader judge the default
order instead of trusting it.

**Memory:** a download count is one row per version per day. The day's salt and its set of hashes are
the only per-address data, and both are deleted when the day ends.

**Declined:**
- **Ranking without downloads.** Downloads are the one signal of what people actually use, and the
  reader can still choose a sort without them.
- **Ranking by git history.** A commit date and a commit count are whatever the repository says.
- **A secret or unpublished formula.** A reader cannot judge an order whose inputs are hidden.
- **Storing addresses to count downloads.** An address is personal data, and a daily salt counts
  unique fetches as well.

**Not on disk.** There is no registry.
