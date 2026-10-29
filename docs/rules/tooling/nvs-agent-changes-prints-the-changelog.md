`nvs agent changes` prints what changed between two versions of Novis, from `CHANGELOG.md` embedded
whole in the binary. `--since <version>` prints every section newer than that version, newest first, as
the file has them. With no `--since` it prints the section of the version the binary is. `--json` prints
the same sections as one document, `{schemaVersion, version, since, sections: [{version, date, text}]}`,
under the conventions the other `nvs agent` documents keep (`rule:tooling/an-agent-asks-the-binary`).

An agent that edits a project written for an older Novis needs to know what moved since, and it has the
same problem with the changelog it has with the language: nothing in its weights is current. The binary
answers for the version it is, so the changelog it carries is the one that describes it, and a copy
fetched from the web may describe a version that is not installed.

**A released binary carries its own section.** The release workflow's build job prepends the section it
releases before it compiles, from the notes the `plan` job rendered from the same commit
(`.github/workflows/release.yml`). Without that the binary would carry every section but its own. A
build from a clone carries the sections already released, and none for the version it is.

**The answers at the edges.** A version older than the oldest section prints every section. A version
newer than the binary prints nothing and says on standard error which version the binary is. A version
the binary has no section for, with no `--since`, prints nothing and says so. Each of these succeeds. A
value that is not `MAJOR.MINOR.PATCH`, with an optional leading `v`, is a usage error with exit status
`2`.

**The primer names it, and `nvs agent init`'s pointers do not.** `rule:tooling/an-agent-asks-the-binary`
keeps the pointers to four verbs, because a verb added later is one no agent learns of until every
installed pointer is rewritten. The primer is read at the start of every session and is generated from
this binary's own reference, so a line in its marked usage block reaches every agent at its next session
with nothing rewritten. `changes` reads about releases, not about the language, so it is not a fifth way
to look a name up.

**The Formatter section is the reason to read it before a reformat.** A commit that changes what
`nvs fmt` writes is listed under *Formatter*, and a project reads that list and reformats in one
deliberate commit.

- **Memory:** the file's bytes, once per binary, a few KiB today and nothing on the request path. It is
  stored uncompressed. A later change compresses it if it ever passes 1 MiB.

**Declined:**
- **Fetching the changelog from the release page.** It needs a network, and it may describe a version
  that is not the one installed.
- **Compressing the file now.** It is a few KiB, and a decompressor is code that buys nothing at that size.
- **A section for an unreleased build.** A clone's build has no notes until a release renders them, and
  notes rendered at build time would describe commits that may never ship.
