- **`0 - $x` at `int`'s smallest value wraps back to itself in silence**, which is why
  `Core\Math::abs` is a member and not sugar for `max($x, 0 - $x)`: the composition is total at that
  row and quietly wrong, while `abs` refuses. A case asserting the divergence asserts that the
  derivation *answered* and `abs` *refused*, never what the arithmetic wrapped to — the overflow
  policy is another member's question and pinning it here fails the case for the wrong reason.
  [until: gone crates/nvs-stdlib/src/math.rs:Core\Math::abs]
