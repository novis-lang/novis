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

A new sink therefore needs no rule of its own — it needs the predicate applied. The four
grammars the library parses are all instructions by it
(`rule:security/every-grammar-is-a-sink`), and a member nobody classified fails closed
(`rule:security/unclassified-parameter-refuses-tainted`).

**A name that picks code is an instruction.** Every `Core\Reflect` parameter whose text picks a class,
a method, a property, a constant, an enum or an enum case is a sink: `Reflect::forClass`,
`ClassInfo::constant`, the `$name` of `get`, `set` and `call`, `EnumInfo::of`, `EnumInfo::valueOf` and
`AttributeInfo::field`. A request path segment passed there would choose what runs.
`ClassInfo::hasProperty` and `hasMethod` stay neutral, because they return a `bool` and run nothing.
A dispatcher compares the text with its own list, or writes `Core\Taint::assertTrusted` with its
reason. `crates/nvs-stdlib/tests/taint_signatures.rs` fails on a `Core\Reflect` text parameter that
is not a sink on a member that returns anything but a `bool`.

**Declined:**
- **Denying `tainted` at the process boundary, so it may leave only through a typed carrier.** Strictly
  stronger and wrong: it refuses a file's contents, an outbound request body, a bound database parameter
  and a log field, each of which is safe and common. The predicate is what separates them, so a boundary
  rule would be this rule with its useful half deleted.
