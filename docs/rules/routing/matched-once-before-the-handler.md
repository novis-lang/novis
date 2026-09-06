The server matches the incoming request against the compiled table **once**, before any application
code runs, and records the result on the request. `Core\Request::route(): ?Core\Router\Match` returns
that match. `null` means the table claimed nothing for this method and path — or the program declared no
`#[Route]` at all, which builds no table and matches nothing. Nothing during the request can move it:
the match lives on the request carrier rather than on the context, so there is no place a second match
could be written from inside the program.

It is what three readers were written against. The CSRF check (`rule:security/csrf-is-on-by-default`)
needs to know which handler a `POST` is for, the `route` metric label needs the declared name, and the
access decision rides on it uninterpreted (`rule:security/access-is-checked-for-presence-not-meaning`).
Each reads the door's answer rather than matching again, which **removes** a match for most requests
rather than adding one — a match is about a quarter of the header parse the server performs anyway.

`Core\Router::match(Http\Method, tainted string)` remains for matching some *other* verb and path the
caller chose. It takes the same walk as the door, so the two cannot disagree about one path, and an
in-process test request (`rule:testing/in-process-request`) crosses the door's match the same way.
