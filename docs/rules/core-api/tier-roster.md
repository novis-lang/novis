Every subsystem has a recorded placement, including the ones no milestone has scheduled, because an entry
nobody will build is more useful named than pending. A class's tier is read off the roster rather than
re-argued at the point someone wants it, and a subsystem that is deferred says so with its reason.

The roster also records **splits**: a class may be Tier 0 and unconditional while a backend behind it is
Native and feature-gated, which is how the metrics class and the shared cache tier are placed. That split
is the general rule rather than a special case, and it is what keeps the `Core` name unconditional
(`rule:core-api/core-means-always-present`) while the thing it talks to is optional.

**Designed, not shipped.** Every class the roster places at Tier 0 is registered in
`crates/nvs-stdlib/src/registry.rs`: the four `spec-*-outstanding.txt` ratchets under
`crates/nvs-stdlib/tests/` are the gates that say so, each holds no keys, and each only shrinks. What
the roster places at Tier 0 and nothing provides is the four first-party sandboxed components. The
tier-boundary guard in `crates/nvs-stdlib/tests/tier_boundary.rs` holds the rule for what is
registered.
