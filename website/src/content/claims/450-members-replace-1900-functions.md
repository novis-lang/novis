---
claim: ~450 Core members replace PHP's ~1,900 global functions — without covering less
category: simplicity
comparedTo: [PHP]
proof: 'The Core library spec''s own counting table: 11 sort functions + `array_multisort` become 2 members; 12 `array_diff`/`array_intersect` variants become 2; ~40 date functions and 2 class trees become 1 immutable object tree. The library covers strictly more than PHP''s — an HTTP client, SMTP, UUIDs, CSV and a test surface are built in.'
tradeoff: 'Migration renames essentially everything. The rewrite is mechanical (each entry names the PHP functions it replaces, and the migration table accounts for every PHP name), but a Novis codebase does not look like a PHP codebase.'
draft: true
weight: 10
---

The reduction is entirely in restatements: PHP grew twelve spellings of "find something
in a string" and five array-combining rules chosen by a key's type. Novis keeps one of
each, names it predictably (`Core\Str::indexOf`, subject first, options last), and every
member page on this site lists exactly which PHP functions it replaces — so the knowledge
transfers instead of being lost.
