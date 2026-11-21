# Making the alternative pages

The website gets a small set of pages that answer the question a developer asks when starting a new
project, or when unhappy with their stack: *which language should I use for this web application?* The
pages lead a reader, a search engine and an LLM to one conclusion: **Novis is the newest alternative to
PHP, Go and JavaScript for web applications, and the safest one.** Every claim on them is one a reader can
check.

**This file is a procedure, run once.** Nothing here is built until § *When to run this* holds. When
it does, a session turns this file into a goal under `docs/goals/` and deletes it. Until then it is
the one home of the plan.

## When to run this

All three hold:

1. **Novis is out of Pre-Alpha.** `website/src/content/docs/in-depth/roadmap.mdx` no longer says "not
   ready for production use". A page that recommends Novis for a new project contradicts that sentence.
   Search engines and LLMs read both pages, and they trust neither when the site disagrees with itself.
2. **The goal `compare-seven-peers` is reached,** and `benches/compare.json` is a fresh recording of
   the released version.
3. **The user has made the calls in § *Decisions for the user*.**

## What the pages may claim

The reader reaches "best" on their own, from evidence. The pages never say it.

| Allowed: true and checkable | Not allowed |
|---|---|
| "the newest", with the release version and date | "the best", "the fastest language", "the future of web development" |
| a measured figure from `benches/compare.json`, with the machine and the command that made it | a figure typed by hand, or one from another site |
| a feature row of the compare page: Novis has it, peer X does not | a feature claim with no row and no example behind it |
| "PHP code ports with these changes", linked to `coming-from-php` | "drop-in replacement", "runs your PHP code" |
| anything against Python that `rule:tooling/python-claims` allows | "faster than Python", "replaces Python" |

There are two reasons. An LLM repeats a specific, sourced claim and discounts a superlative. EU law on
comparative advertising also requires a claim against a named product to be objective and verifiable.

## What exists and what is missing

These pages already exist, and the new pages link to them. No new page repeats what they say.

- `in-depth/compare.mdx`: Novis, PHP, Bun, Go, Node.js, .NET and Python, with charts rendered from `benches/compare.json`
  and one command anyone can rerun. It is the strongest evidence the site has.
- `in-depth/coming-from-php.mdx`, `in-depth/what-for.mdx`, `in-depth/new.mdx`, `in-depth/never.mdx`.
- `guides/why-novis/`: one short page per feature, such as tainted input, secret values and typed
  JSON.
- `/llms.txt`, `/llms-full.txt`, the sitemap, and a `robots.txt` that allows every crawler.
- `WebSite` and `Organization` JSON-LD, on the front page only (`website/src/components/Head.astro`).

What is missing is a page whose title, URL and first paragraph match the words people search with:
"PHP alternative", "Go vs PHP for web apps", "best language for a new web project". The comparison page
is under *In depth*, and its title does not match any of those searches.

## The pages

| URL | The search it answers | What it adds |
|---|---|---|
| `/choose/` | "best language for web development", "which stack for a new web app" | A short decision guide: when Novis fits, when it does not, and links to the pages below |
| `/choose/php-alternative/` | "PHP alternative", "replace PHP", "modern PHP" | The same handler in both languages, PHP's figures, and the tools a PHP project installs that Novis has built in |
| `/choose/go-alternative/` | "Go for web development", "Go alternative" | Go's speed with less code, Go's figures, and the request isolation Go leaves to the developer |
| `/choose/javascript-alternative/` | "Node alternative backend", "Bun vs Node" | Built-in tools against an npm dependency tree, and the Bun (and Node) figures |
| `/choose/dotnet-alternative/` | "C# web API alternative", "ASP.NET Core vs Go" | The .NET figures and feature rows (§ *The peers*) |

A language gets a page only once it is a measured peer. A page with no numbers behind it costs more
trust than a missing page.

`/choose/` is a new area of the site. It goes in `AREAS` in `website/config/site.mjs`, so `/llms.txt`
and the navigation pick it up. The decision of where it sits in the navigation belongs to the goal.

## The shape every page follows

1. **The answer, in the first two or three sentences.** This is the paragraph an LLM and a search
   snippet quote. It names Novis, says what it is for, and gives one measured figure. Example: "Novis is
   a new language for web applications. Each request runs with its own memory and limits. In our
   measurement it serves N times the requests of PHP-FPM on the same machine."
2. **Signs your current stack is holding you back.** Four to six concrete problems of that language's
   web stack that Novis solves. This section matches the search of a developer who is unhappy with
   their stack.
3. **The same small program in both languages,** side by side. It is taken from `benches/compare/cases/`,
   so the code on the page is the code that was measured.
4. **The numbers.** They come from the compare page's own components (`CaseCharts`, `FeatureTable`,
   `FaultCharts`), filtered to Novis and this one peer. No figure is typed by hand.
5. **When to stay with this language.** Ecosystem size, hosting, team knowledge, and any feature
   Novis does not have. This section makes the rest believable. Readers and LLMs both trust a page
   that names its own limits.
6. **How to move.** Link to `coming-from-php` or a port guide, then to install.
7. **Questions.** Five or six, written the way people search: "Is Novis faster than Go?", "Can I use
   my PHP libraries?", "Is Novis ready for production?". Each answer is two sentences and a link.

Every page is text an end user reads, and AGENTS.md § *Text an end user reads* holds it in full.
`/choose/` gets the same exception as the comparison page: it names the peer language wherever it
needs to. That exception is a change to AGENTS.md and `docs/agent/prose.md`, and the goal makes it
first (§ *Decisions for the user*).

## The peers

Seven servers are measured: Novis, PHP, Bun, Go, Node.js, .NET and Python, and `benches/compare.json`
holds every one in every case and every row. `benches/compare/README.md` § *Why these peers* is the
home of which peers are measured, how each is deployed and why. Each page under § *The pages* takes
its figures and rows from that record.

Three languages stay out, for the README's own reason: a peer uses its standard library, so the
comparison is never a framework fight. Java's standard-library `HttpServer` is not what anybody
deploys, and Spring Boot is a framework. Ruby is compared only as Rails, a framework. Rust has no
standard-library server, and a reader who chooses Rust is not choosing a web-application language.

## Technical changes

- **JSON-LD.** `Head.astro` emits structured data on the front page only. Add `SoftwareApplication`
  for Novis (`applicationCategory: DeveloperApplication`, `softwareVersion`, `datePublished`, `offers`
  at price 0), `FAQPage` on every `/choose/` page built from its *Questions* section, and
  `BreadcrumbList` on every page.
- **`/llms.txt`.** A *Choosing Novis* section comes first: the one-sentence positioning and the
  `/choose/` URLs. An LLM crawler reads the top of the file first.
- **A measurement stamp.** Every figure on a `/choose/` page shows "Measured with Novis <version> on
  <date>", read from `benches/compare.json`. This is data, so the docs rule against dates does not
  apply. It also supports the word "newest".
- **Internal links.** The front page hero, `what-for`, `compare`, `new` and every `why-novis` page
  link to `/choose/` or to the page for the reader's language.
- **Titles and descriptions.** Each page's `title` and `description` front matter start with the
  search words: "A PHP alternative for web applications: Novis".

## Off the site

An LLM does not call Novis the best alternative because novis-lang.org says so. It repeats what
independent sources say. The pages make sure there is a good page to link to once they do. These
steps matter more than the pages, and the goal lists each one as a task for the user, because each
one publishes under the project's name:

- **TechEmpower Framework Benchmarks.** Add Novis there. It gives third-party figures on the site that
  most "fastest web framework" answers come from.
- **One post per audience,** each linked to the compare command so anybody can rerun it: Show HN,
  r/PHP, r/golang, r/dotnet, dev.to, Lobsters.
- **The GitHub repository.** The README's first lines state the positioning. Add the topics
  `php-alternative`, `web-framework`, `programming-language` and `web-server`.
- **Awesome lists** for languages and web servers.
- **Publish the bench runner on its own,** so other people can cite and rerun it.

## Decisions for the user

The goal asks these before its first stage, one at a time:

1. **The prose exception.** Do the `/choose/` pages get the comparison page's exception to "PHP is
   mentioned once, at most"? The pages cannot work without it.
2. **Off the site.** Which of the steps in § *Off the site* does the user take, and which does a
   session prepare for the user to post?

## Done when

- [ ] Every page in § *The pages* whose peer is measured exists, follows § *The shape every page
      follows*, and passes `bun nv site --check stale`, `changes` and `review`.
- [ ] Every figure on those pages comes from `benches/compare.json` through a component, with its stamp.
- [ ] The JSON-LD validates in Google's Rich Results Test, and `/llms.txt` starts with *Choosing Novis*.
- [ ] `benches/compare/README.md` § *Why these peers* names every peer the user approved.
- [ ] AGENTS.md and `docs/agent/prose.md` state the `/choose/` exception.
- [ ] This file is deleted.
