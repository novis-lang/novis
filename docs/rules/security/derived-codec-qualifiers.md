A derived codec introduces no qualifier rule of its own. A decoder assigns into declared property
types, so a payload that carries `tainted` requires the fields receiving it to declare it, and the
check happens where the qualifier is statically known — the **call site** that decodes, not inside the
codec. Only `string` and `bytes` carry a qualifier, so an integer, a decimal, an enum or an instant
field needs nothing. Declaring `tainted` on a decoded field is friction at the moment somebody is
trying to be quick, and it is the same declaration hand-written hydration already needed; the derive
is only where a newcomer meets it first.

A member that reads its own payload decides by its owner: every `Core\Request` decoder,
`Core\Http\Response::jsonAs`, `Core\Jwt::verifyIssued`, and `queryAs` and `streamAs` on a
`Core\Db\Connection` or `Core\Db\Transaction` carry `tainted` always. A row's text is input because
an earlier request may have stored it (`rule:security/tainted-sources`), so a `#[Db\Derive]` class or
an inline shape fills a text column only into a `tainted` field. A field no column maps is refused
by its type instead, and gets no second report. `Core\Arr::shapeAs`
converts the array it is handed, so its argument decides: by the test an encoder applies
(`rule:security/taint-propagation`), an array whose type carries `tainted` or `mixed` anywhere, or a
literal holding such a value, requires every text field it fills to be `tainted` (`E0810`). An
`array<string>` or a literal of the program's own values fills a plain `string`. A literal element that
is a call is read as outside text, because its type is not kept there; binding it to a typed variable
first is the fix. `Core\Json::decodeAs` decides by its document the same way: a `tainted string`
document is admitted, and every text field it fills must be `tainted` (`E0810`), while the program's
own `string` fills a plain field. Admitting the document launders nothing, because the class it
answers carries the mark in each field that receives text.

The check reaches every class the type argument reaches, not only one written at its root. One walk
steps into an inline shape's fields, an array's element and a nullable, at any depth, and through a
deriving class's own shape property. So `jsonAs<{ blog: { post: Post } }>`, `jsonAs<array<{ post:
Post }>>` and `jsonAs<Blog>` with a `{ post: Post }` property each hold `Post` to `E0810`, to
`E0820` where its constructor cannot be filled, and to `E0821` where it has no codec, as
`jsonAs<Post>` does. Every typed decoder that builds from a document runs it:
`Core\Request::jsonAs`, `queryAs` and `postAs`, `Core\Json::decodeAs` and `decodeEach`, and
`Core\Arr::shapeAs`. A row decoder does not, because a row has no column for a class. The run-time
decoder's own refusal of a class it cannot build stays as the last line. Declined: a walk per check
that knew only a root class and an array of one; a class under a shape then decoded plain text into a
plain `string`, which is the hole this paragraph closes.

A `secret` property on a `#[Json\Derive]` class is a field its decoder fills, so a login form read
through `jsonAs` or `postAs` lands its password in a `secret tainted string` and the debug record
replaces it (`rule:testing/a-recorded-request-keeps-its-input-with-credentials-replaced`). Encoding
was already a sink (`rule:security/secret-sinks-refuse`), and it stays one: a call of
`Core\Json::encode` or `Core\Response::json` whose argument's type reaches such a class, through an
array, a shape, a union or another field, is `E0462` at the call, naming the property. A value the
checker cannot see through, one typed `mixed` or handed to an encoder in a bag, is refused by the
encoder when it reaches the object, by the class's `secret` bits, with a `LogicError` naming the
property. A class that writes its own encoder decides for itself, and skipping the property keeps the
class encodable. So such a class has a decoder and no usable derived encoder.

A `#[Db\Derive]` class keeps the refusal at the declaration: a row is read and written by the same
derive, so a `secret` property there is `E0462` where it is written, with skipping it as the fix.

Trade-offs: one walk of the type argument per decode call and of the argument's type per encode call
at compile time, and at run time one scan
of a class's slot bits per object encoded whose class has a `secret` property.

**Declined:** keeping the declaration refusal for JSON. It made the replacement in a recorded body
unreachable, because no derived decoder could fill a `secret` property, so a password decoded with
`jsonAs` sat in a plain `tainted string`.
