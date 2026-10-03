# The Novis website

Astro + Starlight. Everything the site needs — tooling, configuration, content — lives inside
this folder. The repository's unattended loop does not write here; what it writes is the
example tree at `../docs/examples/`, which the build reads in place, so the site keeps no copy
of it. Updating the site from the repository is **one command, fired by a human (or an agent
that was asked to)**:

```sh
npm run sync     # pull the Core reference from the repository
npm run build    # build the static site into dist/ 
npx astro build --base /novis/ # build with a custom base
```

## Commands

| Command | What it does |
| --- | --- |
| `npm run dev` | dev server with live reload |
| `npm run build` | production build into `dist/` |
| `npm run preview` | serve the built site locally |
| `npm run sync` | the render, under the name a human types |
| `npm run sync:render` | `bun nv render --website`: publish the spec + registry → `src/data/core.json` |

## Who owns which file

The whole design hangs on one rule: **tool-owned files are regenerated from scratch;
human-owned files are never overwritten.** No Core reference page is a file: one route
renders them all at build time, and a member's prose is its `about.md`.

| Path | Owner | Notes |
| --- | --- | --- |
| `src/data/core.json` | tool | regenerated on every sync |
| `config/novis.tmLanguage.json` | tool | rendered from the editors' grammar on every sync — edit `../editors/vscode/syntaxes/nvs.tmLanguage.json` |
| `src/data/core-changelog.json` | human | per-member changelog entries |
| `src/pages/reference/core/[...slug].astro` | human | the one route that renders every Core class and member page |
| `src/content/docs/reference/core/index.mdx` | human | the Core reference's landing page |
| `../docs/examples/**` | the proofs sweep | each member's `about.md` and examples, read in place by `src/lib/feature.ts`; the site holds no copy. `bun nv proofs --run` runs each example and diffs it against its `.out` file |
| `config/spec-overrides.mjs` | human | corrections for spec table rows the renderer cannot read — every fix goes here, never into the renderer |
| `config/site.mjs` | human | **all placeholder URLs live here** — swap them once to go live. It also holds `AREAS`, the four areas the header, the footer, the sidebars and `llms.txt` are built from |
| `config/external-links.mjs` | human | the rule that a link off the site opens in a new tab with `rel="noopener noreferrer nofollow"` — except the project's own repository and Discord links, which open in a new tab with no `rel`: an integration decorates content links at build time, and a component spreads `externalLinkAttrs(href)` onto any anchor it writes itself |


## How the Core reference works

`bun nv render --website` writes it (`../tools/nv/renderers/website-core.ts`).

1. The member tables of `../docs/spec/01-core-library.md` — the file the repo declares
   authoritative for the designed surface — are the `../data/spec/core-members.json`
   record. Well-formed member rows become full member records (signature, parameters,
   options, return type, PHP "Replaces" list, taint classification). The chapter's
   prose gives each class its summary, enums and constants. A row the renderer cannot
   read is a **render warning**, fixed in `config/spec-overrides.mjs`.
2. The built `nvs` binary's registry (`nvs meta --json`, ADR 0117) says what is
   implemented, and **every registered member is published, and nothing else**: a
   member the registry does not hold gets no page, and a class with no registered member
   does not appear at all. A registered class or member the spec's tables do not name is
   built from the registry's own signature, so the published set is the proofs roster's
   Core features. With no binary built, the render stops.
3. The same registry carries each member's card: short description,
   parameter/shape-key/return/error descriptions, authored next to the Rust
   implementation. Precedence is **field-wise**: a doc field the registry carries wins,
   one it lacks falls back to the spec, so documentation moves member by member.
4. `src/pages/reference/core/[...slug].astro` renders a page per published class and
   member. A member page is its signature, its `about.md` (or the card's short
   description when it has none), then Parameters, Options, Return value, Errors,
   Changelog, Examples and Related. A section with nothing in it is left out, heading
   included. The last line of `about.md` may be `related: Core\Bytes::length, …`: the
   names become the Related links, and a name with no page fails the build. Each
   member's `examples` field in `core.json` is its feature path in the proofs roster,
   which is the directory its `about.md` and examples are read from.

Novis code blocks get syntax highlighting from `config/novis.tmLanguage.json`
(languages `novis` / `nvs` in fenced code blocks). `sync:render` writes it from the editors'
grammar at `../editors/vscode/syntaxes/nvs.tmLanguage.json`
(`../tools/nv/renderers/website-grammar.ts`). The one difference is the top level: a fence is a
snippet with no `<?nvs`, so it starts in code mode where a file starts in text mode.

## How the site is laid out

Four areas, in the order a reader learns them: Guides (`/guides/`), Syntax (`/syntax/`),
Reference (`/reference/`) and In-Depth (`/in-depth/`), plus Install (`/install/`). Each
has its own sidebar: `astro.config.ts` holds one top-level sidebar group per area, and
`src/routeData.ts` shows a page only its own area's group. A page outside every area has
no sidebar. There are no previous and next links. The site does not publish the rulebook
or the decision records; a page that wants the reasoning links to the record on GitHub
(`config/site.mjs` § `decisionRecord`). A removed page simply stops existing, with no
redirect.

## Machine-facing surface

- `/sitemap-index.xml` — generated by Starlight on every build (needs `SITE_URL`).
- `/llms.txt` — every page with its description, for agents (`src/pages/llms.txt.ts`).
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
4. Set the Pages source to **GitHub Actions** and the custom domain to `SITE_URL`'s host. No
   `CNAME` file: GitHub writes one only when publishing from a branch, and ignores any that exists
   when a workflow publishes — one in `public/` would ship as a dead file at `/CNAME`.
5. `npm run sync`, review what it wrote, and commit it — publishing is the push, not a command.

## How it is published

`.github/workflows/pages.yml` builds this folder on every push to `main` and hands `dist/` to
GitHub Pages as an artifact. Nothing built is ever committed: `dist/` is gitignored, there is no
`gh-pages` branch, and the deployed site is the build of the commit it came from.

The workflow runs `npm ci && npm run build` — **not** `npm run sync`. Sync rewrites tracked files
and reads the built `nvs` binary for registry docs, so a deploy that ran it would publish less than
your local sync does and would decide page ownership with nobody looking. The site therefore
follows the repository only as far as the last committed sync: after landing spec or
registry changes, run `npm run sync` and commit what it wrote.

The same workflow builds on pull requests without deploying, so a build that no longer compiles
fails there rather than on `main`.
