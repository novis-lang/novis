- **A nullable property cannot default to `null`.** `public ?string $s = null;` is `E0472: a
  property default must be a `string|null` constant — not a constant of the declared type`, and the
  same for every `?T`; a local `?object $m = null;` is fine, it is only the property-default folder
  that refuses the `null` literal. Declare the property non-nullable and fill it in `constructor`.
  [until: reviewed 2026-09-06]
