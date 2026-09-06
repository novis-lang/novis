`nvs meta --json` (`rule:tooling/meta-json`) is the single machine-readable source of documentation, and
every renderer consumes it:

```
                      ┌─ tools/reference.py  → docs/novis.md
nvs meta --json ──────┼─ website sync:core    → core.json → MDX
                      └─ nvs doc              → Markdown pages
```

Nothing re-derives documentation from source, and no renderer is authoritative for content. The `Core`
half of the JSON is the registry's card (`rule:core-api/reference-card`); a program's own declarations
join the same document through one optional argument (`rule:tooling/meta-json-takes-a-program`) rather
than through a second pipeline beside it, and the renderer shipped in the binary
(`rule:tooling/nvs-doc-renders-and-decides-nothing`) is only that.

Two renderers already sit on the `Core` half — the one-file reference and the website's core data — and
neither reads a Rust file to find a description. A third source of truth for user declarations would be
exactly the duplication that shape exists to avoid.
