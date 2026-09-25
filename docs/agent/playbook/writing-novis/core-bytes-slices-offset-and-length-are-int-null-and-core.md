- **`Core\Bytes::slice`'s offset and length are `int|null`, and `Core\Bytes::length` answers `uint`,
  so feeding one to the other does not compile.** `E0401: expected int|null, found uint` lands on the
  argument, so it reads as the wrong member rather than as a missing cast. Write
  `int $length = Core\Bytes::length($b) as int;` once at the top and slice with that.
  [until: gone crates/nvs-stdlib/src/bytes.rs:Core\Bytes::length]
