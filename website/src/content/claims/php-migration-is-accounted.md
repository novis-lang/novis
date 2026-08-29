---
claim: Every PHP function has a named outcome — replaced, rewritten or deliberately dropped
category: compatibility
comparedTo: [PHP]
proof: 'The migration spec is generated from two inputs that account for every PHP global function: what Novis has (each entry''s "Replaces" column) and what it dropped with the rewrite for it. "Did we lose something real?" is a question the table answers, not a discussion.'
tradeoff: 'Novis is not PHP-compatible at the source level: existing PHP code does not run unmodified, and features PHP relies on — `eval`, stream wrappers, magic methods, superglobals — are deliberately gone (ADR 0052 lists the closed doors).'
draft: true
weight: 10
---

Language migrations fail on the long tail: the five functions your codebase uses that
the new language forgot about. Novis attacks the tail head-on — the Core library was
designed by walking PHP's entire global function list and deciding each name's fate
explicitly. Every member page on this site shows the PHP names it replaces, and the
planned `nvs convert` tool applies the mechanical renames automatically.
