Every decompression runs under an absolute output ceiling and a ratio of output to input, and a
call may lower either and raise neither. The two are one bound: an absolute ceiling alone is one
every request may reach whatever it sent, and a ratio alone lets a large input buy a proportionately
large output, so what a decode compares against is `min(input × ratio, ceiling)`.

`[limits] max_decompressed` and `[limits] max_decompression_ratio` carry the ceiling, default to
64 MiB and 1000:1, and are `System`-class — an operator's decision, never a request's. A call names
`$maxBytes` and `$maxRatio` to ask for less, and gets the smaller of the ask and the configured
value on each axis independently. **There is no spelling for an unbounded decompression**: not an
argument, since the largest `uint` is still clamped and `0` is a bound of zero; and not a
configuration, since `false` — which `rule:config/three-changeability-classes` reads as "no ceiling"
for the limits a request may raise — reads here as the shipped default.

A breach throws rather than truncating, because a truncated decompression that looks like success is
the failure this bound exists to prevent. The throw is a `ParseError` naming this rule, never an
`IOError`: a hostile archive and a failing disk are different questions, and a caller that cannot
tell them apart retries the one it should have refused. The bound is applied while the output grows
rather than to a buffer already allocated, so a bomb costs the ceiling and never the size its own
header claims.

`Core\Compress` and `Core\Zip` share this one rule, and `Core\Zip` applies it per entry *and* across
the archive — an archive whose entries are each within the bound and whose total is not is the same
attack one level up. This is why both classes are Tier 0 rather than sandboxed components
(`rule:core-api/tier-roster`): a sandboxed decoder gets a memory cap for free and this rule not at
all.
