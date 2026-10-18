The registry is the only source of a `Core` member's surface and its reference documentation: its
signature, its parameter names and types, its qualifiers, its options and its card
(`rule:core-api/reference-card`). `nvs meta --json` publishes it, and every consumer — the website's
`Core` pages, `nvs help`, a hover, the one-file reference — reads that output and nothing else.

A documentation field the registry has not written is **absent**, never filled from somewhere else. The
JSON leaves the key out, so a consumer tells "not written yet" from "written, and empty" without learning a
convention, and shows the field as missing. Each field stands alone: a member whose card has a short
description and no parameter text shows the one and not the other, so a card is filled in one field at a
time and no consumer waits on a whole card. The same reading keeps a consumer correct against an older
toolchain whose rows carry fewer fields.

A class's introduction page under `docs/reference/core/` or `docs/reference/novis/` is prose beside the
card, never a second copy of anything the registry carries.

**Declined:**
- **A design document beside the registry that lists every member and is authoritative for the ones not
  implemented yet.** Two descriptions of one member drift, and a consumer has to choose between them
  member by member. Nothing in such a list ever designed a member the registry lacked, so it only
  restated the registry.
