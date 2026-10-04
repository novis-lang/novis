- **A regex tier read back out of `ExprTypeTable` covers `Core\Regex::compile` and nothing else.**
  `crates/nvs-types/src/intrinsics.rs`'s roster carries one Regex row, so a literal handed straight to
  `matches`, `match`, `matchAll`, `replace`, `replaceWith` or `split` is never folded and has no
  recorded tier — a test counting `regex_tiers()` over the conformance suite sees an eighth of the
  patterns it looks like it sees. Find the pattern argument through `nvs_syntax::walk` and the class's
  own registry rows instead, and settle each one by handing it to `compile`, which is what
  `every_written_regex_pattern_in_the_suite_has_its_tier_recorded` does. [until: gone crates/nvs-types/tests/intrinsics.rs:every_written_regex_pattern_in_the_suite_has_its_tier_recorded]
