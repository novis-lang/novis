```nvs
Core\Request::route(): ?Router\Match;                                   // the match the server made
Core\Router::match(Http\Method $method, tainted string $path): ?Router\Match;
Core\Router::methodsFor(tainted string $path): array<Http\Method>;      // [] ⇒ 404, else 405 + Allow:
Core\Router::url(callable $route, array<string, mixed> $params = []): string;         // launder (URL path)
Core\Router::urlAbsolute(callable $route, array<string, mixed> $params = []): string; // launder (URL)
Core\Router\Match::is(callable $route): bool;                           // did this method's route matchCore\Router\Match::params(): array<tainted string|int|uint|decimal|Core\Uuid|Parses>;
Core\Router\Match::param(string $name): ?(tainted string|int|uint|decimal|Core\Uuid|Parses);
Core\Router\Match::method(): Http\Method;
Core\Router\Match::access(): ?string;                                   // the resolved name of `allow:`
Core\Router\Match::accessAs<E>(): ?E;                                   // `allow:` as a case of enum E
Core\Router\Match::attribute<T>(): ?T;                                  // the method's payload of shape T
```

The server matches a request once, before any application code, and `Core\Request::route()` is that
match; `match` asks the same table about a verb and a path the program chose. Either answers which
method's route matched, through `is`, typed captures, the declared verb and the access decision, and **nothing invocable** — no `->invoke()`,
no callable, no class-and-method strings. Matching and dispatching are two steps: a match is data, and
the one call that runs a handler is `Core\Router::dispatch`, whose jump the compiler generates from
the table and whose gate answers every request the handler does not run for
(`rule:routing/a-gate-answers-around-every-handler`).

A program may also dispatch by hand. Any route in the table may have matched, so a capture's type is
one union and a read is guarded by `is`: one `if` per route, such as
`if ($match->is(Shop::show))`, in which the program states the type a capture already is with `as`.
A reference written at the call folds to the route's row index while compiling, and one passed as a
value is compared by row when the call runs. The same reference reads "is this the current page" in a
template, so a link and a match check name a route the same way. The access decision rides on the match uninterpreted
(`rule:security/access-is-checked-for-presence-not-meaning`), so a hand-written dispatcher checks it
once above those checks and the check covers every arm. `accessAs<E>()` answers it as a case of the
enum `E` the call site writes, and `null` when it is not one of `E`'s cases; `access()` answers the
resolved name of the constant `allow:` wrote, which is the only reading of a class constant.

A program can call `dispatch`, write its own `switch`, use `url` alone, or ignore all of it, and none
of those choices is penalised; keeping the match separate from the call is what makes the table
adoptable piece by piece. Hand-written dispatch costs a list of every route that nothing checks,
which is what `dispatch` removes.

Typed `callable` would not move this line: routes share no signature and handlers no return type, so
routing is not a case that forces `rule:types/grammar`'s deferral. Signing a route's identity is
`rule:core-classes/router-signed-url`.

**Declined**, each because it is a framework opinion:
- **An invocable match.** Dispatch is a call the program makes, and the match it reads stays data
  (`rule:routing/a-match-is-not-invocable`).
- **A controller convention.** No base class, naming rule or directory layout.
- **A built-in return-value-to-response rule.** A returned value reaches the response only through the
  program's own `Core\Router\Responder::respond`, so the program decides what each value sends
  (`rule:routing/a-gate-answers-around-every-handler`).
- **Middleware, filters, groups or a route-level rate limit.** The gate is the one place around every
  handler, and what a program checks there is its own, which is why
  `rule:core-classes/ratelimit-two-members` declines to decide what a limit does for a route.
- **`HEAD`-implies-`GET`, an automatic `OPTIONS`, trailing-slash normalisation.** Each is somebody's
  wrong default. The server decides them above the table, visibly
  (`rule:http-server/head-runs-as-get`, `rule:http-server/a-preflight-is-answered-before-any-code-runs`,
  `rule:http-server/a-trailing-slash-is-never-normalised`).
