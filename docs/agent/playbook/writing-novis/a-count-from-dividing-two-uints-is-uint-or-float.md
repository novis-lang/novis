- **`/` on two `uint`s gives `uint|float`, so a count made by dividing a size does not compile
  where a `uint` is needed.** `Core\Str::repeat`'s count is one such place, and a `.scale.nvs`
  ladder only fails there when `--record-perf` ramps it, a whole recording later. Write
  `Core\Math::intDiv($size as int, 66) as uint`, and run `nvs check` on the file before recording.
  `benches/members/core/Json/decode.scale.nvs` does it. `until: gone benches/members/core/Json/decode.scale.nvs:Core\Math::intDiv`
