- **A closure is an object of a compiler-synthesized class, so it renders as that class's name.**
  `rule:types/anonymous-function` makes a literal an object with one field per capture, so
  `Core\Test::assertSame($f, $g)` over two `callable`s prints ``a `Script$fn0` `` and
  ``a `Script$fn1` `` — a name no rule owns. Keep a `callable` out of a case about how a value
  renders; the same goes for `Tag::Unset`, which no file-scope expression produces at all.
  [until: gone crates/nvs-ir/src/lower/snapshots/nvs_ir__lower__tests__an_anon_fn_lowers_to_a_captured_environment_object_and_an_invoke_method.snap:Script$fn0]
