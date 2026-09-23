- **Widening a union of enum-case types rewrites every `E0401` that names it, and a
  `--EXPECTF-ERROR--` case has the whole list frozen in it.** The accepted set is generated from the
  parameter's type, so a refusal case
  (`tests/conformance/core/hash-hmac-refuses-a-weak-digest.nvst`) is a *roster* case, and `%A` never
  covers the message. Before adding a case to a `CoreTy::EnumCase` union, `grep -rn "<the union's
  first case>" tests/conformance/` and update every hit. [until: reviewed 2026-09-06]
