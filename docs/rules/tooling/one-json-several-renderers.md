`nvs meta --json` (`rule:tooling/meta-json`) is the single machine-readable source of documentation, and
every renderer consumes it:

```
                      ┌─ nv reference        → docs/novis.md
                      ├─ nv render --website  → core.json → MDX
nvs meta --json ──────┼─ nvs doc              → Markdown pages
                      └─ nvs agent            → the primer, the index, one card
```

Nothing re-derives documentation from source, and no renderer is authoritative for content. The `Core`
half of the JSON is the registry's card (`rule:core-api/reference-card`); a program's own declarations
join the same document through one optional argument (`rule:tooling/meta-json-takes-a-program`) rather
than through a second pipeline beside it, and the renderer shipped in the binary
(`rule:tooling/nvs-doc-renders-and-decides-nothing`) is only that.

Two renderers already sit on the `Core` half — the one-file reference and the website's core data — and
neither reads a Rust file to find a description. A third source of truth for user declarations would be
exactly the duplication that shape exists to avoid.

The fourth arm answers a coding agent rather than a reader (`rule:tooling/an-agent-asks-the-binary`),
and it is on this diagram for the reason the others are: it renders at the call and holds nothing, so
the binary that compiles a program is the binary that documents it.
