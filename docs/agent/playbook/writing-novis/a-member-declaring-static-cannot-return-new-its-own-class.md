- **A member declaring `: static` cannot `return new <its own class name>(…)`, and the diagnostic is
  `E0741: a body declaring `static` returns a value that is not the called class`.** `static` is the
  class the *call* named, which a subclass may be, so naming the declaring class is narrower than
  the declared return — and the message says what is wrong rather than how to spell it. Write
  `return new static(…)`, which is what `Core\Db\Codec`'s and `Core\Json\Codec`'s `fromRow`/
  `fromJson` fixtures need whenever the case asserts that nothing was reported.
  [until: reviewed 2026-09-15]
