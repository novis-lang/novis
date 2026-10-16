```nvs
enum Core\Audience { case Public; }
type Core\Access = {allow: mixed, csrf?: bool};
```

`allow` is required, and its declared type is `mixed` because the compiler promises not to know what
the value means. What constrains it is narrower than an ordinary payload
(`rule:attributes/payload-is-a-compile-time-constant`): it must be **an enum case or a class constant**
and never a bare literal, since the guarantee is precisely that the name resolves. `Role::Admin` and
`Policy::ADMIN` are accepted; `"admin"` and `1` are `E0761`, and an `#[Access]` carrying no `allow` at
all is `E0760`. The resolved name rides on the row for whoever dispatches to read.

`Core\Audience::Public` is the one access decision `Core` names, so *this route is open* is a resolvable
name rather than a magic string. `Core` ships that one case and will not grow a second — a roster of
access levels is interpretation, which the compiler refuses. Every other value is the application's.

`csrf` is optional and defaults to on for the four unsafe verbs. `csrf: false` on a method whose every
verb is safe is `E0764`: there is nothing to opt out of.

**Exactly one `#[Access]` per method**, and a second is `E0763`, naming both. Two decisions are two
readings — conjunction or disjunction — and choosing between them silently is the failure this
prevents. A method carrying several `#[Route]` attributes still carries one `#[Access]`, covering all
of them.
