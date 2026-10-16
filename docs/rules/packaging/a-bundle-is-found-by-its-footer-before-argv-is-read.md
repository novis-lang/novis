The build command appends, after the host binary's own last section, the flat file list followed by a
fixed-size footer:

```
magic ("NVSB") | format_version: u16 | manifest_offset: u64 | manifest_len: u64
```

Appended bytes are invisible to the PE and ELF loaders, which read no further than the sections their
own headers describe, so an `nvs` that finds no footer is the `nvs` it always was.

**The check runs at process start, before a single argument is parsed**, because a bundle's `argv`
belongs to the program it carries: an app whose first argument happens to be `run` or `--help` must not
have it eaten as one of `nvs`'s own. The binary reads its own executable's last footer-sized bytes; on a
magic match it resolves the entry point — entry zero of the list — and every `require` against the
embedded table instead of the real filesystem, and hands each file's bytes to the ordinary
content-hash-then-cache-lookup path exactly as it would for a file read from disk.

No new cache mechanism, no new isolation boundary, no new capability: it is the same `nvs run <entry>`
code path with one different byte source for reads, so there is no second interpreter to drift from
the first. A footer whose `format_version` this host does not understand is treated as no bundle at
all, which is the one behaviour that cannot corrupt anything.
