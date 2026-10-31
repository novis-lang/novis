Every decompression runs under an absolute output ceiling and a ratio of output to input, and a
call may lower either and raise neither. The two are one bound: an absolute ceiling alone is one
every request may reach whatever it sent, and a ratio alone lets a large input buy a proportionately
large output, so what a decode compares against is `min(input × ratio, ceiling)`.

`[limits] max_decompressed` and `[limits] max_decompression_ratio` carry the ceiling, default to
64 MiB and 1000:1, and are `System`-class — an operator's decision, never a request's. A call names
`$maxBytes` and `$maxRatio` to ask for less, and gets the smaller of the ask and the configured
value on each axis independently. 64 MiB is far above any document a request decompresses, such as
a JSON body, an HTML page, a spreadsheet or a CSV export, and below what a request may hold under the
default `[limits] memory`, so a breach is an error the request survives. 1000:1 is above every input
that is not built to compress: text under gzip runs about 3:1 to 8:1, JSON with repeated keys up to
about 60:1, and a log of near-identical lines into the low hundreds, while published bombs start
around 10,000:1. Neither key is under `[limits.hard]`, which bounds a value a request sets for
itself, and a request has no setting to lower them: the call's own arguments are that. **There is no spelling for an unbounded decompression**: not an
argument, since the largest `uint` is still clamped and `0` is a bound of zero; and not a
configuration, since `false` — which `rule:config/three-changeability-classes` reads as "no ceiling"
for the limits a request may raise — reads here as the shipped default, as a malformed value does. A
configuration that spells an unbounded decompression would be written once for a debugging session
and left in place.

A breach throws rather than truncating, because a truncated decompression that looks like success is
the failure this bound exists to prevent. The throw is a `ParseError` naming this rule, never an
`IOError`: a hostile archive and a failing disk are different questions, and a caller that cannot
tell them apart retries the one it should have refused. Its message names this rule, the half of
the bound that stopped the decode, and the two keys a call may lower. The bound is applied while the output grows
rather than to a buffer already allocated, so a bomb costs the ceiling and never the size its own
header claims.

`Core\Compress` and `Core\Zip` share this one rule, and `Core\Zip` applies it per entry *and* across
the archive — an archive whose entries are each within the bound and whose total is not is the same
attack one level up. This is why both classes are Tier 0 rather than sandboxed components
(`rule:core-api/tier-roster`): a sandboxed decoder gets a memory cap for free and this rule not at
all.

**The ceiling bounds what is held, and the ratio bounds what is spent.** A part a consumer inflates as
a stream and never holds — a spreadsheet sheet read row by row
(`rule:core-classes/spreadsheet-reads-as-a-stream`) — is bounded by `input × ratio` alone, because
the request holds a window of it at any time and its CPU cap bounds the time. A part held whole keeps
both bounds, and an archive-wide total counts held parts only. The configured values reach a guest
the way `[image] max_pixels` does, clamped by the host before the call. Every `Core\Compress` and
`Core\Zip` member holds its output, so each keeps both bounds. A spreadsheet reader's message names
the part, the half of the bound that stopped it and the key that raises that half, and names no
rule: a program's caller reads it, and has no way to call with a lower bound. The PDF component's
`Reader::open`, `Reader::text` and `Pages::merge` decode every stream under both bounds, and the
attachments `Reader::open` holds count together against the ceiling. Their message has the
spreadsheet reader's shape. A stream's decode stops as its output grows, in the repository's patched
`hayro-syntax` (`vendor/hayro-syntax`), and an image's pixels are bounded by its dimensions instead.

A handler that decompresses something genuinely larger than the ceiling, such as a 200 MB archive
member, is refused until the operator raises `max_decompressed` for that deployment. The default is
chosen for the deployment that has not thought about it.

- **Memory:** the output buffer, at most the smaller of the two numbers, held by the request that
  asked for it. Nothing else outlives the call.

**Declined:**
- **An optional bound defaulting to unlimited, `?int $maxLength = null`.** The default of an optional
  bound is the whole decision, and `null` spells the wrong one.
- **The request's own `[limits] memory` as the only bound.** Reaching it stops the request and nothing
  catches that (`rule:errors/on-limit`), where a bomb should be an error the handler answers with a
  `400`. A heap ceiling also has no way to express a ratio.
- **A `Core\Compress\Bound` value object holding both numbers.** Two `uint` parameters carry the same
  two numbers without a class, a reference card and tests of their own.
- **Refusing early on the uncompressed size a frame or archive header declares.** The input's author
  wrote that number, so no member uses it, not even as a hint, and the output is measured as it grows.
