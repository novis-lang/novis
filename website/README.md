# The Novis website

Astro + Starlight. Everything the site needs — tooling, rules, content — lives inside
this folder. The repository's unattended loop does not write here; what it writes is the
example tree at `../docs/examples/`, which `sync:examples` mirrors into `examples/` like
any other generated input. Updating the site from the repository is **one command, fired
by a human (or an agent that was asked to)**:

```sh
npm run sync     # pull the rulebook + the Core reference from the repository
npm run build    # build the static site into dist/ 
npx astro build --base /novis/ # build with a custom base
```

## Commands

| Command | What it does |
| --- | --- |
| `npm run dev` | dev server with live reload |
| `npm run build` | production build into `dist/` |
| `npm run preview` | serve the built site locally |
| `npm run sync` | the render and the two sync scripts, in order |
| `npm run sync:render` | `bun nv render --website`: publish `../data/rules/` and `../docs/rules/` → `src/content/docs/docs/rules/**` + `src/data/rules.json`, and the spec + registry → `src/data/core.json` + the Core member pages |
| `npm run sync:decisions` | re-render `../docs/decisions.toml` → `src/data/decisions.json` (the plain-language summary) |
| `npm run sync:examples` | mirror `../docs/examples/` → `examples/` |
| `npm run examples:check` | run every example in `examples/` through the real `nvs` binary and diff against its `.out` file |

## Who owns which file

The whole design hangs on one rule: **tool-owned files are regenerated from scratch;
human-owned files are never overwritten.** For Core reference pages, ownership is
per page and the `novis.draft: true` frontmatter flag is the switch: while it stands,
the page is tool-owned and `sync:render` regenerates it on every run (so its defaults can
never go stale); removing the flag hands the page to humans forever. The stubs contain
no generated prose — every default renders at build time from `core.json` through the
`Method*` components, so even a human-owned page keeps following the repository
wherever it kept a component.

| Path | Owner | Notes |
| --- | --- | --- |
| `src/content/docs/docs/rules/**` | tool | regenerated on every `sync:render`, except the handwritten hub at `index.mdx` — a rule's prose lives in `../docs/rules/`, where the rule is |
| `config/rule-sections.mjs` | human | where each chapter is cut into pages — the one thing about the rulebook the repository does not own |
| `src/data/core.json`, `src/data/rules.json` | tool | regenerated on every sync |
| `src/data/core-changelog.json` | human | per-member changelog entries |
| `src/content/docs/docs/core/**.mdx` | **per page** | tool-owned (regenerated every `sync:render`) while `novis.draft: true`; remove the flag to take ownership — then yours: lead text, description, parameter docs, errors, tips, `<SeeAlso ids={…}>` |
| `src/content/claims/*.md` | human | one file per "Why Novis?" claim; add a file, the page updates |
| `examples/**` | tool | a mirror of `../docs/examples/`, emptied and rewritten on every `sync:examples` — edit the repository's copy, which is where the sweep that writes them lives ([ADR 0134](../docs/decisions/0134.md)). **Gitignored**, unlike the rulebook pages: those are transformed on the way in, this is the same bytes twice. `examples:check` runs it, so a stale mirror fails here rather than shipping |
| `config/spec-overrides.mjs` | human | corrections for spec table rows the renderer cannot read — every fix goes here, never into the renderer |
| `config/site.mjs` | human | **all placeholder URLs live here** — swap them once to go live |
| `config/external-links.mjs` | human | the rule that a link off the site opens in a new tab with `rel="noopener noreferrer nofollow"` — except the project's own repository and Discord links, which open in a new tab with no `rel`: an integration decorates content links at build time, and a component spreads `externalLinkAttrs(href)` onto any anchor it writes itself |

A page whose member leaves the published set is deleted while it is still tool-owned. A
human-owned one stays until a human deletes it.

## How the Core reference works

`bun nv render --website` writes it (`../tools/nv/renderers/website-core.ts`).

1. The member tables of `../docs/spec/01-core-library.md` — the file the repo declares
   authoritative for the designed surface — are the `../data/spec/core-members.json`
   record. Well-formed member rows become full member records (signature, parameters,
   options, return type, PHP "Replaces" list, taint classification). The chapter's
   prose gives each class its summary, enums and constants. A row the renderer cannot
   read is a **render warning**, fixed in `config/spec-overrides.mjs`.
2. The built `nvs` binary's registry (`nvs meta --json`, ADR 0117) says what is
   implemented, and **only implemented members are published**: a member the registry
   does not hold gets no page, a class with no registered member does not appear at
   all, and both arrive on the render after they land. With no binary built, the render
   stops.
3. The same registry carries each member's card: short description,
   parameter/shape-key/return/error descriptions, authored next to the Rust
   implementation. Precedence is **field-wise**: a doc field the registry carries wins,
   one it lacks falls back to the spec, so documentation moves member by member.
4. Member pages render every default from the data at build time (`<MethodSignature>`,
   `<MethodLead>`, `<MethodDescription>`, `<ParamDocs>`, `<MethodReturn>`,
   `<MethodErrors>`); human prose replaces a component where the default is not enough,
   and survives every sync once the page's `draft` flag is removed.

Novis code blocks get syntax highlighting from `config/novis.tmLanguage.json`
(languages `novis` / `nvs` in fenced code blocks).

## How the rulebook works

The repository owns the rules: `../data/rules/<topic>.json` names a chapter and holds
its rules in reading order, `../data/rules/<topic>/<slug>.json` is one rule's record, and
`../docs/rules/<topic>/<slug>.md` is the prose. `sync:render` publishes all of it as three
levels — a hub, 22 chapter pages, and one page per **section**.

A section is a contiguous run of a chapter's rules, and it is the one thing about the
rulebook the repository does not own, because a chapter there is one document a person
scrolls and a chapter here cannot be: Security alone is 89 rules. The cut lives in
`config/rule-sections.mjs`, named by first-rule slug rather than by index, so a rule
added mid-chapter joins the section it was written into and a stale cut fails the sync
loudly. A chapter cut into one section has no section pages at all — its rules render on
the chapter page, so a short chapter costs one click rather than two.

Pages are plain `.md`, never `.mdx`: rule prose is full of `#[Attribute(…)]`, `{field: T}`
and `<T>`, all of which MDX would read as JSX. The per-rule chrome is raw HTML around the
prose, which Markdown parses normally either side of a blank line. Every rule gets an
anchor of its own slug, and every `rule:<topic>/<slug>` citation in the prose is rewritten
to point at it — so a cross-reference survives any later edit to a heading.

The site publishes what is **true now**. The frozen rationale behind a rule stays in the
repository at `../docs/decisions/NNNN.md`, and a rule's "Decided in" row links out to it
(`config/site.mjs` § `decisionRecord`). `/docs/decisions/` is the plain-language summary of
that same set, rendered from `src/data/decisions.json`.

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
| `--nv-rose` | `#bd767a` | as `--nv-quiet`: the small annotations beside something else — a comment in a signature, a chip, a hint |
| `--nv-mist` | `#ffe4e4` | what the accent lies *on* — the light theme's highlight ground, selected text |
| `--nv-teal` | `#5abab6` | affirmative ("accepted", "proof", keywords) and, at a whisper, the tint of every neutral surface |

Surfaces are the load-bearing part. Page, panels, hairlines and body text are one ramp of
near-grey between `--nv-ink` and `--nv-fog`, both a few percent of teal, so the ground stays
a grey that merely leans cool: tint it toward the accent instead and it stops reading as a
neutral carrying an accent and starts reading as a colored page. The ramp's six steps land
on Starlight's own lightnesses, which is what keeps the stock contrast ratios.

The two themes lighten and darken the same four. The light theme uses the accent as drawn;
the dark theme has to lift it (`#b20038` is 2.4:1 on that ground), but lifts **lightness
alone** — `oklch(from var(--nv-accent) 0.68 c h)`, keeping the crimson's own chroma and hue,
because mixing toward white takes the chroma with it and gives back the rose instead of the
accent. Caution and danger (`--nv-warn`, `--nv-danger`) are the two
semantics the palette cannot carry and are the only colors outside it — a taint sink drawn
in the accent's crimson is indistinguishable from a link. Every text token clears WCAG AA
against the surface it is painted on, in both themes; keep it that way when adding one.

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
magick mark.png -filter Lanczos -resize 160x160 -background white \
  -gravity center -extent 180x180 -alpha remove -alpha off public/apple-touch-icon.png
```

`mark.png` and the three `i*.png` are scratch, and nothing ignores them — delete them once
the two icons are written.

The trim matters: the export carries a thin transparent margin, which at 16px costs the ring
its counters. Each size is sharpened for the size it is, which `-define icon:auto-resize`
cannot do. The home-screen icon is padded to white because iOS fills transparency with black,
and white is the ground the mark's own disc already carries under its lower half.

## Going live checklist

1. Replace `SITE_URL` in `config/site.mjs` — GitHub and Discord already point at the real ones.
2. Replace the sitemap URL in `public/robots.txt`.
3. Replace the demo data in `src/content/docs/impressum.md` and `datenschutz.md`.
4. Review pages still carrying `draft: true` / draft-flagged claims.
5. Set the Pages source to **GitHub Actions** and the custom domain to `SITE_URL`'s host. No
   `CNAME` file: GitHub writes one only when publishing from a branch, and ignores any that exists
   when a workflow publishes — one in `public/` would ship as a dead file at `/CNAME`.
6. `npm run sync`, review what it wrote, and commit it — publishing is the push, not a command.

## How it is published

`.github/workflows/pages.yml` builds this folder on every push to `main` and hands `dist/` to
GitHub Pages as an artifact. Nothing built is ever committed: `dist/` is gitignored, there is no
`gh-pages` branch, and the deployed site is the build of the commit it came from.

The workflow runs `npm ci && npm run build` — **not** `npm run sync`. Sync rewrites tracked files
and reads the built `nvs` binary for registry docs, so a deploy that ran it would publish less than
your local sync does and would decide page ownership with nobody looking. The site therefore
follows the repository only as far as the last committed sync: after landing rules, spec or
registry changes, run `npm run sync` and commit what it wrote.

The same workflow builds on pull requests without deploying, so a build that no longer compiles
fails there rather than on `main`.
