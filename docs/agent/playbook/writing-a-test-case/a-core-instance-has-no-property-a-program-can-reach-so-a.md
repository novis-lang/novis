- **A `Core` instance has no property a program can reach, so a rule that writes one is writing a
  member.** `rule:core-classes/ratelimit-gcra` spells `Core\RateLimit\Decision` as `readonly
  allowed: bool, ...`, but `$d->allowed` does not compile; `CoreTy::Instance`'s doc comment is the
  rule. Check `CoreTy::Instance` before transcribing any rule that writes a `Core` value's fields
  with a colon.
  [until: gone crates/nvs-stdlib/src/registry.rs:no constructor, no property and no subclass]
