`nvs --help` opens with two lines and no third:

```
Novis-Lang
version: 0.0.1 · commit: 9c62ae176 · 2026-09-07
```

Line one is the identity, and it is `README.md`'s H1 verbatim rather than a second
phrasing of it, and a test in `crates/nvs-cli/src/main.rs` asserts the two agree. Line two is which build is answering, which is the first thing a bug report needs and the
last thing a reporter thinks to look up. `-dirty` is appended to the hash when the tree was modified at
build time, so a binary built over uncommitted work says so in its own banner.

**A build that is not a release says so in its version.** `nvs --version`, this banner and `nvs info`
print `0.0.1+dev` for every build but a release, and `0.0.1` for the release alone. `+dev` is SemVer
build metadata, so it changes no version comparison. Without it, every build of `main` after a release
prints that release's number. `Cargo.toml` keeps the released version until the next release run moves
it, so no number is guessed ahead of the release that decides it. `crates/nvs-cli/build.rs` calls a
build a release when `NVS_RELEASE` names its version, which the release workflow sets because it
builds before the tag exists, or when the checkout is clean and `HEAD` carries `v<version>`. A build
it cannot tell about is `+dev`, so a distribution packaging a release from a tarball sets
`NVS_RELEASE`. Every other use of the version, such as cache keys, the language server and the HTTP
client's `User-Agent`, keeps the plain number.

**The date is the commit's, never the build's.**
`rule:packaging/a-build-records-no-timestamp` is the reason and holds the mechanism: derived from the
commit, it answers how old the binary is while leaving two builds of one commit byte-identical.

`Novis-Lang` is underlined through `anstyle`, which clap writes via `anstream`. A redirected or piped
`--help` therefore gets plain text and `NO_COLOR` is honoured, without the code that builds the string
testing for a terminal — the same arrangement the diagnostic renderer already runs under.

Because the date and hash are the commit's, `--help` changes with every commit and with nothing else:
two builds of one commit, in separate checkouts, print the same banner.

**Declined:**
- **A wall-clock build timestamp on line two.** It makes every build of identical source a different
  binary for a cosmetic field (`rule:packaging/a-build-records-no-timestamp`).
- **Moving `main` to the next version, or a `-dev` pre-release of it, right after a release.** Below
  1.0 the bump chosen at release time decides the next number, and `major` needs `allow_contract`
  (`rule:packaging/below-1-0-the-breaking-slot-moves-left`). A number written earlier is a guess that
  the release would often have to correct.
- **The commit hash as the build metadata.** The line already names the commit, so `+dev` says the
  one thing the line was missing.
- **A one-line `about` string of its own.** It drifts from the README's H1, which is two homes for
  one fact.
