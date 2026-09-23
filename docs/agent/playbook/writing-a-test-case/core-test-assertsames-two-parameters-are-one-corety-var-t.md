- **`Core\Test::assertSame`'s two parameters are one `CoreTy::Var("T")` bound to argument 1, so
  `assertSame(null, $x)` does not exist.** Writing the literal first binds `T` to `null`, and the
  subject is then `E0401: expected 'null', found 'mixed'` at the *second* argument, which reads as
  "this member refuses a null comparison" and is only the signature. `mixed $nothing = null;` then
  `assertSame($nothing, $subject)` is the only way to ask identity's symmetry through this member.
  [until: reviewed 2026-09-06]
