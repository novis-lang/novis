Every handwritten website page lists in `covers:` the proofs roster's ids of the features it explains,
and `website/site.lock.json` records the hash each feature had when the page was last reread against
it. The hash covers the feature's help card or reference section, its `about.md` and its examples.

A covered id the roster no longer has is **broken**. A covered feature whose hash moved since the
page's stamp is **stale**. `bun nv session --wrap` refuses every broken id, and every stale page the
session made stale itself: the session that changes a feature rereads the pages that cover it, fixes
them and runs `bun nv site --stamp`, the same way it owes the feature's proofs. CI's `proofs` job runs
`bun nv site --check` over the whole tree, so a stale page the wrap did not attribute is still caught
before a merge. A stamp is written by whoever reread the page, never to quiet the check.

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
