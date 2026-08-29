# The Novis website

Astro + Starlight. Everything the site needs — tooling, rules, content — lives inside
this folder; nothing here is touched by the repository's unattended loop, and updating
the site from the repository is **one command, fired by a human (or an agent that was
asked to)**:

```sh
npm run sync     # pull ADRs + the Core reference from the repository
npm run build    # build the static site into dist/
```

## Commands

| Command | What it does |
| --- | --- |
| `npm run dev` | dev server with live reload |
| `npm run build` | production build into `dist/` |
| `npm run preview` | serve the built site locally |
| `npm run sync` | both sync scripts, in order |
| `npm run sync:adrs` | republish `../docs/adr/*.md` → `src/content/docs/docs/adr/` + `src/data/adrs.json` |
| `npm run sync:core` | reparse the spec + registry → `src/data/core.json`, create missing member pages |
| `npm run examples:check` | run every example in `examples/` through the real `nvs` binary and diff against its `.out` file |

## Who owns which file

The whole design hangs on one rule: **tool-owned files are regenerated from scratch;
human-owned files are never overwritten.** For Core reference pages, ownership is
per page and the `novis.draft: true` frontmatter flag is the switch: while it stands,
the page is tool-owned and `sync:core` regenerates it on every run (so its defaults can
never go stale); removing the flag hands the page to humans forever. The stubs contain
no generated prose — every default renders at build time from `core.json` through the
`Method*` components, so even a human-owned page keeps following the repository
wherever it kept a component.

| Path | Owner | Notes |
| --- | --- | --- |
| `src/content/docs/docs/adr/**` | tool | regenerated on every `sync:adrs` — edit the ADRs in `../docs/adr/` instead |
| `src/data/core.json`, `src/data/adrs.json` | tool | regenerated on every sync |
| `src/data/core-changelog.json` | human | per-member changelog entries; the tool only creates the empty file |
| `src/content/docs/docs/core/**.mdx` | **per page** | tool-owned (regenerated every `sync:core`) while `novis.draft: true`; remove the flag to take ownership — then yours: lead text, description, parameter docs, errors, tips, `<SeeAlso ids={…}>` |
| `src/content/claims/*.md` | human | one file per "Why Novis?" claim; add a file, the page updates |
| `examples/core/<Class>/<member>/*.nvs` | human | runnable examples; sibling `.out` = expected output, verified by `examples:check` |
| `scripts/spec-overrides.mjs` | human | corrections for spec table rows the parser cannot read — every fix goes here, never into the parser |
| `config/site.mjs` | human | **all placeholder URLs live here** — swap them once to go live |

A member removed from the spec leaves its page behind as an *orphan*; `sync:core` lists
orphans at the end of its run for a human to review and delete.

## How the Core reference works

1. `scripts/lib/spec.mjs` parses `../docs/spec/01-core-library.md` — the file the repo
   declares authoritative for every `Core` signature. Well-formed member tables become
   full member records (signature, parameters, options, return type, PHP "Replaces"
   list, taint classification); roster tables and bullet rosters become *surface*
   classes ("Planned" in the sidebar); anything unreadable is a **sync warning**, fixed
   in `scripts/spec-overrides.mjs`.
2. `scripts/lib/registry.mjs` scans `../crates/nvs-stdlib/src/*.rs` for registered
   `CoreClass` declarations — that is what drives the per-member
   **Available / Not yet implemented** badge, automatically, on every sync.
3. `scripts/lib/meta.mjs` asks the built `nvs` binary for its registry docs
   (`nvs meta --json`, ADR 0117): short description, parameter/shape-key/return/error
   descriptions, authored next to the Rust implementation. Precedence is **field-wise**:
   a doc field the registry carries wins, one it lacks falls back to the spec — so
   documentation migrates member by member with no flag day. A toolchain without the
   subcommand just means "spec only", reported, never fatal.
4. Member pages render every default from the data at build time (`<MethodSignature>`,
   `<MethodLead>`, `<MethodDescription>`, `<ParamDocs>`, `<MethodReturn>`,
   `<MethodErrors>`); human prose replaces a component where the default is not enough,
   and survives every sync once the page's `draft` flag is removed.

Novis code blocks get syntax highlighting from `config/novis.tmLanguage.json`
(languages `novis` / `nvs` in fenced code blocks).

## How the ADR pages work

`sync:adrs` republishes every `../docs/adr/NNNN-*.md` verbatim, with three additions:
a styled metadata panel (status, date, scope, amends/amended-by with resolved titles),
rewritten links (ADR→ADR links stay on the site; links into the repo go to GitHub), and
the site-wide **hover tooltips**: any link to `/docs/adr/NNNN/` anywhere on the site
shows the target ADR's title, status and "In short" summary instantly on hover or
keyboard focus (`src/scripts/adr-tooltips.ts`).

## Machine-facing surface

- `/sitemap-index.xml` — generated by Starlight on every build (needs `SITE_URL`).
- `/llms.txt` — every page with its description, for agents (`src/pages/llms.txt.ts`).
- `/site-index/` — the same inventory as a human-readable page.
- `public/robots.txt` — allows everything, names the sitemap.

## Theming

One knob: `--nv-base` at the top of `src/styles/custom.css`. Every accent shade, badge
and highlight derives from it via `color-mix()` — change that line and the whole site
re-tints.

## Going live checklist

1. Replace every constant in `config/site.mjs` (domain, GitHub, Discord).
2. Replace the sitemap URL in `public/robots.txt`.
3. Replace the demo data in `src/content/docs/impressum.md` and `datenschutz.md`.
4. Review pages still carrying `draft: true` / draft-flagged claims.
5. `npm run sync && npm run build`, deploy `dist/`.
