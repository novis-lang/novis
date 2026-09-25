- **`Core\Str::length` counts characters, and a CRLF is one of them — so it is the wrong ruler for a
  round trip.** A `Core\Csv` probe measuring `"a\r\nb"` read 3 on both sides, which reads as "the
  reader normalized the CRLF away"; it had not. Any claim about *which bytes* survived a member is
  written with `Core\Encoding::toHex($s as bytes)`, and `length` is kept for a count of characters.
  [until: gone crates/nvs-stdlib/src/str.rs:Core\Str::length]
