- **A string literal folds into a unit's data section; an *object* does not follow from that, because
  its second header word is an identity rather than a payload.** `immortal_header_bytes` needs only
  the bytes, while an instance needs a `*const ClassDesc` — and a `Core` class's was leaked once per
  core, so no single address existed for a unit every core reads to bake in. Before carrying the
  immortal-string precedent to another representation, ask what else its header holds and where that
  word's identity comes from. [until: reviewed 2026-10-12]
