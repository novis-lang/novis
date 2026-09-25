- **The coverage gates and `bun nv gaps` attribute a case to a class textually, so a new return
  variant or call spelling drops it.** `Attribution::new` reads `return_ty:`, its `arrows` matches
  `->member(`, `tools/nv/cmd/gaps.ts`'s `RETURNS_RE` is the same regex, and
  `conformance_coverage.rs` spells a call `Class::name(`. Teach every one the variant, and spell
  `return_ty:` inline; a named `const` is invisible to the regex.
  [until: gone tools/nv/cmd/gaps.ts:RETURNS_RE]
