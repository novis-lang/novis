- **PHP's four `str*cmp` twins disagree with each other on magnitude, so an oracle over
  `Core\Str::compare` needs a sign normalization first.** `strcmp`, `strnatcmp` and `strnatcasecmp`
  answer -1/0/1 where `strcasecmp` still returns the byte difference, while `compare` is always one
  of three literals. Wrap each PHP call in a sign function before comparing; `compare` is also the
  only ordering two strings have, since `<` over two `string`s does not lower.
  [until: reviewed 2026-09-06]
