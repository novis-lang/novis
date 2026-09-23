- **`Qual::Sink` needs no rule of its own in `nvs-types`, and that is not the gap it looks like.** A
  classified `CoreTy::Text(q)`/`Blob(q)` lowers to exactly what its unclassified spelling does
  (`nvs_types::core_lib`), so a sink parameter's refusal is ordinary assignability — a `tainted
  bytes` argument does not satisfy a plain `bytes` — and grepping for code that reads `Qual::Sink`
  finds `crates/nvs-stdlib` and nothing else. What needs a checker rule is the opposite shape, a
  sink whose parameter is `mixed` (`Core\Debug::dump`, `Core\Serialize::encode`), where nothing
  below the call can still see the qualifier; those live in `expr/quals.rs` as call-site walks over
  the written arguments. [until: reviewed 2026-09-06]
