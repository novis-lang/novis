# The Novis website

Astro + Starlight. Everything the site needs — tooling, rules, content — lives inside
this folder. The repository's unattended loop does not write here; what it writes is the
example tree at `../docs/examples/`, which `sync:examples` mirrors into `examples/` like
any other generated input. Updating the site from the repository is **one command, fired
by a human (or an agent that was asked to)**:

```sh
npm run sync     # pull ADRs + the Core reference from the repository
npm run build    # build the static site into dist/ 
npx astro build --base /novis/ # build with a custom base
```

## Commands

| Command | What it does |
| --- | --- |
| `npm run dev` | dev server with live reload |
| `npm run build` | production build into `dist/` |
| `npm run preview` | serve the built site locally |
| `npm run sync` | all three sync scripts, in order |
| `npm run sync:adrs` | republish `../docs/adr/*.md` → `src/content/docs/docs/adr/` + `src/data/adrs.json` |
| `npm run sync:core` | reparse the spec + registry → `src/data/core.json`, create missing member pages |
| `npm run sync:examples` | mirror `../docs/examples/` → `examples/` |
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
| `examples/**` | tool | a mirror of `../docs/examples/`, emptied and rewritten on every `sync:examples` — edit the repository's copy, which is where the sweep that writes them lives ([ADR 0134](../docs/adr/0134-every-shipped-feature-owes-four-proofs.md)). **Gitignored**, unlike the ADR mirror: that one is transformed on the way in, this one is the same bytes twice. `examples:check` runs it, so a stale mirror fails here rather than shipping |
| `scripts/spec-overrides.mjs` | human | corrections for spec table rows the parser cannot read — every fix goes here, never into the parser |
| `config/site.mjs` | human | **all placeholder URLs live here** — swap them once to go live |

A member removed from the spec leaves its page behind as an *orphan*; `sync:core` lists
orphans at the end of its run for a human to review and delete.

## How the Core reference works

1. `scripts/lib/spec.mjs` parses `../docs/spec/01-core-library.md` — the file the repo
   declares authoritative for the designed surface. Well-formed member tables become
   full member records (signature, parameters, options, return type, PHP "Replaces"
   list, taint classification); anything unreadable is a **sync warning**, fixed
   in `scripts/spec-overrides.mjs`.
2. `scripts/lib/registry.mjs` scans `../crates/nvs-stdlib/src/*.rs` for registered
   `CoreClass` declarations, and **only implemented members are published**: a member
   the registry does not hold gets no page, a class with no registered member does not
   appear at all, and both arrive automatically on the sync after they land. A page
   whose member leaves the published set is deleted if still tool-owned (`draft: true`)
   and reported as an orphan if human-owned.
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

Four knobs: the palette block at the top of `src/styles/custom.css`, taken from the logo.
Every other color — the accent scale, the neutral ramp, every hairline, wash and callout —
derives from those four via `color-mix()`, so re-tinting the site is still editing one
block. A hex literal belongs in that block and nowhere else.

| | | Where it lands |
| --- | --- | --- |
| `--nv-accent` | `#b20038` | links, active nav, focus rings — the one color allowed to shout |
| `--nv-rose` | `#bd767a` | the muted step of the neutral ramp: secondary text, hairlines, panel edges |
| `--nv-mist` | `#ffe4e4` | headings in the dark theme, the page ground in the light one |
| `--nv-teal` | `#5abab6` | everything affirmative — "accepted", "proof", keywords in a signature |

The two themes lighten and darken the same four: the raw accent reads on white (6.6:1) but
not on the dark ground (2.5:1), so the dark theme lifts it toward white and the light theme
uses it as drawn. Caution and danger (`--nv-warn`, `--nv-danger`) are the two semantics the
palette cannot carry and are the only colors outside it — a taint sink drawn in the accent's
crimson is indistinguishable from a link. Every text token clears WCAG AA against the
surface it is painted on, in both themes; keep it that way when adding one.

## Logo and favicon

`media/novis-logo.png` (512×512, the export beside its `.afdesign` source) is the one copy.
The header renders it through Starlight's `logo` option and sizes it in
`src/components/Header.astro`. The icons are cut from the same file — regenerate them after
a logo change, from `website/`:

```sh
magick media/novis-logo.png -trim +repage mark.png
magick mark.png -filter Lanczos -resize 48x48 -unsharp 0x0.6+0.5+0.02 i48.png
magick mark.png -filter Lanczos -resize 32x32 -unsharp 0x0.6+0.6+0.02 i32.png
magick mark.png -filter Lanczos -resize 16x16 -unsharp 0x0.6+0.8+0.02 i16.png
magick i48.png i32.png i16.png public/favicon.ico
magick mark.png -filter Lanczos -resize 160x160 -background '#ffe4e4' \
  -gravity center -extent 180x180 -alpha remove -alpha off public/apple-touch-icon.png
```

The trim matters: the export carries ~7% transparent margin, which at 16px costs the ring
its counters. Each size is sharpened for the size it is, which `-define icon:auto-resize`
cannot do. The home-screen icon sits on `--nv-mist` because iOS fills transparency with
black.

## Going live checklist

1. Replace `SITE_URL` in `config/site.mjs` — GitHub and Discord already point at the real ones.
2. Replace the sitemap URL in `public/robots.txt`.
3. Replace the demo data in `src/content/docs/impressum.md` and `datenschutz.md`.
4. Review pages still carrying `draft: true` / draft-flagged claims.
5. `npm run sync && npm run build`, deploy `dist/`.
