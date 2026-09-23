- **A new rule's `because` may name only records whose own `changes:` block names that rule, so
  citing an older record as background fails `python tools/records.py --check`.** A rule created by
  0162 and listing `["0162", "0051"]` because 0051 placed the class reads as good provenance and is
  refused with `0051.md:3  <rule>'s because names 0051, but its changes: does not name the rule` —
  the two halves are one relation and a frozen record cannot grow a new entry. Cite background
  records in the fragment's prose, and keep `because` to the records that created or amended the
  rule. [until: reviewed 2026-09-09]
