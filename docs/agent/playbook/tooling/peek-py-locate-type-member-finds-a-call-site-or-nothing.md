- **`peek.py --locate Type::member` finds a call site or nothing, never the definition, when the
  member is on an inherent `impl`.** A member is written `fn of(` under `impl Registry`, so the
  qualified spelling only exists where somebody *calls* it: `--locate Registry::of` answered with a
  line inside that file's own tests and `Registry::request` with `NOT FOUND`, while both were defined
  and public three hundred lines above. Locate the type instead, or take the file's outline with
  `grep -n '^\s*pub fn ' <file>` and read the region. [until: gone tools/peek.py:--locate]
