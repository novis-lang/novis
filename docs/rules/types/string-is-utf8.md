A `string` cannot hold invalid UTF-8 at any point in its lifetime. The invariant is enforced at every
construction site — literals, conversions, concatenation, and every stdlib member that builds a
string — the same way an array's element type is enforced on every write (`rule:types/arrays`). That
is why there is no second, encoding-aware family of string members: there is only one encoding a
`string` can hold, so there is only one correct answer to "how long is it".

**A `string`'s length, indexing and iteration operate on extended grapheme clusters** (Unicode UAX
#29) — the unit a person reading the source calls "one character", including a flag emoji, an emoji
built from a ZWJ sequence, or a letter with a combining accent. Byte-level and codepoint-level
operations remain available under separately named members.
`nvs_stdlib::granularity` states that default in code, once, and every `Core\Str` member with a unit
reads it from there.

Two consequences an implementer owes: what counts as one character is pinned to whichever Unicode
version `nvs-runtime` embeds, and can change across a runtime upgrade; and `Core\Str::length` is
O(n), a vectorized scan over ASCII and a full segmentation run over anything else. Normalization
(NFC/NFD) is explicitly out of scope — grapheme awareness says nothing about whether two visually
identical strings compare equal.
