# Working on the Novis website

**This file applies only inside `website/`.** The repository root has its own
`AGENTS.md` governing the language workspace; it does not mention this directory
and does not need to. Neither governs the other, and a session should be working
on one or the other, not both.

Read [README.md](README.md) first — it is the full picture. This file is the
short list of things that will otherwise cost you a rebuild.

## The one rule everything else follows

**A fact lives in exactly one place in this repository. This site changes its
shape, never its content.**

Before you write a sentence stating a fact about Novis, find where that fact
already lives. If it is in an ADR, link to the ADR and say the same thing in
plainer words. If it is in code, generate it. If you find yourself typing a
keyword list, a limit, a version number or a rule that exists elsewhere — stop,
and derive it instead.

## Before you edit anything

1. `python site.py check` — tells you which pages are stale and why.
2. Look at the file's frontmatter `edit:` field.
   - `locked` — do not touch the prose. At all.
   - `review` — you may rewrite, but say in your summary what you changed.
   - `open` — go ahead.
3. Look for `{/* keep */}` … `{/* /keep */}` regions. **Never rewrite what is
   inside one, never reflow it, never "improve" it.** A person put it there
   because they were not happy with what a tool wrote.

## Never edit a generated file

These paths are produced by `python site.py sync` and are gitignored. Editing
one is silently undone on the next build:

| Path | Edit this instead |
|---|---|
| `src/content/docs/design/**` | the ADR under `docs/adr/` |
| `src/grammars/nvs.tmLanguage.json` | `crates/nvs-syntax/src/token.rs`, or `tools/gen_grammar.py` for the categories |
| `src/generated/status.json` | the status block in `docs/implementation-plan.md` |

Every generated file carries a header naming its source. If you are about to
edit a file that has one, you are in the wrong file.

## Writing rules

These are not style preferences. The first two are binding on every document
this project publishes, per [ADR 0080](../docs/adr/0080-the-audience-nvs-is-built-for.md):

1. **The pitch is isolation, qualifiers and uncoloured suspension.** Never
   "faster than PHP". Do not write a performance comparison with PHP anywhere on
   this site.
2. **PHP-shaped syntax is an on-ramp, never a compatibility promise**, and no
   page may imply otherwise. This includes small things: do not tag a code fence
   `php`, do not write "just like PHP", do not describe Novis code as "PHP with
   types".
3. **Write for a reader whose English is their second or third language.** Short
   sentences. Common words. One idea per sentence. No idiom, no metaphor that
   depends on English, no jokes. This is not dumbing down — it is the difference
   between a reader who continues and one who leaves.
4. **Say what a thing is before you say how it works.** The lead names the
   thing. The mechanism comes later.
5. **Be honest about what does not exist.** Novis is pre-alpha. A page that
   describes an unbuilt feature as though you could use it today costs more
   trust than it buys attention. Use the `<Unwritten>` component rather than
   filler.
6. **Do not restate reasoning.** The ADRs hold every rejected alternative and
   every measurement, and they are published. Link to them.

## The page shape

Every page under `/docs/` goes: **title, lead, quick start, how it works, in
depth.** `python site.py check` enforces the title and the lead. See
[`docs/safety/isolated-scripts.mdx`](src/content/docs/docs/safety/isolated-scripts.mdx)
for the worked example — copy its structure rather than inventing one.

## Links

**Use relative links in page content**, not root-absolute ones. Every version of
this site is served under a different base path (`/`, `/main/`, `/v0.3/`), and a
`/docs/…` link would drop the reader out of the version they were reading.
Sidebar entries in `astro.config.ts` are the exception: Starlight prefixes those
itself, so they are written with a leading slash.

## Adding a page

```sh
python site.py draft docs/language/enums \
    --title "Enums" \
    --from adr/0010-enums-are-a-value-type.md
```

That writes the frontmatter, stamps the sources, lays out the shape and quotes
the ADR's own summary as reference material. Fill in the prose, add the page to
the sidebar in `astro.config.ts`, then `python site.py check`.

## Finishing

```sh
python site.py check     # provenance, shape, locks, generated freshness
python site.py build     # the real gate: broken links and MDX failures
```

The build is what catches a broken internal link, a moved snippet source or an
MDX page that will not render. `check` catches everything else. Both are fast —
the whole site builds in about three seconds.

Commit `website/` changes separately from language changes. They are reviewed by
different people for different reasons, and the CI jobs are separate too.

## Do not touch

- Anything outside `website/`, except to *read* it. This subproject generates
  from `docs/adr/`, `docs/implementation-plan.md`, `docs/plan/`, `docs/setup.md`,
  `examples/` and `crates/nvs-syntax/` — and it only ever reads them.
- The root `AGENTS.md`, `CLAUDE.md`, or anything under `tools/` at the
  repository root. Those belong to the language workspace and adding website
  material to them would charge every language session with context it does not
  need.
