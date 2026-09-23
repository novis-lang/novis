- **`nvs_syntax::parse` hands back trivia but no tokens, so a formatter's runs are the *complement* of
  the trivia spans.** `Parsed` is `stmts`, `trivia` and `index`; every byte the grammar skips is a
  trivium, so what lies between two consecutive trivia is tokens and nothing else, and that is the
  tiling `crates/nvs-fmt/src/print.rs` walks. It is enough to rewrite whitespace and comments and it is
  not enough to *insert* a space inside `$a+$b`, so the first rule that needs token boundaries either
  puts them in `Parsed` beside the trivia — which `Lexer::with_trivia`'s own doc anticipates — or reads
  them from `nvs_syntax::tokenize`. [until: reviewed 2026-09-12]
