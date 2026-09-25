- **A `Core` class a `foreach` walks is not accepted where a `Core` row declares `Iterable<T>`.**
  `Core\IO::writeStream($path, $part->content())` fails as ``expected
  `array<bytes>|Iterable<bytes>|Iterator<bytes>`, found `Core\Request\PartContent` ``; conforming to
  `foreach` and to a declared `Iterable<T>` are two questions, only the first answered today. Run
  `target/debug/nvs test <case>.nvst` before writing the `--EXPECTF-ERROR--` block.
  [until: gone crates/nvs-stdlib/src/request.rs:PartContent]
