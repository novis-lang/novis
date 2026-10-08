Every handwritten website page lists in `covers:` the proofs roster's ids of the features it explains,
and `website/site.lock.json` records the hash each feature had when the page was last reread against
it. The hash covers the feature's help card or reference section, its `about.md` and its examples.

A covered id the roster no longer has is **broken**. A covered feature whose hash moved since the
page's stamp is **stale**. `bun nv session --wrap` refuses every broken id, and every stale page the
session made stale itself: the session that changes a feature rereads the pages that cover it, fixes
them and runs `bun nv site --stamp`, the same way it owes the feature's proofs. CI's `proofs` job runs
`bun nv site --check` over the whole tree, so a stale page the wrap did not attribute is still caught
before a merge.

A stamp is written by whoever reread the page, never to quiet the check. Every snippet a page shows
lives under `website/snippets/` beside exactly one of its expected output, a `.out` it must print with
exit 0, or its expected diagnostic, a `.err` that `nvs check` must report for a program that does not
compile. That same CI step checks every one, so no page carries Novis code that is not known to run,
and no compiler error a page shows differs from the one the compiler prints.
