- **`every_error_path_is_asserted_or_declared_unreachable` keys on a message *stem*, so deleting an
  echoed refusal from a case un-covers every `Fault::` site whose message opens the same way.** One
  case echoing `` Core\Json::decodeAs(): `…` `` was covering four unrelated engine faults, and moving
  that refusal to a compile-time one left all four owing. Declare each site instead — `unreachable
  from source` plus the code that refuses first, within 8 lines and **on one line**, since the gate
  matches the phrase per line and a wrapped one is invisible.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:DECLARATION]
