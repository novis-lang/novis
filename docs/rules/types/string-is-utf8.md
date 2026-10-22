A `string` cannot hold invalid UTF-8 at any point in its lifetime. The invariant is enforced at every
construction site — literals, conversions, concatenation, and every stdlib member that builds a
string — the same way an array's element type is enforced on every write (`rule:types/arrays`). That
is why there is no second, encoding-aware family of string members: there is only one encoding a
`string` can hold, so there is only one correct answer to "how long is it".

The type says which kind of data a value is, and that is a security property as much as a naming one.
Treating an attacker's byte buffer as text without asserting its encoding is the bug behind a length
check made in the wrong unit and a truncation that splits a multi-byte sequence; with the invariant,
the only way from untrusted bytes to text is a checked `bytes as string` (`rule:types/bytes`).

A `Core` member that reads outside bytes as text checks them where they enter, and throws a
`ParseError` for a run that is not UTF-8: `Core\Csv::rows` naming the line, and `Core\Jwe::decrypt`
for a payload whose tag held. A member whose input is already a `string`, and which cuts or removes
only ASCII bytes, has nothing to check: `Core\Csv::parse` is one. A plaintext that a seal opens is
checked too, because a tag proves who sealed the bytes and not that they are text: `Core\SignedCookie::open`
refuses it as it refuses a forgery, and `Core\Session::getSecret` and `Core\Cache::getSecret` read it
as a miss. In the runtime, a `string` value is made from bytes only through `Value::str`, which checks
nothing. `clippy.toml` refuses it outside an audited site, which says why its bytes are UTF-8, and
`Value::text` makes one from a Rust `&str`.

**A `string`'s length, indexing and iteration operate on extended grapheme clusters** (Unicode UAX
#29) — the unit a person reading the source calls "one character", including a flag emoji, an emoji
built from a ZWJ sequence, or a letter with a combining accent. Byte-level and codepoint-level
operations remain available under separately named members.
`nvs_stdlib::granularity` states that default in code, once, and every `Core\Str` member with a unit
reads it from there. Byte length could not be the unmarked unit beside the invariant: a byte-indexed
`Core\Str::at` would have to return the interior byte of a multi-byte character, which is not a
`string`, so `length` and `at` would count in two different units.

What counts as one character is pinned to whichever Unicode version `nvs-runtime` embeds, and can
change across a runtime upgrade. Counting is O(n) on the first ask — a vectorized scan over ASCII,
where one byte is provably one cluster, and a full segmentation run over anything else — and the
count is then kept in the string's header, so a later `Core\Str::length` on the same string is O(1).
A concatenation carries the count across only when both operands already have one, corrected at the
seam, because a cluster can span the join. A position on a string whose clusters are all one byte is
a byte offset; on any other string, `Core\Str::at` and the members that take a position walk the
clusters to it on every call. Normalization (NFC/NFD) is out of scope: grapheme awareness says
nothing about whether two visually identical strings compare equal.

- **Memory:** every string allocation carries one more word, 8 bytes, for its cached grapheme count,
  filled lazily the first time anything asks. No per-cluster offset table is kept, so indexing a
  string with wide clusters costs time rather than memory.

**Declined:**
- **Code points as the unmarked unit.** A code point count still disagrees with what a person sees:
  a flag emoji or a letter with a combining mark is more than one. It stays reachable as
  `Core\Str::codePoints`.
- **Bytes as the unmarked unit.** It cannot index a guaranteed-UTF-8 `string` at all, as above.
- **One string type with a run-time "is valid UTF-8" flag.** A function still could not tell from its
  signature which kind of data it holds, which is the ambiguity the two types remove.
- **Counting graphemes eagerly at every construction site.** It pays the segmentation cost on every
  string, and most strings a request builds are never asked their length.
- **A cached offset table for random access by grapheme index.** It would cost one word per cluster
  on every string ever indexed, and the walk is linear.
- **Replacing a bad run with U+FFFD where outside bytes enter.** It changes the data without a word,
  and a later check could still be fooled by an overlong form such as `C0 BC`.
