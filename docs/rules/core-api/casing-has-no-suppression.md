There is no configuration, no per-project override and no suppression annotation for a casing error. A name
that must violate the convention — because it mirrors an external wire format, or because a converter
produced it unmodified — does not compile until it is renamed, and a mapping layer is the only route.

A suppression attribute would be the first compiler-level opt-out of any kind in this language, which is a
larger precedent than the rule it would soften; and a warning tier would be ignored indefinitely, since
every other diagnostic here is an error. The same reasoning covers the other spelling rules: a rule whose
whole value is that the author decided is worth nothing if the author can skip it.

The stance is the piece most likely to be revisited, and only under pressure from a concrete boundary that
does not exist yet. The question would then be whether *that one* boundary needs an escape, not whether the
check should have been a warning.
