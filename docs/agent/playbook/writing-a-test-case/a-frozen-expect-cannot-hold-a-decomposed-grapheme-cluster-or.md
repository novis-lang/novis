- **A frozen `--EXPECT--` cannot hold a decomposed grapheme cluster or an invisible byte, and
  nothing warns you.** `"cafe\u{0301}"` sliced at its last cluster renders like the precomposed `é`,
  and a refusal that quotes its subject unescaped (`Core\Encoding::encodeText`) puts a raw C1
  control in the expectation; either fails with two halves that look the same. Echo
  `Core\Encoding::toHex($s as bytes)` for any cell that is not plainly ASCII.
  [until: reviewed 2026-09-06]
