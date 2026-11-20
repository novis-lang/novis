Every change to what a feature does reaches the text a user reads about it: the feature's `about.md`,
its help card, the website pages that cover it, and the release notes of the release that ships it.
The feature's **tests** are what say a behaviour changed, so they are part of the hash that makes
those texts stale, and an added, removed or changed feature owes an entry in
`website/src/data/changes.json`.

**A feature's tests** are the tests whose `covers:` marker names it, as `markersIn` finds them under
`MARKER_ROOTS`. A `.nvst` case is its whole file. A Rust `#[test]` and a package's `#[Test]` method run
from the marker's line to the closing brace at the indent of their `fn` or `function` line, so an edit
to another test in the same file moves nothing. A case credited only because it calls
`Class::member(` is not in the hash: it usually tests something else.

**Four hashes**, each over what decides one verdict, all in `tools/nv/lib/texts.ts`:

| Hash | Over | Decides |
|---|---|---|
| page hash | the card, `about.md`, every file of the example directory, and the feature's tests | a handwritten page's stamp (`rule:testing/website-pages-are-stamped`) |
| text hash | the card, `about.md` and the example directory | a page's search questions |
| `about.md` stamp, help stamp | the page hash without that text itself | whether that text is stale |
| behaviour hash | the feature's tests alone | `--check changes` |

The questions keep the text hash because they are search phrases, not a description of behaviour. An
`about.md` stamp leaves out `about.md`, and a help stamp leaves out the card, so rewriting a text never
makes that same text stale; rewriting `about.md` still makes every covering page stale. The behaviour
hash is the tests alone, so a rewritten description or example owes no `changes.json` entry. Every
kind of feature owes a test (`rule:testing/feature-proofs`), so a finished feature has one to move.

**The lock**, `website/site.lock.json`, keeps its shape, `key -> { feature id -> hash }`. A page is
keyed by its path under `website/src/content/docs/`, an `about.md` by its path
(`docs/examples/core/Str/length/about.md`), and a help card by `help:<feature id>`. A language feature
has no help key, because its reference section is written from the files the hash already reads. A
feature with no `about.md` has no about key: the Description proof owes that file. `bun nv site
--check stale` names a stale or never-stamped `about.md` or help card beside the stale pages, and
`bun nv site --stamp` takes all three kinds of key. A stamp is written by whoever reread the text
against the change, never to quiet the check. A test-only edit, such as a rename, makes the texts stale
too, and there is no exception list for it: the reread is cheap and a list would grow.

`bun nv site --stamp --baseline` ran once, when the tests joined the hash. It stamped every
`about.md` and help card, and restamped every page whose stamp equalled the text hash, a stamp that was
current before. It refuses to run while the lock holds any `about.md` or `help:` key, so it cannot quiet
a later change.

**`changes.json`** holds the entries not yet released: `{ "entries": [...], "unseen": {...} }`.

- An entry has a `kind` (`new`, `changed`, `fixed`, `faster` or `removed`), `features` (one or more
  roster ids), and a `text` of one to three sentences in the voice of AGENTS.md § *Text an end user
  reads*, naming each feature in backticks.
- `instead`, what to write instead, is required on a `removed` entry and on one with `breaking: true`.
  `highlight: true` puts an entry first in its section. `security: true` puts it under *Security
  fixes*, and its `text` says what an attacker could do before the fix.
- A `faster`, `highlight` or `security` entry may name a feature whose hash did not move: a speed-up or
  a security fix does not always change a covering test.
- `unseen` names a feature whose behaviour hash moved while nothing a user sees changed, such as a
  renamed test, with the hash it was written at and why. It clears the feature only at that hash, the
  release empties it, and it is never used for a change a user can see.
- A speed-up whose change added or moved a test gets a `faster` entry and an `unseen` record whose
  `why` says the results are the same. A `faster` entry never clears a moved hash, so the check stays
  strict about results, and a speed-up that also changes a result is a `changed` entry.
- A configuration key is not on the roster, so a renamed key is named in the text of the entry for the
  feature it serves.

**The snapshot**, `data/proofs/released.json`, is `{ "since": "<tag>", "features": { id -> behaviour
hash } }`. `bun nv site --check changes` compares it with the roster now: a feature not in it needs a
`new` entry, a feature in it that the roster no longer has a `removed` entry, and a feature whose
behaviour hash moved a `changed` or `fixed` entry or a current `unseen` record. It also fails on an entry that
breaks the shape above, on prose problems in its text, and on an id neither side has. The snapshot is a
file, not a build of the tag, because the roster comes from `nvs meta --json` and a check cannot build
an old binary.

**The release step.** `bun nv release --notes <version>` starts its notes with the entries, in the six
sections of `docs/agent/changelog-summary.md`: *Security fixes*, *Breaking changes*, *New*, *Faster*,
*Fixes*, *Internal changes*. A `security` entry goes under *Security fixes*; otherwise a `removed` or
`breaking` entry under *Breaking changes*, `new` and `changed` under *New*, `faster` under *Faster* and
`fixed` under *Fixes*. *Internal changes* counts the commit types no entry covers, and the commit log
follows under *Details*. The same text is the GitHub release body and what is prepended to
`CHANGELOG.md`. Then `bun nv site --changes --release <version>` adds each entry to the changelog of
every feature it names (`website/src/data/core-changelog.json`), empties `entries` and `unseen`, and
writes a new snapshot named after the version. `.github/workflows/release.yml` runs it after the notes
are rendered.

The generated reference half of the site cannot drift from the binary, but a description, a help card
and a guide can, and nothing else connects them to the behaviour they explain. A test is the one place
a behaviour change is always written, so hashing the tests makes exactly the texts about a changed
feature stale, in the session that holds the context to fix them. A release note written from entries
says what a user can now do, in a user's words, at the time the change is made.

**Cost.** Nothing at run time and nothing per request. `bun nv site --check stale` scans the
`covers:` markers once per run, as `bun nv proofs` does. A test-only edit costs a reread of the
feature's `about.md`, help card and covering pages, and an `unseen` record. The snapshot is one JSON
file of one hash per roster feature.

**Declined:**

- *A periodic review pass over every text.* It finds drift late, and never says which change caused it.
- *A check of the wording by word lists.* AGENTS.md § *Text an end user reads* says nothing checks the
  wording, and a word check was declined before: it passes stiff text and fails plain text.
- *A changelog generated from commit messages.* A commit subject is written for contributors, not for a
  user, and says what is true in the tree, not what a user can now do.
