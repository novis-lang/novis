The registry's operator may only **withhold** a version's bytes, for malware or a legal order, and
always as a signed, logged advisory that states its reason. The operator never deletes, never edits,
never reassigns a name, and runs no review queue.

- **A withheld version** keeps its name, its version and its digest in the index and the log; only its
  bytes stop being served. `nvs pkg audit` reports it as it reports a retraction
  (`rule:packaging/a-known-bad-version-is-retracted-not-deleted`), and resolution does not select it.
- **The advisory** is signed with the log's key and is a log entry
  (`rule:packaging/the-registry-keeps-an-append-only-log`), so a withhold cannot be silent.
- **A short published policy page** states this, in words an end user reads.

An operator who can delete, edit or reassign is a second way to change what a name means, and a
compromised operator account would be the most powerful credential in the registry. Withholding is the
one power malware and a court order need, and making it public and signed means every use of it can be
checked. A review queue would make the operator responsible for every package's content, which no
registry can carry.

**Declined:**
- **Deletion by the operator.** Every lockfile that holds the digest would stop building with no
  record of why.
- **Reassigning a name after a dispute.** A name that changes hands changes what every existing
  dependency on it means (`rule:packaging/a-package-name-is-vendor-slash-name`).
- **A review queue before publication.** It delays every release and implies a safety promise no
  reviewer can keep.

The operator withholds from a shell on the server, with `nvs run jobs/withhold.nvs --config nvs.toml
<package> <version> <reason>` (`apps/registry/jobs/withhold.nvs`). No web credential can withhold,
because the website has no operator account. `Releases::withhold` saves the reason and a `withhold` log
entry together, and the index file, signed with the log's key, shows `{"withheld": {"reason"}}`. The
registry's own archive route answers `451` for a withheld digest. Once the log and the index have the
reason, the job deletes the archive from every store and from the work folder (`Stores::remove`), and
running it again finishes a delete a store failed. Resolution refuses a withheld version and prints its
reason (`crates/nvs-cli/src/package/resolve.rs`). The policy page is `GET /policy`
(`apps/registry/src/PolicyPage.nvs`).

**Not on disk.** There is no `nvs pkg audit`. A CDN in front of a store may serve
a cached archive until its cache lifetime ends, because the service never purges a CDN.
