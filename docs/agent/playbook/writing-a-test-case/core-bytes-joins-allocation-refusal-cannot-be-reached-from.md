- **`Core\Bytes::join`'s allocation refusal cannot be reached from source, and the obvious probe
  pins the wrong member.** Its size is a sum, so a huge separator (`Core\Bytes::fill(1e12, 44)`) is
  refused by `fill`, and the row pins `fill` twice. The reachable count-shaped refusals are
  `Core\Str::repeat`/`padStart`/`padEnd`, `Core\Bytes::repeat`/`fill` and `Core\Random`'s two; a
  *product*-sized member reaches `nvs_runtime::affordable`'s sentence, a `uint`-sized one only the
  allocator's. [until: reviewed 2026-09-06]
