- **A proof over a `Core\Compress` stream cannot compare its result with `Core\Hash::equals`,
  because `finish` answers `tainted bytes`.** The decompressing stream's `finish` is tainted
  unconditionally (`rule:security/tainted-sources`), so an example checking a stream against
  `Core\Compress::decompress` fails to compile at the *comparison*, and the diagnostic names the
  argument rather than the stream. Declare the result `tainted bytes` and compare the two lengths,
  or compare frames the compressing half produced, whose `finish` is plain `bytes`.
  [until: reviewed 2026-09-21]
