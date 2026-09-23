- **The ordering refusal is one throw in one wording, and a case only reaches it through `mixed`.**
  `Core\Math::min("a", 1)` never runs — `T` unifies at the first argument, so the second is `E0401`
  — so both halves of an orderless pair are laundered through `mixed` locals, and an array through
  `array<mixed>`, before `nvs_stdlib::ordering::compare_values` sees them. All seven members then
  raise the same `Fault::thrown`, `<member> has no natural order for tag N against tag M: …`,
  differing only in the name in front. [until: reviewed 2026-09-06]
