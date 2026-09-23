- **`as array<SomeClass>` is refused with `E0711`; convert to `array<mixed>` and convert each
  element where it is read.** `rule:types/conversion`'s `array<T> as array<U>` row checks every
  element by its runtime tag, which a class is not decided by, so it surfaces once a container of
  objects rounds through something answering `mixed` such as `Core\Serialize::decode`. The spellings
  that compile are `... as array<mixed>` then `$copy[0] as Cell`, and nested, `($outer[0] as
  array<mixed>)[0] as Cell`. [until: gone crates/nvs-diagnostics/src/lib.rs:Code::new("E0711")]
