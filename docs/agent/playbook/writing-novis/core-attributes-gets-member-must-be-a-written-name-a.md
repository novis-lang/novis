- **`Core\Attributes::get`'s `$member` must be a *written* name; a computed one silently answers
  `null`.** A loop over `array<string> $names` reading one option per name prints nothing at all,
  while the same reads spelled as literals answer — `rule:attributes/structural-retrieval` decides
  it, since a written name is checked against the target's declarations and a computed one falls
  back to the empty result. Write the parameter name at the call site, and where a list is really
  wanted, build the array of results rather than the array of names.
  [until: reviewed 2026-09-20]
