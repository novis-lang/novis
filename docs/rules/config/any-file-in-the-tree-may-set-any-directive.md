Because `rule:config/ownership-is-the-trust-boundary` is uniform, **any file in the tree may set any
directive**, `System` class included: capabilities, `[limits.hard]`, `[mode] ceiling` and
`[[extension]]` entries are as legitimate in `conf.d/host.toml` as in the root file, and a capability
grant in an included file takes effect.

Whoever can write an included file cleared exactly the same bar as whoever can write the root file, so
a rule restricting what an include may say would buy no security an attacker does not already have,
while costing per-host capability sets and per-environment extension sets. Confining `System`
directives to the root, or letting an include only narrow, would each invert deny-by-default at the
file layer: the root would have to grant every right any environment needs so that each host could
take some away.
