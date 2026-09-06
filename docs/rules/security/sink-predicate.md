> A `string`/`bytes` parameter is a `tainted` sink when its content becomes an instruction that a
> parser executes. It is not a sink when its content is data that a parser returns, or that a
> serializer or a protocol frames.

That one line derives every worked case, which is the test of it being the right predicate rather than
a restatement. A query's text is a sink and its bound parameters are not, because the wire protocol
frames each value. An argv element is data *because* there is no shell to execute it, while the
executable path is an instruction. A header value is a sink because `CR`/`LF` re-frames the message; a
body is not, because `Content-Length` frames it. A path component is a sink because `..` and
separators direct the resolver; a file's contents are not. A regex pattern is a sink; its subject is
not.

A new sink therefore needs no decision record of its own — it needs the predicate applied. The four
grammars the library parses are all instructions by it
(`rule:security/every-grammar-is-a-sink`), and a member nobody classified fails closed
(`rule:security/unclassified-parameter-refuses-tainted`).
