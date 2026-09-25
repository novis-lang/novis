- **A `-p nvs-hir` member fixture reports the same `Class::member` miss twice, so a test that counts
  the diagnostics fails on a check that works.** One `self::missing()` in a method body comes back as
  two identical `E0309`s from `MemberResolver::check`, and every older test in that module asserts
  with `.any()`, which hides it. Assert over the whole filtered set — all of them carry the help, or
  none of them do — rather than indexing the first and pinning a count of one.
  [until: gone crates/nvs-hir/src/members.rs:MemberResolver]
