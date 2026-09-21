`bytes` is a primitive scalar, peer to `string`, for data with no encoding at all — a file's contents,
a socket read, a hash digest, a request body before anyone has claimed it is text. It has the same
copy-on-write, interned-buffer value semantics `string` already has, minus the UTF-8 invariant, and it
is indexed and sliced by **byte offset**; there is no other unit for it to be ambiguous about.

It is not a class: a flat, contiguous, immutable-until-copied buffer has no use for identity,
properties or a vtable. It is not `array<uint>` either, which would cost a tagged value per byte plus
a per-write element check for a type whose whole point is that it has no per-element structure.

Anywhere the host hands a program data it has not itself asserted is text, it hands over `bytes`.
Structured input stays `array<mixed>` — `Core\Request`, `Core\Server` and `Core\Json::decode`'s result
are untyped deliberately (`rule:types/unions-and-mixed`) — and this rule is about the *scalar* payload
underneath, once one is pulled out of `mixed`. Converting is `as`, checked in the direction that can
fail (`rule:types/conversion`), which is what makes "treat these untrusted bytes as text" a reviewable,
throwing event rather than an unasserted assumption.

There is no dedicated literal token: `"…" as bytes` covers the valid-UTF-8 case for free, and
`Core\Encoding::fromHex()`/`::fromBase64()` cover arbitrary binary constants.
