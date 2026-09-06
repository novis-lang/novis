```
nvs info                # build, host and licensing facts, plus the component table
nvs info --licenses     # the same, plus every license text in full
nvs -i / nvs -i --licenses
```

"What is this binary, what is in it, and what may I do with it?" is one question asked by one person at
one moment, so it is one call, the shape `php -i` already has. The default is the summary because the
full texts are some 55 KB and a terminal is the wrong place to put them unasked; `--licenses` is the
complete legal record. The report is plain two-column text with no colour and no paging, so it pipes.

**This is the one place in Novis where an operation is deliberately reachable two ways.**
`rule:core-api/shape-rules`'s "no operation reachable two ways" governs the `Core` library, not the
CLI, and the reason is specific: `-i` is the spelling a PHP developer tries first, and the point of the
command is that nobody should have to hunt for it. Combining `-i` with a subcommand is refused rather
than guessed at.

Fields that do not exist yet are not printed; the report grows a section as each thing it describes
lands. **It reports no per-request state, ever** — that is `rule:testing/debug-probes`' territory and is
flag-gated for reasons this command does not share.
