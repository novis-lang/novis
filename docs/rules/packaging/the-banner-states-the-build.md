`nvs --help` opens with two lines and no third:

```
Novis — The Web-Native Programming Language
version: 0.0.1 · commit: 9c62ae176 · 2026-09-07
```

Line one is the identity, and it is `README.md`'s H1 verbatim rather than a second
phrasing of it. Line two is which build is answering, which is the first thing a bug report needs and the
last thing a reporter thinks to look up. `-dirty` is appended to the hash when the tree was modified at
build time, so a binary built over uncommitted work says so in its own banner.

**The date is the commit's, never the build's.**
`rule:packaging/a-build-records-no-timestamp` is the reason and holds the mechanism: derived from the
commit, it answers how old the binary is while leaving two builds of one commit byte-identical.

`Novis` is underlined through `anstyle`, which clap writes via `anstream`. A redirected or piped
`--help` therefore gets plain text and `NO_COLOR` is honoured, without the code that builds the string
testing for a terminal — the same arrangement the diagnostic renderer already runs under.
