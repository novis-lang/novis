- **Dividing a `uint` makes the value `mixed`, so a chained bench input that uses `/` does not
  compile at all.** `/` on two `uint`s is `uint|float` and a `%` over that is `mixed`, which every
  `uint` parameter refuses with `E0401` — and it lands exactly where `benches/members/README.md`
  tells you to chain iteration N's input to N−1's result. Chain with `*` and `%` only
  (`($total * 7) % 256`), and cast where a member wants the other width
  (`Core\Arr::slice($files, $done as int)`). [until: reviewed 2026-09-21]
