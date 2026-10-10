- **Replacing a bag guard's "never nullable" invariant with the omitted-versus-null pairing turns
  every `CoreTy::Mixed` option red, not just the one you set out to make nullable.** `mixed` admits
  `null` without spelling it, so `Core\Queue::push`'s `args` had to take `Const::NeverWritten` the
  moment `Core\Uri::with` did, and the assertion names the member. Before landing such a pairing, `grep -n 'CoreTy::Mixed' crates/nvs-stdlib/src/`
  for the rows it will reach, and widen each helper's "not given" branch from `Tag::Null` to
  `Tag::Null | Tag::Unset` in the same edit. See `crates/nvs-stdlib/src/queue.rs`. `until: gone crates/nvs-stdlib/src/queue.rs:Const::NeverWritten`
