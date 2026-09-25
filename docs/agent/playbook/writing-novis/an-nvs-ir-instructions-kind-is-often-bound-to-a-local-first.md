- **An `nvs-ir` instruction's kind is often bound to a local first, so grepping `self.emit(` for a
  literal `InstKind::` misses sites.** `let call = InstKind::HelperCall { … };` followed by
  `self.emit(cur, ty, call)` several lines down (`convert.rs`'s `as ?T` and `array<T> as array<U>`
  rows) is invisible to a scanner that parses the kind out of each call. Turn `nvs_codegen::emit`'s
  tolerant `None` arm into an `internal(...)` refusal and run the crate's tests: the backend already
  knows which instructions return a status, so let the suite enumerate the producers.
  [until: gone crates/nvs-ir/src/lower/convert.rs:InstKind::]
