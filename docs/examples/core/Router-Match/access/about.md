Returns the access rule of the route that matched. It is the full name of the constant in the
route's `#[Access(allow: …)]`, such as `Core\Audience::Public` or `App\Role::Admin`.

The server does not check this rule for you. Your program checks it once, before it calls the
route's handler, so one check covers every route. The result is text from your program and not
from the request, so it is not `tainted`.

Every route in a program has an `#[Access]`, so the result is normally never `null`. If it is
`null`, treat it as "access denied".

The examples show the rule of each route, a check for one role, and one check that runs before
every handler.
