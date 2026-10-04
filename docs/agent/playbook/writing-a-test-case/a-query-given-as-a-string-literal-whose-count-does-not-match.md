- **A query given as a string literal whose `?` count does not match its values is a compile error, so a hostile case
  cannot catch it.** The compiler binds a literal statement with the request's own rewriter and
  raises `E0770` there, so the mismatch never reaches run time, and a hostile case holding one is a
  compile diagnostic — which `rule:testing/hostile-case-contract` counts as a failure. Build the
  statement while the program runs — `'insert into t (a) values (?' . Core\Str::repeat(', ?', 2) .
  ')'` — whenever the refusal itself is the attack. [until: gone crates/nvs-diagnostics/src/lib.rs:E0770]
