- **`Core\Str::from` does not exist, so a `.nvs` proof that turns a number into text uses the `as
  string` cast.** The diagnostic is `E0405: Core\Str has no member named from`, which names the miss
  and not the spelling that works, so it reads as a member still to be written. Write `($k as
  string)` — `Core\Str::repeat('k', 1000) . ($k as string)` is what builds a long key from a counter.
  [until: reviewed 2026-09-20]
