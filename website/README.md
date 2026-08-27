# The Novis website

The public site at **novis-lang.org**: a landing page, the pages that explain
what Novis is for, the full documentation, and every design decision.

Everything about the website lives in this directory. It has its own toolchain,
its own tests, its own CI job and its own rules, and it is deliberately isolated
from the language workspace: **nothing outside `website/` needs to know it
exists**, and no language change can be blocked or slowed by it.

## One command

```sh
python site.py dev      # http://localhost:4321, live reload
```

That is the whole workflow. It installs dependencies if they are missing,
regenerates everything derived from the repository, and starts the server.

| Command | What it does |
|---|---|
| `python site.py dev` | Dev server with live reload |
| `python site.py build` | Produce `dist/` |
| `python site.py preview` | Serve `dist/` as it will be served in production |
| `python site.py sync` | Regenerate derived content (`dev` and `build` do this for you) |
| `python site.py check` | Provenance, page shape, edit locks, generated freshness |
| `python site.py draft PATH --from adr/0033-….md` | Scaffold a new page from its sources |
| `python site.py bless PATH` | Re-stamp a page's sources after rereading it |
| `python site.py assemble` | Build every published version into one tree (CI) |
| `python site.py clean` | Remove build output and generated content |

**Requirements:** Node 20.19+ (the repo develops against 24) and Python 3.11+.
Nothing else. No global installs, no Rust toolchain — though the syntax
highlighting is generated from `crates/nvs-syntax`, so a checkout without it
builds with plain code blocks and says so.

### Working offline

The first `python site.py dev` runs `npm ci`. That is the only step that touches
the network, and it happens once.

After that, **every build is fully offline**: search is indexed locally by
Pagefind, type is the system font stack, there is no CDN reference anywhere, and
Astro telemetry is off. A checked-out branch and a populated `node_modules` is
all a developer needs.

## How content works

This is the important part, and it is one idea:

> **A fact lives in exactly one place in this repository. The website changes
> its *shape*, never its *content*.**

The repository's own documentation is written for people building the language:
precise, dense, and assuming you want the argument rather than the summary. A
public reader needs something else — short sentences, the point first, English
that a non-native speaker can follow. Those are different *shapes* of the same
facts, not different facts.

Content on this site is therefore one of three kinds, and knowing which kind a
file is tells you who may edit it.

### 1. Mirrored — generated, never edited

`docs/adr/` is published verbatim under `/design/`. The ADRs already contain
every decision, every rejected alternative and every measurement; restating that
in public prose would create a second home for all of it. So they are published
as they were written, and the guides *link* to them.

`tools/sync_adr.py` changes only the shape: it lifts the H1 into frontmatter,
writes a description from the `In short` block, rewrites the links, escapes
`array<T>` so markdown does not eat it, retags `php` code fences as `nvs`, and
prepends a note saying the page is a mirror.

Output lands in `src/content/docs/design/` and is **gitignored**. If you want to
change one of those pages, edit the ADR.

### 2. Derived — generated from code and plans

| Generated | From | Used by |
|---|---|---|
| `src/grammars/nvs.tmLanguage.json` | the keyword table in `crates/nvs-syntax/src/token.rs` | syntax highlighting |
| `src/generated/status.json` | the status block in `docs/implementation-plan.md` | the status page and the pre-alpha banner |

A keyword added to the lexer highlights correctly on this site the same day,
because there is no second list to remember. `python site.py check` fails if a
generated file no longer matches its source.

Code examples work the same way: `<Snippet file="examples/hello.nvs" />` reads
the real file from the repository at build time, and the build fails if it moves.
An example on this site is a file the Rust test suite already runs.

### 3. Authored — written by a person or an agent, protected from both

The landing page, the "Why Novis" pages and the documentation guides are new
writing. They are not derived from anything mechanically — but every fact in
them comes from somewhere, and that somewhere is declared:

```yaml
---
title: Isolated scripts
edit: review
sources:
  - adr/0006-isolated-script-execution.md@6d40e6e4d61a
  - adr/0005-config-changeability.md@ff01e2af6b7f
---
```

The hash after `@` is the git blob hash of that source when a human last read
this page. When the ADR changes, `python site.py check` reports the page as
stale by name. Nobody has to notice the drift; the check notices.

`python site.py bless <page>` re-stamps it — and it is manual on purpose. A
stamp is a claim that somebody checked, and nothing automated is entitled to
make that claim.

## Editing generated and AI-written prose

An agent may draft and rewrite these pages. A human must always be able to
overrule it and have that stick. Two mechanisms, and they compose.

**Page level** — the `edit` frontmatter field:

| Value | Meaning |
|---|---|
| `open` | Tools and agents may rewrite the prose freely |
| `review` | They may propose; a human merges. The default once a human has read the page |
| `locked` | Nothing automated touches it again |

**Region level** — the one that actually gets used, because you usually want to
keep one hand-tuned paragraph rather than freeze a whole page:

```mdx
{/* keep: reworded 2026-08-28, the original lead lost people */}
An isolated script is another file you run as a job. It gets its own memory,
its own variables and only the permissions you give it.
{/* /keep */}
```

Everything between those markers is carried through verbatim by every tool that
rewrites the file. In a plain `.md` file the same markers are spelled
`<!-- keep -->` and `<!-- /keep -->`; both are understood everywhere.

If a regeneration produces fewer regions than the file had, the leftovers are
appended in a clearly marked block rather than dropped. Losing a person's
paragraph is the one failure this mechanism exists to prevent, so the tool would
rather leave a visible mess than be quietly tidy.

### Where AI fits, and where it does not

`python site.py draft` produces a skeleton: the frontmatter, the stamped
sources, the page shape, the ADR's own `In short` paragraph quoted as reference
material, and `TODO` markers where prose goes. An agent or a person fills it in
and commits the result as an ordinary reviewable file.

**No part of the build calls a model.** CI needs no API key, the output is
deterministic, and the only thing automated about the content pipeline is
*noticing* that a page has gone stale.

## The shape of a documentation page

Every page under `/docs/` is written in the same order, and `site.py check`
enforces the first two steps.

1. **Title.**
2. **Lead** — one or two sentences saying what the thing is and what it is for.
   No mechanism, no milestone, no decision record. A reader who does not speak
   English fluently must get the idea from this alone; if they do not, they
   leave. Written as `<Lead>…</Lead>`.
3. **Quick start** — the smallest example that does something real, then one
   sentence saying what happened.
4. **How it works** — the detail, with more examples. Technical framing is fine
   here; the reader who got this far wants it. Cross-link freely.
5. **In depth** — the complete rules, in as much detail as the feature has.
   Every form, every option, every edge case.

Link to the design decision for *why*. The page itself answers *what*.

[`docs/safety/isolated-scripts.mdx`](src/content/docs/docs/safety/isolated-scripts.mdx)
is the worked example; copy its structure.

## Versioning

Each documentation version is a **separate build of this same tree at a
different git ref**. `website/versions.config.json` names them:

```json
{
  "default": "main",
  "versions": [{ "label": "main", "ref": "main", "base": "/", "preview": true }]
}
```

`python site.py assemble` creates a git worktree per entry, builds each with its
own `SITE_BASE`, and copies the results into one deployable tree:

```
/            the default version
/main/       the development line
/v0.3/       a tag
/versions.json   what the switcher fetches at runtime
```

Nothing about a version is stored twice in the repository. `v0.3`'s
documentation is whatever `v0.3`'s tree says, because that is literally what
gets built. Cutting a release adds one line to the manifest; it copies nothing.

The switcher fetches `/versions.json` at runtime rather than baking the list in,
so a build cut a year ago still knows about versions released since.

## Design

Light is the resting state; dark is a switch, and a real design rather than an
inversion. The palette is in [`src/styles/tokens.css`](src/styles/tokens.css)
with its measured contrast ratios in the comments.

Four rules it is built to:

1. Body text clears WCAG AAA on its surface. Every other text token clears AA.
   Meaningful borders clear 3:1.
2. **Hue is never the only channel.** Every semantic colour is paired with an
   icon and a written label, because a reader with deuteranopia must be reading
   the word.
3. No gradients, no glow, no shadow as decoration. Structure comes from
   hairlines and whitespace, which survive being printed and being zoomed.
4. No web fonts. The system stack is fast, offline, and renders well everywhere.

## Layout

```
website/
  site.py                   the only entry point
  tools/                    modules it calls; each also runs standalone
    common.py               paths, frontmatter, keep regions, git hashes
    gen_grammar.py          keyword table  -> TextMate grammar
    gen_status.py           implementation plan -> status data
    sync_adr.py             docs/adr/      -> /design/
    check_pages.py          provenance, shape, locks, coupling
    draft.py                new page from its sources
    assemble.py             every version -> one deployable tree
  astro.config.ts           Starlight configuration
  versions.config.json      which refs are published
  src/
    site.config.ts          name, domain, extension, binary — one place
    content/docs/           the pages
      index.mdx               landing
      why/                    the pitch
      docs/                   the manual
      status/                 generated status
      design/                 GENERATED mirror of docs/adr — gitignored
    components/             Lead, Snippet, Sources, Pillars, StatusTable…
      overrides/            three Starlight component overrides
    styles/                 tokens.css, site.css
    grammars/               GENERATED — gitignored
    generated/              GENERATED — gitignored
  public/                   CNAME, robots.txt
```

## Known couplings

Two files reimplement Starlight internals, and both say so in their own header:

- `src/components/overrides/ThemeProvider.astro` — Starlight ships dark-first
  and follows the OS. This makes light the default for a visitor with no stored
  preference, while leaving the toggle exactly as it was.
- `src/components/overrides/Banner.astro` — Starlight's banner is per-page
  frontmatter; the pre-alpha notice is not a per-page fact.

`python site.py check` warns when the upstream contract has moved. If the theme
toggle ever breaks after an upgrade, diff those files against
`node_modules/@astrojs/starlight/components/`.

## What is deliberately not here yet

An **interactive playground**. Novis compiles to WebAssembly as a second
codegen backend, which is what would make one possible — and that backend does
not exist before milestone M14. Rather than ship a second implementation of the
language in JavaScript that would disagree with the real compiler, the site uses
static, highlighted, real code samples, and will grow a playground when there is
something honest to run.
