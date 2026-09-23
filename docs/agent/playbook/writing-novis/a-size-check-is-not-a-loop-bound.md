- **A size check is not a loop bound.** `Core\Bytes::repeat` and `Core\Str::repeat` checked the
  product (`affordable` on `len * times`) and then ran `for _ in 0..times`, so an empty subject made
  the product zero for every count and the loop spun a caller-supplied `uint` of iterations
  appending nothing — an unbounded spin on the request path that builds, is correct, and returns.
  Look for a loop whose iteration count is the caller's count rather than the result's size; the
  size seams cannot see it, and both members short-circuit on an empty subject now.
  [until: reviewed 2026-09-06]
