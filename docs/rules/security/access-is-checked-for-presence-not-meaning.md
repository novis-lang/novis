The compiler verifies that a route's access attribute is there and that the name inside it resolves.
It never asks what the name means, never calls anything, and has no opinion about roles, policies or
sessions. An omission is an error rather than a public default — that half is
`rule:attributes/access-is-a-required-sibling`, and the payload's shape is
`rule:attributes/access-payload`.

**Interpretation belongs to whoever dispatches.** The declared name rides on the match the server made
as uninterpreted data, and the application reads and enforces it once, in front of every handler, so
a route whose handler never mentions access is still checked. Under `Core\Router::dispatch` that place
is the gate's `allows`, which runs before every handler the generated jump calls
(`rule:routing/a-gate-answers-around-every-handler`); a hand-written dispatcher checks it above its
`switch`.
**A dispatcher compares a case, never text:** `Core\Router\Match::accessAs<E>()` answers the decision
as a case of the enum `E` and `null` for any other decision, so a renamed case is a compile error at
the comparison and an admin route never reads as a public one. A case reaching `mixed` is only its
integer (`rule:enums/representation`), which is why the typed member names its enum rather than
answering `mixed`. `access()` answers the resolved name as a `string`, for a log line and for a class
constant, which has no enum. A type argument that is not an enum is **`E0841`**. An application that
mixes `Core\Audience::Public` with its own roles asks once per enum, and reads `null` as denied. A
comparison against the text keeps compiling after the case it names is renamed, never matches again,
and serves the route unguarded, so text is the reading for a log line only. **The server enforces
CSRF and nothing else** (`rule:security/csrf-is-on-by-default`): interpreting a role would need a
session, a user model and a role source, all three of which are deliberately outside the binary. Two
enforcement points is how a route ends up checked twice in development and not at all in production,
so there is one.

**The limit is stated rather than implied: the compiler guarantees the decision was *written*, not
that it was *honoured*.** `Core\Router::dispatch` is the dispatch site that closes this for a program
that calls it: no handler runs unless `allows` returned `true` for its match, so a gate that reads the
access decision enforces it on every route. What `allows` concludes stays the application's. An
application that hand-rolls dispatch and never reads the access name gets no enforcement from anyone,
because the compiler cannot follow a dispatch table a program builds from data.

Object-level authorisation is outside it too. Most privilege escalation is a request for a row that
belongs to another user, and no language-level mechanism answers that, because the check needs the
row. The handler that loads the row makes that check.

- **Memory:** `accessAs<E>()` allocates nothing per call: the enum's name is an immortal string
  constant of the unit. The work is one comparison of the row's enum name against the written one and,
  when they are equal, a linear search of the unit's enums by name.

**Declined:**
- **`access(): mixed` answering the constant's value.** A case in `mixed` is its backing integer, so
  `Core\Audience::Public` and `App\Role::Admin` are both `0` there, and a `match` would take the
  public arm for the admin route.
- **`access(Class::class): ?E`, typed from a string argument.** The checker would type a result from
  an argument's value, a special case no other member has, for the answer a written type argument
  already gives.
- **Keeping the text and teaching the comparison.** The silent rename stays, and that is the bug.
- **The decision readable only through `Core\Attributes::get` on the handler.** That call names the
  method, so the check is written once per `switch` arm. An arm that forgets it is an unguarded route,
  and nothing reports the omission.
- **A compile error where an access decision is never read.** The compiler recognises `dispatch`, but
  it cannot follow a dispatch table a program builds from data, and a gate's `allows` may decide on
  something other than the access name.
