Every handwritten website page lists in `covers:` the proofs roster's ids of the features it explains,
and `website/site.lock.json` records the hash each feature had when the page was last reread against
it. The hash covers the feature's help card or reference section, its `about.md` and its examples.

A covered id the roster no longer has is **broken**. A covered feature whose hash moved since the
page's stamp is **stale**. `bun nv session --wrap` refuses every broken id, and every stale page the
session made stale itself: the session that changes a feature rereads the pages that cover it, fixes
them and runs `bun nv site --stamp`, the same way it owes the feature's proofs. CI's `proofs` job runs
`bun nv site --check` over the whole tree, so a stale page the wrap did not attribute is still caught
before a merge. A stamp is written by whoever reread the page, never to quiet the check.

The generated half of the site, the reference, is rewritten from the binary and the roster on every
render and cannot drift; the handwritten half can, because nothing else connects a guide to the
behaviour it explains. A hash makes exactly the pages that cover a changed feature stale, and the
session that changed it is the one holding the context to reread them. `covers:` uses the roster's
ids, so a page and a feature proof name a feature the same way. The cost is that a change to a
feature costs its author a reread of the pages that cover it. `bun nv site` is one tool that holds the
guard, the stamp, the build and every `--check` part, in place of a script per check, and its parts
run in the CI job that has already built `nvs`, because the snippets and the roster both need it.

**A page's search questions are stamped as the page is.** Each entry of `website/src/data/questions.json`
records the feature its questions were written from and that feature's hash, and an entry whose hash
moved is stale. `bun nv site --check questions` fails on every stale or broken entry and on every
built page with no entry, so a new page fails CI until it has questions. It also holds each entry to
four to six questions of at most 15 words, in the words of § *Text an end user reads*, with no question
two pages share. `bun nv site --questions --stamp` is run by whoever rewrote or reread them. The
questions are not a feature proof.

Every snippet a page shows lives under `website/snippets/`, and that same CI step checks each file
beside it against the real binary. A `.err` is the diagnostic `nvs check` must report for a program that
does not compile. A `.out` is what `nvs run` must print with exit 0. A `.http.out` is the response body
a real `nvs serve` sends for the request the snippet's `.nvsr` describes, or for a plain `GET` when it
has none; no simulated request stands in for a server. A snippet with a `.nvsr` always has a `.http.out`.

**Where the command line and a server print different things, the page shows both**, the command line
first. `echo` writes to a different sink in each (`rule:tooling/echo-always-has-a-sink`), and in a
response it escapes markup characters (`rule:core-classes/html-auto-escape`), so a program that echoes
`<`, `&` or a quote prints one text and sends another. The check runs every snippet that compiles in
both contexts, and where both apply and differ it requires both files; where they agree it requires
one and refuses the second. **The front page shows only what the server sends**, with `<Snippet
output="http">`, because a web server is what a visitor first comes to Novis for. So no page carries
Novis code that is not known to run, and no output a page shows differs from what the binary prints in
the context the page names.

**Declined:**
- **Staleness by a "last reviewed" date.** A date says nothing about whether the feature moved since.
- **A periodic review pass.** It finds drift late, and never says which page a change reached.
