- **`bun nv gaps`'s two sections measure two different suites, and an item that mixes them asks for a
  case that already exists.** The *conformance depth by class* block is `tests/conformance/`; the
  *differential gap* block is `tests/differential/`, and a member at the depth floor in the first
  may already have oracle cases in the second. Only the differential block's roster ("a PHP twin and
  no oracle case") answers a differential item; one `ls tests/differential/core/ | grep <class>`
  settles it. [until: gone tools/nv/cmd/gaps.ts]
