- **A bench that picks its chained input out of an array measures the array too.**
  `benches/members/core/Bytes/join.nvs` read its separator as `$separators[$total % 3]`, and a
  packed list synthesizes a key on every read, so the first figure was 8.0 allocations at 168.6
  ns/op where the member's own work is 6.5 at 130.4. Chain through locals a ternary picks
  between — `($total % 2) == 0 ? $short : $long` — and re-measure with `--force`, which appends a
  second ledger row rather than replacing the first. [until: gone benches/members/core/Bytes/join.nvs:($total % 2) == 0 ? $short : $long]
