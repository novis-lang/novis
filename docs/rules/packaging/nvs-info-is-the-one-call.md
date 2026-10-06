```
nvs info                # build, host and licensing facts, plus the component table
nvs info --licenses     # the same, plus every license text in full
```

"What is this binary, what is in it, and what may I do with it?" is one question asked by one person at
one moment, so it is one call. The default is the summary because the full texts are some 55 KB and a
terminal is the wrong place to put them unasked; `--licenses` is the complete legal record. The report is
plain two-column text with no colour and no paging, so it pipes.

**It is reachable one way.** There is no `nvs -i` alias: `rule:packaging/the-cli-surface-is-novis-own`
is why, and what one would cost — a global flag, a second `--licenses` hanging off it, and a
hand-written conflict check for a collision clap cannot express — is the rest of the argument.

Fields that do not exist yet are not printed; the report grows a section as each thing it describes
lands. **It reports no per-request state, ever** — that is `rule:testing/debug-probes`' territory and is
flag-gated for reasons this command does not share.
