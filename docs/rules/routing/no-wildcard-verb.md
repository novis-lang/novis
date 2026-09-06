There is no `Method::Any`, no omitted `method` and no multi-valued `method`. `method` is required,
singular, and one case of the closed enum `Core\Http\Method`; a `#[Route]` written without one is
`E0747`. This is a decision, not a deferral, because two mechanisms read the exact verb set a
wildcard would erase:

- **`Core\Router::methodsFor`** answers the verbs a path serves — an empty list is a `404`, a
  non-empty one is a `405` with an `Allow:` header naming them. A wildcard makes that header either
  unanswerable or a dump of the whole enum, and `405` stops being reachable.
- **CSRF is classified per verb** (`rule:security/csrf-is-on-by-default`) — on for `POST`, `PUT`,
  `PATCH` and `DELETE`, off for the safe verbs — from the declaration, not from the request. A
  wildcard declares a route that is half unsafe.

A wildcard could not even mean *any* verb honestly: `Core\Http\Method` is closed
(`rule:enums/closed-integer-type`), so it would mean the cases this version of `Core` happens to
name, silently changing meaning when one is added.

`rule:routing/repeated-routes-share-a-name-when-they-share-a-path` gives the ergonomics a wildcard
was wanted for — one name, one `url()` target, one metrics series. What it does not give is one
*line*, and that is the point: each verb a route answers stays visible at the declaration, where
`methodsFor` and the CSRF classification read it. A method serving a safe and an unsafe verb carries
one `#[Access]` for both, and the CSRF check is still per verb from that one declaration.
