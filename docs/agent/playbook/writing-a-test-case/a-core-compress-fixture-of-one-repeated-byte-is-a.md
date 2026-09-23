- **A `Core\Compress` fixture of one repeated byte is a decompression bomb by the class's own
  measure, so the *honest* half of a bound test is refused too.** Every codec here takes a buffer of
  one byte past `DEFAULT_MAX_RATIO`'s 1000:1 — 16 MiB of `A` comes back at 1027:1 — so a payload
  written to be easy to build fails the roomy call as well as the tight one, and reads as the member
  ignoring its defaults. Build the payload out of English text repeated, the way the `.nvst` cases in
  `tests/conformance/core/compress-*` already do. [until: reviewed 2026-09-21]
