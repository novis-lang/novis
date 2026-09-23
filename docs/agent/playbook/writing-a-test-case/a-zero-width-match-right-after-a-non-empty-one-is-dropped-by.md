- **A zero-width match right after a non-empty one is dropped by all four `Core\Regex` iterating
  members.** `replace`, `replaceWith`, `matchAll` and `split` use the `regex` crate's
  `captures_iter`/`split`, which skip it, PCRE does not: `Core\Regex::replace("ab", "b*", "-")` is
  `-a-`, PHP says `-a--`. Only an oracle sees it, so a differential row must not put a zero-width
  match after a wide one. [until: gone crates/nvs-stdlib/src/regex.rs:captures_iter]
