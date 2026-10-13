The server computes a name and typed parameters and stops there. It never calls the annotated method,
never decides what a return value means, and never wraps anything: the route table is complete without
crossing into dispatch. The program calls the handler, through `Core\Router::dispatch`
(`rule:routing/a-gate-answers-around-every-handler`) or a `switch` of its own, and a program that does
neither is served exactly the same way.

So a request the table does not claim is an ordinary served request. `route()` answers `null`, the
program runs exactly as it would for any other request, and the `404` — if it is one — is something the
program sends, not something the door refuses with: `dispatch` sends it through the gate's `refused`.
A door that refused a miss would refuse every request of a program declaring no `#[Route]`, which is the
opt-in table inverted.

The boundary holds one layer down as well. `Core\Router::match` answers a `Core\Router\Match` and
nothing invocable (`rule:routing/a-match-is-not-invocable`), and the one decision the server enforces on
a match is CSRF (`rule:security/csrf-is-on-by-default`) — the access name rides through for whoever
dispatches to read. A request the door refused still runs a program that calls `dispatch`, in refusal
mode, where `route()` answers `null` so no handler is reachable and only the gate answers.
