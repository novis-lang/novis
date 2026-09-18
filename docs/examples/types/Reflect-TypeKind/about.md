What a value actually is, when its declared type no longer says. Nearly everywhere in Novis the type
is written down and nobody has to ask; a `mixed` is the exception, and asking one what it is holding
answers with one of ten kinds.

The ten are `Null`, `Bool`, `Int`, `Uint`, `Float`, `Decimal`, `Text`, `Bytes`, `Array` and `Object`.
Every value is in exactly one of them and never in two, so a `match` covering all ten needs no
fallback arm and no answer can be argued with.

**Good to know:** this is the single question PHP asked with fourteen — the whole `is_*` family plus
`gettype`. It also draws lines that family could not: a whole number that cannot be negative is
`Uint` and not `Int`, an exact `Decimal` is never a `Float`, and a run of octets is `Bytes` rather
than text that happens to hold them. A closure is an ordinary object, so there is no separate case
for one to disagree with.
