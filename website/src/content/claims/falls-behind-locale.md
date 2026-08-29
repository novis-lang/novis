---
claim: "No ambient locale: locale-aware formatting is more work than in PHP"
category: honesty
comparedTo: [PHP]
proof: 'ADR 0051: Novis has no `setlocale` and no ambient locale state, so nothing formats "in the user''s language" implicitly.'
draft: true
weight: 40
---

`setlocale` plus locale-aware `strcoll`, `strftime` and `number_format` make quick
locale-sensitive output easy in PHP — and also make it a per-process global that changes
the behaviour of unrelated code, which is why Novis refuses the mechanism. The honest
consequence: until Novis's intl tier ships, locale-correct collation and date names in
arbitrary languages are simply not available, and explicit-argument formatting is more
verbose than PHP's one-liner.
