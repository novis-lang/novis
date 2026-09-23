- **A `.nvs` proof that measures bytes and then slices them hits the `int`/`uint` boundary, and `.`
  does not join two `bytes` at all.** `Core\Bytes::length` and `::at` answer `uint` while `::slice`
  takes `int` offsets, so `slice($b, 0, length($b) - 1)` is `E0401`, and `$a . $b` is `E0707`
  because `bytes` has no string form. Cast at the call — `(Core\Bytes::length($b) - 1) as int` —
  and join with `Core\Bytes::join([...], "" as bytes)`, whose separator is its second argument.
  [until: reviewed 2026-09-21]
