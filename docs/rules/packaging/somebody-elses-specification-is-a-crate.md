Anything with an external specification — a protocol, a parser, a wire format, a cipher, a codec, a
cron expression, a timezone database — is implemented by an existing first-class Rust crate. Anything
about Novis's own compiler or scheduler has no possible crate and is written here. This is the
question beside `rule:packaging/a-c-dependency-answers-two-questions`: not what the tree may depend
on, but what it may write itself.

A second-rate implementation of somebody else's specification is a security surface the project would
then own for as long as the feature exists, so the default is that a feature without such a crate is
not built. Writing one anyway is the exception, argued in the feature's own rule against that cost:
`rule:core-classes/db-crate-boundary` does it for TDS, the one database wire with no crate, because
SQL Server is in the driver matrix at all only that way.

A crate can also be set aside for one call it makes on the request path. The MySQL and MariaDB
drivers read a binary row by hand, about 150 lines in `crates/nvs-db/src/mysql.rs`'s `decode_row`,
because `mysql_common`'s row reader costs what the request path cannot spend. It owns a vector per
text, bytes, decimal and JSON cell, so every such cell was allocated and copied once more before the
copy into the Novis string, and it packs an unsigned integer into a signed one. What is owned is
small and closed: the null bitmap, length-encoded values, fixed-width integers and floats, and the
date and time layouts. A type code the reader does not know is an error, where the crate panics.
What stays borrowed is everything per connection or per statement: packet framing, the handshake,
authentication and column definitions. The argument for any such call names the cost the crate
imposes, the lines owned, and what is still borrowed.

`Core\Json`'s reader is the second such call, in `crates/nvs-stdlib/src/json/reader.rs`. `serde_json`
reads a document by recursion, one native frame per nesting level, so a deep document needed a spare
stack and a depth that bounds stack rather than a loop. It hands a visitor each key as a borrowed or
copied `str`, so a key was copied once more on its way into a Novis string. It widens an integer literal
past `u64::MAX` to a `float` before the caller sees the token, so a too-large literal was silently
rounded. The owned reader is a pull tokenizer of about 750 lines: one event per call, the grammar
checked against one bit per open level, held inline, strings with their escapes, and the RFC 8259
number grammar with a digit count. It never recurses, its depth is a loop bound, and it has no `unsafe`
block. It keeps `serde_json`'s message text, line and column for every syntax error, and a differential
test over every saved fuzz input holds that. What stays borrowed is `serde_json` itself everywhere a
small document of a fixed shape is read, such as JOSE headers, JWK import and the editor's wire, and
as the oracle of that test. Declined: SIMD scanning, a pull-reader crate, whose messages differ, and
the crate's `arbitrary_precision` feature, which routes every number through a private map token.

The split is the one the tree already lives with. Cranelift compiles, and the lowering into it is
written here; `serde_json` parses, and the derive that emits Novis IR from Novis types
(`rule:core-classes/derive-attribute`) could not be a crate. When a feature is half of each — a
database driver borrows its codec and writes its state machine, an exporter borrows its encoder and
writes its wiring (`rule:observability/the-exporters-are-crates`) — which half is which is said
before either is written.
