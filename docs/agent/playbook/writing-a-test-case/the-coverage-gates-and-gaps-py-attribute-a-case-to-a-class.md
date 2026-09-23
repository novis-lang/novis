- **The coverage gates and `gaps.py` attribute a case to a class textually, so a new return variant
  or call spelling drops it.** `Attribution::new` reads `return_ty:`, its `arrows` matches
  `->member(`, `tools/gaps.py`'s `RETURNS_RE` is the same regex, and `conformance_coverage.rs`
  spells a call `Class::name(`. Teach every one the variant, and spell `return_ty:` inline; a named
  `const` is invisible to the regex. [until: reviewed 2026-09-06]
