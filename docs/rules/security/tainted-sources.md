Every member of the request, server, session, environment and CLI accessors, and a script's own
arguments, answers the `tainted` form of whatever it already returned. Header values and any other
client-influenced field are included; a fixed enum-shaped field such as the request method is not
attacker-shaped in the same way and is not required to be.

**An outbound reply's body is input in the same sense**, so a response body read back from a client
answers `tainted string`: pinning an address settles which host the bytes came from and says nothing
about what is in them, and a reply a program asked for is no safer than one it was sent. Its status
code is not tainted — three digits carry nothing a sink can misread. Values read back out of a
database are `tainted` under the same standing rule, which is what closes stored injection by the same
mechanism as reflected.

**An object read from storage is `tainted` too.** `Core\Storage::get` answers `?tainted bytes`: a
disk is where a program puts what a visitor uploaded, so an object is a row by another name.
`Core\Storage::list` answers plain keys, since a key is one to 255 letters, digits, `.`, `-` and `_`,
and like a status code it carries nothing a sink can misread. **A file read through `Core\IO` is not
`tainted`.** Most such reads are the program's own templates, data and assets, and a program that
keeps uploads as plain files chose the folder an upload lands in. An upload itself is `tainted` where
it enters, through `Core\Request\Part`.

**A request field is read as text, and typed at the accessor.** `Core\Request::query` and `::post`
answer one field as `?tainted string` — `null` for a name that is absent and for one written with
brackets — and `::queryArray` and `::postArray` answer a bracketed name's one level of values as
`array<tainted string>`, keyed as the brackets keyed them — so a `foreach` key over one is `tainted
string` as well (`rule:security/taint-propagation`) — refusing a second level with a `ParseError`
that names `queryAs`/`postAs`. A deeper shape is those two members' to validate into a written type,
so no request accessor answers a field as `mixed`. A value that is `mixed` by nature — a decoded
document such as `json()`, a session or cache entry, a row's `get` — carries the qualifier the moment
text is pulled out of it, because text out of `mixed` is `tainted` (`rule:security/taint-propagation`).
A typed reader answers the tainted form itself: a row's `string` is `?tainted string` and its `bytes`
is `?tainted bytes`. The list of sources being **enumerable** is what lets the qualifier attach itself
automatically, and is exactly what `secret` has no equivalent of
(`rule:security/secret-has-no-ambient-source`).

**`Core\Env::secret` is the environment's one read that is not `tainted`.** It answers `?secret
string` (`rule:security/secret-has-no-ambient-source`). `tainted` stops a value at a sink that would
read it as an instruction: a URL, a path, a query, markup. A `secret` reaches none of those, because
every output and instruction sink already refuses it, and it does reach the two places a credential
belongs, an outbound header and a database password, which refuse `tainted`. Carrying both bits would
make the member unusable for its one purpose. The operator who started the process chose the value,
as they chose `nvs.toml`; `Core\Env::get` stays `tainted` because its value may reach any sink.

**`Core\Random\Seeded::bytes` answers `tainted bytes` and `::token` answers `tainted string`, and
every key parameter refuses `tainted`.** Anyone who knows the seed draws the same bytes again, so
they are as predictable as a visitor's text. A key parameter is a sink on the `tainted` axis: no
octet of a key reaches an answer, but a key somebody else chose or can predict protects nothing.
That covers each key `Core\Crypto`, `Core\Crypto\KeyPair::read`, `Core\Hash::hmac`, `Core\Csrf`,
`Core\Jwt`, `Core\Jwe\Key::shared` and `Core\Totp` take, and every key ring (`Core\SignedCookie`,
`Core\Signature`, the session and cache rings, signed URLs). A key a program stored in a database
row is `tainted` too, and reaches a key parameter through `Core\Taint::assertTrustedBytes` with its
reason. The seeded `int`, `float`, `pick`, `sample` and `shuffle` stay plain: none is the type a key
parameter takes. `nvs_stdlib::keyring::KEY` is the one type every key parameter is written with.

No `Core` member names a qualifier, and the taint diagnosis has no opt-out. A `taintedPostAs` would
make every other member's name read as a claim it does not make. A caller who finds the diagnosis
verbose writes `tainted {…}` over the shape.

**Declined:**
- **A `predictable` qualifier of its own.** It would be a third axis that every signature and every
  diagnostic carries, for one source. `tainted` already says "do not let this decide anything".
- **Every `Core\IO` read answering `tainted`.** Each template, fixture and shipped data file would
  need `Core\Taint::assertTrusted` with a reason, and the reason would be the same everywhere. A
  qualifier written that often stops being read. A program that stores uploads should use
  `Core\Storage`, whose reads are `tainted`.
- **A run-time check that a key looks random.** No test of a 32-byte buffer tells a seeded draw from
  a real one.
