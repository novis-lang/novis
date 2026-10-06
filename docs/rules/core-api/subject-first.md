The thing a member operates on is its first parameter, including where the member also takes a callback or
a needle: `Arr::map($array, $fn)`, `Str::replace($subject, $search, $replacement)`,
`Arr::contains($haystack, $needle)`. Haystack comes before needle and subject before pattern, which is the
same rule stated for the two places PHP violates it most visibly.

A rule with no exceptions is learnable in one sentence, and the alternative is what PHP has: `array_map`
takes the callback first and `array_filter` takes the array first, so every call site is a lookup. The cost
is that a ported program's argument order changes at nearly every built-in call.
