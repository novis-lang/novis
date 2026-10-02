A class whose `parse` takes **exactly one `string`** and can fail also carries
`tryParse(string $s): ?T` — `Core\Uri::tryParse` and `Core\Uuid::tryParse` are it today. `tryParse`
**is** `parse` with the throw caught; it is never a second implementation, so there is exactly one
answer to "is this text a `T`".

The global interface `Parses` makes the pair a contract: its one required member is `parse(tainted
string $s): static`, and `tryParse` is a default body on the interface rather than a second required
member. A user class implementing `Parses` declares `parse` alone, and declares its own `tryParse` when
it wants the non-throwing call.

Three conditions keep the shape from regrowing into a `from`/`tryFrom` habit. The `parse` takes one
`string` and nothing else — a parse taking a format or an options bag is a different shape and gets
none. `tryParse` delegates to `parse`. And the spelling is `tryParse`: `…OrNull`, `…Safe` and `…Ex`
stay banned, as does `try…` on every other verb.

**A parse that launders gets no non-throwing twin.** Where laundering is the point, the throw is the
right control flow, and a second launderer to audit is not worth one saved `catch`.

**No separate validity predicate stands beside a parser.** `isValid` members do not exist: a
validity question and its parse answered by two pieces of code is exactly the shape behind
CVE-2024-5458, where one validator accepted user-info that the parser read differently. Ask the
parsed value instead — `Core\Uri::tryParse($s)?->scheme() != null` is the absolute-URI test.
